/// Function Inlining:
/// Function inlining is done here and not in LIR, since it would require fixing
/// phi functions and recomputing the dominance related stuff, which sounds hard.
/// On the other hand doing it on the TAST level makes it impossible(?) to handle
/// situations like returns in the middle of a loop etc.
/// Instead we do it on MIR level.
///
/// This is a super simple function inliner. In order for the function to get
/// inlined the following has to hold:
/// - The function is not recursive
/// - The function is short (< SHORT_FUNCTION_THRESHOLD)
/// - The function is not a method
use crate::{
    mir::{
        BlockID, LocalID, LocalInfo, MIRBlock, MIRFunction, MIRInstruction, MIROperation, MIRPlace,
        MIRProgram, MIRTerminator, MIRValue, TempID, TempInfo,
    },
    parser::Block,
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
                if let Some(func) = program.functions.get_mut(func_name)
                    && self.inline_calls(func, &candidates)
                {
                    changed = true;
                }
            }
        }
        // This optimization can insert new unreachable blocks to a function. We remove them here.
        for func in program.functions.values_mut() {
            Self::remove_unreachable_blocks(func);
        }
    }
}

impl MIRFunctionInlining {
    /// Removes unreachable blocks from a MIRFunction, which may have appeared during inlining.
    fn remove_unreachable_blocks(func: &mut MIRFunction) {
        let mut reachable = HashSet::new();
        let mut stack = vec![func.entry_block];
        reachable.insert(func.entry_block);

        while let Some(bid) = stack.pop() {
            if let Some(block) = func.blocks.get(&bid)
                && let Some(term) = &block.terminator
            {
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

        func.block_order.retain(|id| reachable.contains(id));
        func.blocks.retain(|id, _| reachable.contains(id));
    }

    /// Returns true if the function should be inlined.
    fn is_inlineable(&self, func: &MIRFunction) -> bool {
        let mut instr_count = 0;
        let mut is_recursive = false;

        for block in func.blocks.values() {
            instr_count += block.instructions.len();

            for instr in &block.instructions {
                // VCalls are skipped since we don't know which function to inline.
                if let MIROperation::Call = instr.operation
                    && let MIRValue::Function(called_name) = &instr.operands[0]
                    && *called_name == func.name
                {
                    is_recursive = true;
                    break;
                }
            }
            if is_recursive {
                break;
            }
        }

        instr_count > 0 && instr_count < self.max_func_size && !is_recursive
    }

    /// Inline all needed calls in the MIRFunction.
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
                    if let MIROperation::Call = instr.operation
                        && let MIRValue::Function(func_name) = &instr.operands[0]
                        && candidates.contains_key(func_name)
                    {
                        call_idx = Some(idx);
                    }
                }
            }

            if let Some(idx) = call_idx {
                self.perform_inline(func, block_id, idx, candidates);
                changed = true;
            }
        }
        changed
    }

    /// Creates a mapping from the inlined LocalIDs to Caller LocalID, analogously for Temporaries.
    /// Inserts the locals and temps into the caller function.
    fn remap_locals_and_temps(
        &self,
        caller: &mut MIRFunction,
        callee: &MIRFunction,
    ) -> (HashMap<LocalID, LocalID>, HashMap<TempID, TempID>) {
        let mut local_map = HashMap::new();
        for inlined_local in &callee.locals {
            let new_id = caller.locals.len();
            caller.locals.push(LocalInfo {
                id: LocalID(new_id),
                name: inlined_local
                    .name
                    .map(|n| StrID::from(format!("inline_{}_{}", callee.name, n))),
                typ: inlined_local.typ.clone(),
                is_parameter: false,
                parameter_index: None,
            });
            local_map.insert(inlined_local.id, LocalID(new_id));
        }

        let mut temp_map = HashMap::new();
        for callee_temp in &callee.temps {
            let new_id = caller.temps.len();
            caller.temps.push(TempInfo {
                id: TempID(new_id),
                typ: callee_temp.typ.clone(),
            });
            temp_map.insert(callee_temp.id, TempID(new_id));
        }

        (local_map, temp_map)
    }

    /// Creates a mapping from the inlined BlockID to Caller BlockID.
    /// Blocks are reserved at the end of the function blocks.
    /// Block them self are not inserted yet.
    fn create_block_map(
        &self,
        caller: &mut MIRFunction,
        callee: &MIRFunction,
    ) -> (HashMap<BlockID, BlockID>, usize) {
        let mut block_map = HashMap::new();
        let mut next_free_block_id = caller.blocks.keys().map(|b| b.0).max().unwrap_or(0) + 1;
        for inlined_bid in &callee.block_order {
            block_map.insert(*inlined_bid, BlockID(next_free_block_id));
            next_free_block_id += 1;
        }
        (block_map, next_free_block_id)
    }

    fn create_return_local(
        &self,
        caller: &mut MIRFunction,
        instr: &MIRInstruction,
        callee_name: StrID,
    ) -> Option<LocalID> {
        let dest = instr.destination.as_ref()?;
        let typ = match dest {
            MIRPlace::Local(id) => caller.locals[id.0].typ.clone(),
            MIRPlace::Temp(id) => caller.temps[id.0].typ.clone(),
            _ => panic!("Unsupported destination"),
        };
        let new_id = LocalID(caller.locals.len());
        caller.locals.push(LocalInfo {
            id: new_id,
            name: Some(StrID::from(format!("inline_ret_{}", callee_name))),
            typ,
            is_parameter: false,
            parameter_index: None,
        });
        Some(new_id)
    }

    fn split_off_after_block(
        &self,
        func: &mut MIRFunction,
        caller_block_id: BlockID,
        instr_idx: usize,
        after_block_id: BlockID,
        call_instr: &MIRInstruction,
        return_local: Option<LocalID>,
    ) {
        // Split the current block into two parts.
        let caller_block = func.blocks.get_mut(&caller_block_id).unwrap();
        let mut after_instructions = caller_block.instructions.split_off(instr_idx);
        after_instructions.remove(0); // Remove the call instruction

        // If a function returned a value insert the assign instruction after the inlined call
        // from the return local to the real call destination.
        if let (Some(dest), Some(proxy_id)) = (&call_instr.destination, return_local) {
            after_instructions.insert(
                0,
                MIRInstruction {
                    operation: MIROperation::Assign,
                    destination: Some(dest.clone()),
                    operands: vec![MIRValue::Local(proxy_id)],
                    source_position: call_instr.source_position,
                },
            );
        }

        // Remove the terminator from the split block. It will be filled later.
        // Move it to the after block.
        let old_terminator = func
            .blocks
            .get_mut(&caller_block_id)
            .unwrap()
            .terminator
            .take();

        func.blocks.insert(
            after_block_id,
            MIRBlock {
                id: after_block_id,
                instructions: after_instructions,
                terminator: old_terminator,
            },
        );
    }

    fn inject_argument_assignments(
        &self,
        caller: &mut MIRFunction,
        caller_block_id: BlockID,
        call_instr: &MIRInstruction,
        callee: &MIRFunction,
        local_map: &HashMap<LocalID, LocalID>,
    ) {
        let block = caller.blocks.get_mut(&caller_block_id).unwrap();
        // Skip the first operand since it's the function name.
        for (i, arg_val) in call_instr.operands.iter().skip(1).enumerate() {
            let param_local = callee
                .locals
                .iter()
                .find(|l| l.parameter_index == Some(i))
                .unwrap();
            block.instructions.push(MIRInstruction {
                operation: MIROperation::Assign,
                destination: Some(MIRPlace::Local(local_map[&param_local.id])),
                operands: vec![arg_val.clone()],
                source_position: call_instr.source_position,
            });
        }
    }
    fn inject_callee_blocks(
        &self,
        func: &mut MIRFunction,
        callee: &MIRFunction,
        block_map: &HashMap<BlockID, BlockID>,
        local_map: &HashMap<LocalID, LocalID>,
        temp_map: &HashMap<TempID, TempID>,
        after_block_id: BlockID,
        return_local: Option<LocalID>,
    ) {
        for callee_bid in &callee.block_order {
            let callee_block = &callee.blocks[callee_bid];
            // Remap all instructions of the callee to use the locals and temps of the caller.
            let mut new_instructions: Vec<MIRInstruction> = callee_block
                .instructions
                .iter()
                .map(|instr| {
                    let mut new_instr = instr.clone();
                    new_instr.destination = new_instr
                        .destination
                        .map(|d| self.remap_place(d, local_map, temp_map));
                    new_instr.operands = new_instr
                        .operands
                        .into_iter()
                        .map(|o| self.remap_value(o, local_map, temp_map))
                        .collect();
                    new_instr
                })
                .collect();

            // Remap the terminators of each block.
            // Change callee's BlockIDs to caller BlockIDs.
            // Change returns to (assign to the return local, jumps to the after block)
            let new_term = match &callee_block.terminator {
                Some(MIRTerminator::Return { value, position }) => {
                    if let Some(val) = value {
                        new_instructions.push(MIRInstruction {
                            operation: MIROperation::Assign,
                            destination: Some(MIRPlace::Local(return_local.unwrap())),
                            operands: vec![self.remap_value(val.clone(), local_map, temp_map)],
                            source_position: *position,
                        });
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
                    condition: self.remap_value(condition.clone(), local_map, temp_map),
                    if_true: block_map[if_true],
                    if_false: block_map[if_false],
                    position: *position,
                }),
                other => other.clone(),
            };

            // Insert the new block to the caller block stack.
            let new_bid = block_map[callee_bid];
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

    /// Update the block order of the caller.
    fn update_block_order(
        &self,
        func: &mut MIRFunction,
        caller_bid: BlockID,
        callee: &MIRFunction,
        block_map: &HashMap<BlockID, BlockID>,
        after_bid: BlockID,
    ) {
        let pos = func
            .block_order
            .iter()
            .position(|&id| id == caller_bid)
            .unwrap();
        let mut to_add: Vec<BlockID> = callee
            .block_order
            .iter()
            .map(|bid| block_map[bid])
            .collect();
        to_add.push(after_bid);
        func.block_order.splice(pos + 1..pos + 1, to_add);
    }

    /// Actually performs the inline.
    fn perform_inline(
        &self,
        func: &mut MIRFunction,
        block_id: BlockID,
        instr_idx: usize,
        candidates: &HashMap<StrID, MIRFunction>,
    ) {
        let call_instr = func.blocks[&block_id].instructions[instr_idx].clone();
        let callee_name = match &call_instr.operands[0] {
            MIRValue::Function(name) => name,
            _ => unreachable!(),
        };
        let callee = &candidates[callee_name];

        // First create a mapping from caller Locals/Temps to callee Locals/Temps as they
        // have to be renamed and insert new locals to the callers function.
        let (local_map, temp_map) = self.remap_locals_and_temps(func, callee);
        // Now renumber all blocks of the inlined function, and create a mapping from callee BlockID to caller BlockID.
        let (block_map, next_id) = self.create_block_map(func, callee);

        // TODO: Comment
        let return_local = self.create_return_local(func, &call_instr, *callee_name);

        // Create the after block and move the instructions after the call to it.
        let after_block_id = BlockID(next_id);
        self.split_off_after_block(
            func,
            block_id,
            instr_idx,
            after_block_id,
            &call_instr,
            return_local,
        );

        // Insert Assign instructions from the caller arguments to callee's locals, before the jump to the callee block.
        self.inject_argument_assignments(func, block_id, &call_instr, callee, &local_map);

        // Insert callee's block to the caller. Remap all BlockIDs, LocalIDs and TempIDs.
        self.inject_callee_blocks(
            func,
            callee,
            &block_map,
            &local_map,
            &temp_map,
            after_block_id,
            return_local,
        );

        // Fix the terminator of the first block.
        // Make if a jump to the first block of the inlined function.
        let callee_entry_block_id = block_map[&callee.entry_block];
        func.blocks.get_mut(&block_id).unwrap().terminator = Some(MIRTerminator::Jump {
            target: callee_entry_block_id,
            position: call_instr.source_position,
        });

        self.update_block_order(func, block_id, callee, &block_map, after_block_id);
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
