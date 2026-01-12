/// Function Inlining:
/// Function inlining is done here and not on LIR,
/// since it would require fixing phi functions and recomputing
/// the dominance related stuff, which sounds hard. Instead we do it on MIR level.
///
/// This is a super simple function inliner. In order for the function to get
/// inlined the following has to hold:
/// - The function is not self recursive
/// - The function is short (< 10 instructions)
/// - The function is not a method
use crate::{
    mir::{
        BlockID, LocalID, LocalInfo, MIRBlock, MIRFunction, MIRInstruction, MIROperation, MIRPlace,
        MIRProgram, MIRTerminator, MIRValue, TempID, TempInfo,
    },
    utils::StrID,
};
use std::collections::{HashMap, HashSet};

const SHORT_FUNCTION_THRESHOLD: usize = 10;
pub struct MIRFunctionInlining {
    /// Maximum number of instructions for the function to be inlined.
    max_func_size: usize,
}
impl MIRFunctionInlining {
    pub fn new() -> Self {
        Self {
            max_func_size: SHORT_FUNCTION_THRESHOLD,
        }
    }

    pub fn run(&self, program: &mut MIRProgram) {
        let mut changed = true;
        while changed {
            changed = false;

            let candidates = program
                .functions
                .iter()
                .filter(|(_, func)| self.is_inlineable(func))
                .map(|(name, func)| (*name, func.clone()))
                .collect();

            for func_name in &mut program.function_order {
                if let Some(mut func) = program.functions.get_mut(&func_name) {
                    if self.inline_calls(&mut func, &candidates) {
                        changed = true;
                    }
                }
            }
        }
        for func in program.functions.values_mut() {
            Self::remove_unreachable_blocks(func);
        }
    }
    fn remove_unreachable_blocks(func: &mut MIRFunction) {
        let mut reachable = HashSet::new();
        let mut stack = vec![func.entry_block];
        reachable.insert(func.entry_block);

        while let Some(bid) = stack.pop() {
            if let Some(block) = func.blocks.get(&bid) {
                if let Some(term) = &block.terminator {
                    let targets = match term {
                        MIRTerminator::Jump { target, .. } => vec![*target],
                        MIRTerminator::Branch {
                            if_true, if_false, ..
                        } => vec![*if_true, *if_false],
                        MIRTerminator::Return { .. } => vec![],
                        MIRTerminator::Unreachable { .. } => vec![],
                    };

                    for target in targets {
                        if reachable.insert(target) {
                            stack.push(target);
                        }
                    }
                }
            }
        }

        func.block_order.retain(|id| reachable.contains(id));
        func.blocks.retain(|id, _| reachable.contains(id));
    }

    /// Returns true if the function should be inlined.
    fn is_inlineable(&self, func: &MIRFunction) -> bool {
        // TODO: Remove empty functions.
        let mut instr_count = 0;
        let mut is_recursive = false;

        for block in func.blocks.values() {
            instr_count += block.instructions.len();

            for instr in &block.instructions {
                if let MIROperation::Call = instr.operation {
                    if let MIRValue::Function(called_name) = &instr.operands[0] {
                        if *called_name == func.name {
                            is_recursive = true;
                        }
                    }
                }
            }
        }

        instr_count > 0 && instr_count < self.max_func_size && !is_recursive
    }

    fn inline_calls(
        &self,
        func: &mut MIRFunction,
        candidates: &HashMap<StrID, MIRFunction>,
    ) -> bool {
        let mut changed = false;

        // Iterate over the copy since we're gonna modify the block order.
        for block_id in func.block_order.clone() {
            let mut call_idx = None;

            if let Some(block) = func.blocks.get_mut(&block_id) {
                for (idx, instr) in block.instructions.iter().enumerate() {
                    if let MIROperation::Call = instr.operation {
                        if let MIRValue::Function(func_name) = &instr.operands[0] {
                            // Don't inline recursive functions.
                            if candidates.contains_key(func_name) && *func_name != func.name {
                                call_idx = Some(idx);
                            }
                        }
                    }
                }
            }
            if let Some(idx) = call_idx {
                self.perform_inline(func, block_id, idx, candidates);
                changed = true;
                continue; // TODO: Remove that?
            }
        }
        changed
    }

    fn perform_inline(
        &self,
        func: &mut MIRFunction,
        block_id: BlockID,
        instr_idx: usize,
        candidates: &HashMap<StrID, MIRFunction>,
    ) {
        let instruction = func.blocks[&block_id].instructions[instr_idx].clone();
        let inlined_name = match &instruction.operands[0] {
            MIRValue::Function(name) => name,
            _ => unreachable!(),
        };

        let inlined = &candidates[inlined_name];

        let proxy_return_local = if let Some(dest) = &instruction.destination {
            let new_id = LocalID(func.locals.len());
            let typ = match dest {
                MIRPlace::Local(id) => func.locals[id.0].typ.clone(),
                MIRPlace::Temp(id) => func.temps[id.0].typ.clone(),
                _ => panic!("Unsupported destination"),
            };
            func.locals.push(LocalInfo {
                id: new_id,
                name: Some(StrID::from(format!("inline_ret_{}", inlined_name))),
                typ,
                is_parameter: false,
                parameter_index: None,
            });
            Some(new_id)
        } else {
            None
        };

        // First rename all variables from the inlined function and insert them in the caller function.
        // Inlined LocalID -> Caller LocalID
        let mut local_map = HashMap::new();
        for inlined_local in &inlined.locals {
            let new_id = func.locals.len();
            func.locals.push(LocalInfo {
                id: LocalID(new_id),
                name: inlined_local
                    .name
                    .map(|n| StrID::from(format!("inline_{}_{}", inlined_name, n))),
                typ: inlined_local.typ.clone(),
                is_parameter: false,
                parameter_index: None,
            });
            local_map.insert(inlined_local.id, LocalID(new_id));
        }

        // Same with the temporary variables.
        let mut temp_map = HashMap::new();
        for callee_temp in &inlined.temps {
            let new_id = func.temps.len();
            func.temps.push(TempInfo {
                id: TempID(new_id),
                typ: callee_temp.typ.clone(),
            });
            temp_map.insert(callee_temp.id, TempID(new_id));
        }

        let mut next_free_block_id = func.blocks.keys().map(|b| b.0).max().unwrap_or(0) + 1;
        // Now renumber all blocks of the inlined function.
        let mut block_map = HashMap::new();
        for inlined_bid in &inlined.block_order {
            block_map.insert(*inlined_bid, BlockID(next_free_block_id));
            next_free_block_id += 1;
        }

        // Create the after block.
        let after_block_id = BlockID(next_free_block_id);

        // Split the current block into two parts.
        let mut caller_block = func.blocks.get_mut(&block_id).unwrap();

        let mut after_instructions = caller_block.instructions.split_off(instr_idx);
        after_instructions.remove(0); // Remove the call instruction
        if let (Some(dest), Some(proxy_id)) = (&instruction.destination, proxy_return_local) {
            after_instructions.insert(
                0,
                MIRInstruction {
                    operation: MIROperation::Assign,
                    destination: Some(dest.clone()),
                    operands: vec![MIRValue::Local(proxy_id)],
                    source_position: instruction.source_position,
                },
            );
        }

        // If it's an entry block we insert additional assign instructions at the beginning of the block.
        for (i, arg_val) in instruction.operands.iter().skip(1).enumerate() {
            let param_local = inlined
                .locals
                .iter()
                .find(|l| l.parameter_index == Some(i))
                .unwrap();
            caller_block.instructions.push(MIRInstruction {
                operation: MIROperation::Assign,
                destination: Some(MIRPlace::Local(local_map[&param_local.id])),
                operands: vec![arg_val.clone()],
                source_position: instruction.source_position,
            });
        }

        let old_terminator = func.blocks.get_mut(&block_id).unwrap().terminator.take();

        // Fix the terminator of the first block. Set it to the first block of the inlined function.
        let inlined_entry_id = block_map[&inlined.entry_block];
        func.blocks.get_mut(&block_id).unwrap().terminator = Some(MIRTerminator::Jump {
            target: inlined_entry_id,
            position: instruction.source_position,
        });

        func.blocks.insert(
            after_block_id,
            MIRBlock {
                id: after_block_id,
                instructions: after_instructions,
                terminator: old_terminator,
            },
        );
        // func.block_order.push(after_block_id);

        // Now insert all blocks from the inlined function to the caller.
        for inlined_bid in &inlined.block_order {
            if let Some(inlined_block) = inlined.blocks.get(inlined_bid) {
                let mut new_instructions = Vec::new();

                // Now remap all instructions in the inlined function.
                for instr in &inlined_block.instructions {
                    let mut new_instr = instr.clone();
                    new_instr.destination = new_instr
                        .destination
                        .map(|dest| self.remap_place(dest, &local_map, &temp_map));
                    new_instr.operands = new_instr
                        .operands
                        .into_iter()
                        .map(|oper| self.remap_value(oper, &local_map, &temp_map))
                        .collect();
                    new_instructions.push(new_instr);
                }

                let new_term = match &inlined_block.terminator {
                    Some(MIRTerminator::Return { value, position }) => {
                        // Return is mapped to an assign to the caller destination register and a jump to the after block.
                        if let Some(val) = value {
                            if let Some(dest) = &instruction.destination {
                                new_instructions.push(MIRInstruction {
                                    operation: MIROperation::Assign,
                                    destination: Some(MIRPlace::Local(proxy_return_local.unwrap())),
                                    operands: vec![self.remap_value(
                                        val.clone(),
                                        &local_map,
                                        &temp_map,
                                    )],
                                    source_position: *position,
                                });
                            }
                        }
                        Some(MIRTerminator::Jump {
                            target: after_block_id,
                            position: *position,
                        })
                    }
                    Some(MIRTerminator::Jump { target, position }) => Some(MIRTerminator::Jump {
                        target: block_map[target],
                        position: *position,
                    }),
                    Some(MIRTerminator::Branch {
                        condition,
                        if_true,
                        if_false,
                        position,
                    }) => Some(MIRTerminator::Branch {
                        condition: self.remap_value(condition.clone(), &local_map, &temp_map),
                        if_true: block_map[if_true],
                        if_false: block_map[if_false],
                        position: *position,
                    }),
                    other => other.clone(),
                };

                let new_bid = block_map[inlined_bid];
                func.blocks.insert(
                    new_bid,
                    MIRBlock {
                        id: new_bid,
                        instructions: new_instructions,
                        terminator: new_term,
                    },
                );
            }
        }
        let pos = func
            .block_order
            .iter()
            .position(|&id| id == block_id)
            .unwrap();
        let mut to_add = Vec::new();
        for inlined_bid in &inlined.block_order {
            to_add.push(block_map[inlined_bid]);
        }
        to_add.push(after_block_id);

        func.block_order.splice(pos + 1..pos + 1, to_add);
    }

    fn remap_value(
        &self,
        val: MIRValue,
        locals: &HashMap<LocalID, LocalID>,
        temps: &HashMap<TempID, TempID>,
    ) -> MIRValue {
        match val {
            MIRValue::Local(id) => MIRValue::Local(locals[&id]),
            MIRValue::Temp(id) => MIRValue::Temp(temps[&id]),
            other => other,
        }
    }

    fn remap_place(
        &self,
        place: MIRPlace,
        locals: &HashMap<LocalID, LocalID>,
        temps: &HashMap<TempID, TempID>,
    ) -> MIRPlace {
        match place {
            MIRPlace::Local(id) => MIRPlace::Local(locals[&id]),
            MIRPlace::Temp(id) => MIRPlace::Temp(temps[&id]),
            MIRPlace::Field {
                base,
                class_name,
                field_name,
                field_index,
            } => MIRPlace::Field {
                base: Box::new(self.remap_place(*base, locals, temps)),
                class_name,
                field_name,
                field_index,
            },
            MIRPlace::Index { base, index } => MIRPlace::Index {
                base: Box::new(self.remap_place(*base, locals, temps)),
                index: Box::new(self.remap_value(*index, locals, temps)),
            },
        }
    }
}
