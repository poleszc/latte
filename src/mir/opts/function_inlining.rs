use std::collections::HashMap;

use crate::{
    mir::{
        BlockID, LocalID, LocalInfo, MIRBlock, MIRFunction, MIRInstruction, MIROperation, MIRPlace,
        MIRProgram, MIRTerminator, MIRValue, TempID, TempInfo,
    },
    utils::StrID,
};

/// Function Inlining module:
/// Function inlining is done here and not on LIR,
/// since it would require fixing phi functions and recomputing
/// the dominance related stuff, which sounds hard. Instead we do it on MIR level.
///
/// This is a super simple function inliner. In order for the function to get
/// inlined the following has to hold:
/// - The function is not self recursive
/// - The function is short (< 10 instructions)
/// - The function is not a method
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
    }

    /// Returns true if the function should be inlined.
    fn is_inlineable(&self, func: &MIRFunction) -> bool {
        // TODO: Remove empty functions.
        let mut instr_count = 0;
        for block in func.blocks.values() {
            instr_count += block.instructions.len();
        }
        instr_count > 0 && instr_count < self.max_func_size
    }

    fn inline_calls(
        &self,
        mut func: &mut MIRFunction,
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

        // First rename all variables from the inlined function and insert them in the caller function.
        // Inlined LocalID -> Caller LocalID
        let mut local_map = HashMap::new();
        for inlined_local in &inlined.locals {
            let new_id = func.locals.len();
            let new_local = LocalInfo {
                id: LocalID(new_id),
                name: inlined_local
                    .name
                    .map(|n| StrID::from(format!("inline_{}_{}", inlined_name, n))),
                typ: inlined_local.typ.clone(),
                is_parameter: false,
                parameter_index: None,
            };
            func.locals.push(new_local);
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

        // Split the current block into two parts.
        let mut after_instructions = func
            .blocks
            .get_mut(&block_id)
            .unwrap()
            .instructions
            .split_off(instr_idx);
        after_instructions.remove(0); // Remove the call instruction
        let old_terminator = func.blocks.get_mut(&block_id).unwrap().terminator.take();

        // Create the after block.
        let after_block_id = BlockID(func.blocks.len());
        func.blocks.insert(
            after_block_id,
            MIRBlock {
                id: after_block_id,
                instructions: after_instructions,
                terminator: old_terminator,
            },
        );
        func.block_order.push(after_block_id);

        // Now renumber all blocks of the inlined function.
        let mut block_map = HashMap::new();
        for inlined_bid in &inlined.block_order {
            let new_bid = BlockID(func.blocks.iter().len() + inlined_bid.0);
            block_map.insert(*inlined_bid, new_bid);
        }

        // Now insert all blocks from the inlined function to the caller.
        for inlined_bid in &inlined.block_order {
            if let Some(inlined_block) = inlined.blocks.get(inlined_bid) {
                let mut new_instructions = Vec::new();

                // If it's an entry block we insert additional assign instructions at the beginning of the block.
                if *inlined_bid == inlined.entry_block {
                    for (i, arg_val) in instruction.operands.iter().skip(1).enumerate() {
                        let param_local = inlined
                            .locals
                            .iter()
                            .find(|l| l.parameter_index == Some(i))
                            .unwrap();
                        new_instructions.push(MIRInstruction {
                            operation: MIROperation::Assign,
                            destination: Some(MIRPlace::Local(local_map[&param_local.id])),
                            operands: vec![self.remap_value(
                                arg_val.clone(),
                                &local_map,
                                &temp_map,
                            )],
                            source_position: instruction.source_position,
                        });
                    }
                }

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
                                    destination: Some(dest.clone()), // TODO: Bug?
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
                func.block_order.push(new_bid);
            }
        }
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
