/// Dead Code elimination:
/// Eliminates unused code.
/// https://sites.cs.ucsb.edu/~yufeiding/cs293s/slides/293S_07_SSA_dead.pdf
use crate::{lir::*, opts::OptimizationPass};
use std::collections::{HashMap, HashSet};

pub struct DeadCodeElimination {
    /// Set of live registers.
    live_registers: HashSet<RegisterID>,
    /// RegisterID to instruction which defines it.
    definition_map: HashMap<RegisterID, (BlockID, usize)>,
}

impl DeadCodeElimination {
    pub fn new() -> Self {
        Self {
            live_registers: HashSet::new(),
            definition_map: HashMap::new(),
        }
    }

    /// Builds the "register" to "instruction defining it" mapping.
    fn build_definition_map(&mut self, function: &LIRFunction) {
        self.definition_map.clear();
        for block_id in &function.block_order {
            let block = function.blocks.get(block_id).unwrap();
            for (idx, instr) in block.instructions.iter().enumerate() {
                if let Some(dest) = instr.destination {
                    self.definition_map.insert(dest, (*block_id, idx));
                }
            }
        }
    }

    // Whether the operation has side effects -> we need to keep it.
    fn has_side_effects(op: &LIROperation) -> bool {
        matches!(
            op,
            LIROperation::Call
                | LIROperation::VCall
                | LIROperation::ArraySet
                | LIROperation::FieldSet
                | LIROperation::NewObject
                | LIROperation::NewArray // TODO: Any more?
        )
    }

    /// Mark the LIRValue as live.
    fn mark_value_live(&mut self, value: &LIRValue) {
        if let LIRValue::Register(reg_id) = value {
            self.live_registers.insert(*reg_id);
        }
    }

    /// Mark the LIRTerminator as live -> it's arguments.
    fn mark_terminator_live(&mut self, term: &LIRTerminator) {
        match term {
            LIRTerminator::Branch { condition, .. } => {
                self.mark_value_live(condition);
            }
            LIRTerminator::Return {
                value: Some(val), ..
            } => {
                self.mark_value_live(val);
            }
            _ => {}
        }
    }

    fn find_live_registers(&mut self, function: &LIRFunction) {
        self.live_registers.clear();
        let mut worklist: Vec<(BlockID, usize)> = Vec::new();

        // Start from marking the "for sure used statements" like prints,
        // array sets, field sets etc. and values used in terminators.
        for block_id in &function.block_order {
            let block = function.blocks.get(block_id).unwrap();
            // All values in terminators are live.
            if let Some(term) = &block.terminator {
                self.mark_terminator_live(term);
                // Add the instruction defining the terminator arguments to worklist.
                match term {
                    LIRTerminator::Branch {
                        condition: LIRValue::Register(reg_id),
                        ..
                    } => {
                        let def_loc = self.definition_map.get(reg_id).unwrap();
                        worklist.push(*def_loc);
                    }
                    LIRTerminator::Return {
                        value: Some(LIRValue::Register(reg_id)),
                        ..
                    } => {
                        let def_loc = self.definition_map.get(reg_id).unwrap();
                        worklist.push(*def_loc);
                    }
                    _ => {}
                }
            }

            // All instructions with side effects are live.
            for instr in &block.instructions {
                if Self::has_side_effects(&instr.operation) {
                    for operand in &instr.operands {
                        // Mark the operand as live and push the instruction which defines it to the worklist.
                        self.mark_value_live(operand);
                        if let LIRValue::Register(reg_id) = operand
                            && let Some(&def_loc) = self.definition_map.get(reg_id)
                        {
                            worklist.push(def_loc);
                        }
                    }
                }
            }
        }

        // Not walk backwards through the instructions and find live registers.
        while let Some((block_id, instr_idx)) = worklist.pop() {
            let block = function.blocks.get(&block_id).unwrap();
            let instr = &block.instructions[instr_idx];
            // Mark the operands as live and insert their definitions to the worklist.
            for operand in &instr.operands {
                if let LIRValue::Register(reg_id) = operand {
                    // If it wasn't marked as live before, insert it's definition to the map.
                    if self.live_registers.insert(*reg_id) {
                        let def_loc = self.definition_map[reg_id];
                        worklist.push(def_loc);
                    }
                }
            }
        }

        todo!()
    }

    /// Removes dead instructions -> instructions which destination registers aren't mapped as live.
    fn eliminate_dead_instructions(&mut self, function: &mut LIRFunction) -> bool {
        let mut changed = false;

        for block_id in &function.block_order {
            if let Some(block) = function.blocks.get_mut(block_id) {
                let old_instruction_count = block.instructions.len();

                // Leave only those instructions which:
                // - Have side effects (prints, calls, arr[] = x, a.fld = x)
                // - Have the destination marked as live (phis are removed here as well)
                block.instructions.retain(|instr| {
                    if Self::has_side_effects(&instr.operation) {
                        return true;
                    }

                    if let Some(dest) = instr.destination
                        && self.live_registers.contains(&dest)
                    {
                        return true;
                    }
                    false
                });

                if block.instructions.len() == old_instruction_count {
                    changed = true;
                }
            }
        }
        changed
    }
}

impl OptimizationPass for DeadCodeElimination {
    fn name(&self) -> String {
        String::from("Dead Code Elimination")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        self.build_definition_map(function);
        self.find_live_registers(function);
        self.eliminate_dead_instructions(function)
    }
}

impl Default for DeadCodeElimination {
    fn default() -> Self {
        Self::new()
    }
}
