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
    utils::StrID,
};
use std::collections::{HashMap, HashSet};

const SHORT_FUNCTION_THRESHOLD: usize = 10;
pub struct MIRFunctionInlining {
    /// Maximum number of instructions for the function to be inlined.
    max_func_size: usize,
}

struct InliningContext<'a> {
    callee: &'a MIRFunction,
    local_map: HashMap<LocalID, LocalID>, // Callee LocalID -> Caller LocalID
    temp_map: HashMap<TempID, TempID>,    // Callee TempID -> Caller TempID
    block_map: HashMap<BlockID, BlockID>, // Callee BlockID -> Caller BlockID
    return_local: Option<LocalID>,
    after_block_id: BlockID,
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
            let mut callee_name = None;

            if let Some(block) = func.blocks.get_mut(&block_id) {
                for (idx, instr) in block.instructions.iter().enumerate() {
                    if let MIROperation::Call = instr.operation
                        && let MIRValue::Function(func_name) = &instr.operands[0]
                        && candidates.contains_key(func_name)
                    {
                        call_idx = Some(idx);
                        callee_name = Some(*func_name);
                    }
                }
            }

            if let Some(idx) = call_idx {
                self.perform_inline(func, block_id, idx, &candidates[&callee_name.unwrap()]);
                changed = true;
            }
        }
        changed
    }

    /// Actually performs the inline.
    fn perform_inline(
        &self,
        caller: &mut MIRFunction,
        block_id: BlockID,
        instr_idx: usize,
        callee: &MIRFunction,
    ) {
        let call_instr = caller.blocks[&block_id].instructions[instr_idx].clone();
        let ctx = InliningContext::new(caller, callee, &call_instr);

        // Create the after block and move the instructions after the call to it.
        ctx.split_off_after_block(caller, block_id, instr_idx, &call_instr);

        // Insert Assign instructions from the caller arguments to callee's locals, before the jump to the callee block.
        ctx.inject_argument_assignments(caller, block_id, &call_instr);

        // Insert callee's block to the caller. Remap all BlockIDs, LocalIDs and TempIDs.
        ctx.inject_callee_blocks(caller);

        // Fix the terminator of the first block.
        // Make if a jump to the first block of the inlined function.
        let callee_entry_block_id = ctx.block_map[&callee.entry_block];
        caller.blocks.get_mut(&block_id).unwrap().terminator = Some(MIRTerminator::Jump {
            target: callee_entry_block_id,
            position: call_instr.source_position,
        });

        self.update_block_order(caller, block_id, &ctx);
    }

    /// Updates the block order of the caller after callee block have been added.
    fn update_block_order(
        &self,
        func: &mut MIRFunction,
        caller_bid: BlockID,
        ctx: &InliningContext,
    ) {
        let pos = func
            .block_order
            .iter()
            .position(|&id| id == caller_bid)
            .unwrap();
        let mut to_add: Vec<BlockID> = ctx
            .callee
            .block_order
            .iter()
            .map(|bid| ctx.block_map[bid])
            .collect();
        to_add.push(ctx.after_block_id);
        func.block_order.splice(pos + 1..pos + 1, to_add);
    }
}

impl<'a> InliningContext<'a> {
    fn new(caller: &mut MIRFunction, callee: &'a MIRFunction, call_instr: &MIRInstruction) -> Self {
        let mut local_map = HashMap::new();
        let mut temp_map = HashMap::new();
        let mut block_map = HashMap::new();

        // First create a mapping from caller Locals/Temps to callee Locals/Temps as they
        // have to be renamed and insert new locals to the callers function.
        for inlined_local in &callee.locals {
            let new_id = LocalID(caller.locals.len());
            caller.locals.push(LocalInfo {
                id: new_id,
                name: inlined_local
                    .name
                    .map(|n| StrID::from(format!("inline_{}_{}", callee.name, n))),
                typ: inlined_local.typ.clone(),
                is_parameter: false,
                parameter_index: None,
            });
            local_map.insert(inlined_local.id, new_id);
        }

        for callee_temp in &callee.temps {
            let new_id = TempID(caller.temps.len());
            caller.temps.push(TempInfo {
                id: new_id,
                typ: callee_temp.typ.clone(),
            });
            temp_map.insert(callee_temp.id, new_id);
        }

        // Create a mapping from the inlined BlockID to Caller BlockID.
        // Blocks are reserved at the end of the function blocks.
        // Block them self are not inserted yet.
        let mut next_id = caller.blocks.keys().map(|b| b.0).max().unwrap_or(0) + 1;
        for inlined_bid in &callee.block_order {
            block_map.insert(*inlined_bid, BlockID(next_id));
            next_id += 1;
        }

        let after_block_id = BlockID(next_id);

        // Since LIR assumes Temporaries can't leak through block, we need to create a local
        // in which the return value will be stored, so it can leak through blocks.
        let return_local = call_instr.destination.as_ref().map(|dest| {
            let typ = match dest {
                MIRPlace::Local(id) => caller.locals[id.0].typ.clone(),
                MIRPlace::Temp(id) => caller.temps[id.0].typ.clone(),
                _ => panic!("Unsupported destination"),
            };
            let new_id = LocalID(caller.locals.len());
            caller.locals.push(LocalInfo {
                id: new_id,
                name: Some(StrID::from(format!("inline_ret_{}", callee.name))),
                typ,
                is_parameter: false,
                parameter_index: None,
            });
            new_id
        });

        Self {
            callee,
            local_map,
            temp_map,
            block_map,
            return_local,
            after_block_id,
        }
    }

    /// Splits the block with the inlined call into two blocks (before call, after all)
    /// and removes the call instruction itself.
    fn split_off_after_block(
        &self,
        caller: &mut MIRFunction,
        block_id: BlockID,
        idx: usize,
        call_instr: &MIRInstruction,
    ) {
        let caller_block = caller.blocks.get_mut(&block_id).unwrap();
        let mut after_instructions = caller_block.instructions.split_off(idx);
        after_instructions.remove(0); // Remove the call instruction.

        // If a function returned a value insert the assign instruction after the inlined call
        // from the return local to the real call destination.
        if let (Some(dest), Some(ret_id)) = (&call_instr.destination, self.return_local) {
            after_instructions.insert(
                0,
                MIRInstruction {
                    operation: MIROperation::Assign,
                    destination: Some(dest.clone()),
                    operands: vec![MIRValue::Local(ret_id)],
                    source_position: call_instr.source_position,
                },
            );
        }

        // Remove the terminator from the split block. It will be filled later.
        // Move it to the after block.
        let old_term = caller_block.terminator.take();
        caller.blocks.insert(
            self.after_block_id,
            MIRBlock {
                id: self.after_block_id,
                instructions: after_instructions,
                terminator: old_term,
            },
        );
    }

    fn inject_argument_assignments(
        &self,
        caller: &mut MIRFunction,
        block_id: BlockID,
        call_instr: &MIRInstruction,
    ) {
        let block = caller.blocks.get_mut(&block_id).unwrap();
        // Skip the first operand since it's the function name.
        for (i, arg_val) in call_instr.operands.iter().skip(1).enumerate() {
            let param = self
                .callee
                .locals
                .iter()
                .find(|l| l.parameter_index == Some(i))
                .unwrap();
            block.instructions.push(MIRInstruction {
                operation: MIROperation::Assign,
                destination: Some(MIRPlace::Local(self.local_map[&param.id])),
                operands: vec![arg_val.clone()],
                source_position: call_instr.source_position,
            });
        }
    }

    fn inject_callee_blocks(&self, caller: &mut MIRFunction) {
        for callee_bid in &self.callee.block_order {
            let callee_block = &self.callee.blocks[callee_bid];

            // Remap all instructions of the callee to use the locals and temps of the caller.
            let mut new_instructions: Vec<MIRInstruction> = callee_block
                .instructions
                .iter()
                .map(|instr| {
                    let mut new_instr = instr.clone();
                    new_instr.destination = new_instr.destination.map(|d| self.remap_place(d));
                    new_instr.operands = new_instr
                        .operands
                        .into_iter()
                        .map(|o| self.remap_value(o))
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
                            destination: Some(MIRPlace::Local(
                                self.return_local.expect("Return value without proxy"),
                            )),
                            operands: vec![self.remap_value(val.clone())],
                            source_position: *position,
                        });
                    }
                    Some(MIRTerminator::Jump {
                        target: self.after_block_id,
                        position: *position,
                    })
                }
                Some(MIRTerminator::Jump { target, position }) => Some(MIRTerminator::Jump {
                    target: self.block_map[target],
                    position: *position,
                }),
                Some(MIRTerminator::Branch {
                    condition,
                    if_true,
                    if_false,
                    position,
                }) => Some(MIRTerminator::Branch {
                    condition: self.remap_value(condition.clone()),
                    if_true: self.block_map[if_true],
                    if_false: self.block_map[if_false],
                    position: *position,
                }),
                other => other.clone(),
            };

            // Insert the new block to the caller block stack.
            caller.blocks.insert(
                self.block_map[callee_bid],
                MIRBlock {
                    id: self.block_map[callee_bid],
                    instructions: new_instructions,
                    terminator: new_term,
                },
            );
        }
    }

    fn remap_value(&self, val: MIRValue) -> MIRValue {
        match val {
            MIRValue::Local(id) => MIRValue::Local(self.local_map[&id]),
            MIRValue::Temp(id) => MIRValue::Temp(self.temp_map[&id]),
            other => other,
        }
    }

    fn remap_place(&self, place: MIRPlace) -> MIRPlace {
        match place {
            MIRPlace::Local(id) => MIRPlace::Local(self.local_map[&id]),
            MIRPlace::Temp(id) => MIRPlace::Temp(self.temp_map[&id]),
            MIRPlace::Field {
                base,
                class_name,
                field_name,
                field_index,
            } => MIRPlace::Field {
                base: Box::new(self.remap_place(*base)),
                class_name,
                field_name,
                field_index,
            },
            MIRPlace::Index { base, index } => MIRPlace::Index {
                base: Box::new(self.remap_place(*base)),
                index: Box::new(self.remap_value(*index)),
            },
        }
    }
}
