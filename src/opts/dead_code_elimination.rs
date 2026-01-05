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

                if block.instructions.len() != old_instruction_count {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourcePosition;
    use crate::parser::ast::{Type, TypeKind};
    use crate::utils::StrID;
    use std::collections::HashMap;

    fn create_test_function(entry_block: BlockID) -> LIRFunction {
        LIRFunction {
            name: StrID::from("test"),
            return_type: Type {
                kind: TypeKind::Int,
                position: SourcePosition(0, 0),
            },
            parameter_types: vec![],
            parameter_registers: vec![],
            registers: vec![],
            blocks: HashMap::new(),
            block_order: vec![],
            entry_block,
            source_position: SourcePosition(0, 0),
        }
    }

    // Helper to create a simple block with a Return terminator.
    fn returning_block(id: BlockID, value: Option<LIRValue>) -> LIRBlock {
        LIRBlock {
            id,
            instructions: vec![],
            terminator: Some(LIRTerminator::Return {
                value,
                position: SourcePosition(0, 0),
            }),
            predecessors: vec![],
        }
    }

    #[test]
    fn test_simple_dead_instruction_is_removed() {
        // r0 = 10;
        // r1 = 20;
        // return r1;
        let mut func = create_test_function(BlockID(0));
        let r0 = RegisterID(0);
        let r1 = RegisterID(1);

        let mut block = returning_block(BlockID(0), Some(LIRValue::Register(r1)));
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r0),
            operands: vec![LIRValue::IntConst(10)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r1),
            operands: vec![LIRValue::IntConst(20)],
            source_position: SourcePosition(0, 0),
        });

        func.blocks.insert(BlockID(0), block);
        func.block_order.push(BlockID(0));

        let mut dce = DeadCodeElimination::new();
        let changed = dce.run_on_function(&mut func);

        assert!(changed);
        let final_block = func.blocks.get(&BlockID(0)).unwrap();
        assert_eq!(final_block.instructions.len(), 1);
        assert_eq!(
            final_block.instructions[0].destination,
            Some(r1),
            "Only the definition of r1 should remain"
        );
    }

    #[test]
    fn test_transitive_dead_code_is_removed() {
        // r0 = 10;
        // r1 = r0 + 5; (dead because r1 is not used)
        // r2 = 100;
        // return r2;
        let mut func = create_test_function(BlockID(0));
        let r0 = RegisterID(0);
        let r1 = RegisterID(1);
        let r2 = RegisterID(2);

        let mut block = returning_block(BlockID(0), Some(LIRValue::Register(r2)));
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r0),
            operands: vec![LIRValue::IntConst(10)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::IntAdd,
            destination: Some(r1),
            operands: vec![LIRValue::Register(r0), LIRValue::IntConst(5)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r2),
            operands: vec![LIRValue::IntConst(100)],
            source_position: SourcePosition(0, 0),
        });

        func.blocks.insert(BlockID(0), block);
        func.block_order.push(BlockID(0));

        let mut dce = DeadCodeElimination::new();
        let changed = dce.run_on_function(&mut func);

        assert!(changed);
        let final_block = func.blocks.get(&BlockID(0)).unwrap();
        assert_eq!(final_block.instructions.len(), 1);
        assert_eq!(final_block.instructions[0].destination, Some(r2));
    }

    #[test]
    fn test_instruction_with_side_effects_is_kept() {
        // r0 = 10;
        // call print(r0);
        // r1 = 20;
        // return 0;
        let mut func = create_test_function(BlockID(0));
        let r0 = RegisterID(0);
        let r1 = RegisterID(1);

        let mut block = returning_block(BlockID(0), Some(LIRValue::IntConst(0)));
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r0),
            operands: vec![LIRValue::IntConst(10)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Call,
            destination: None,
            operands: vec![LIRValue::Register(r0)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r1),
            operands: vec![LIRValue::IntConst(20)],
            source_position: SourcePosition(0, 0),
        });

        func.blocks.insert(BlockID(0), block);
        func.block_order.push(BlockID(0));

        let mut dce = DeadCodeElimination::new();
        let changed = dce.run_on_function(&mut func);

        assert!(changed);
        let final_block = func.blocks.get(&BlockID(0)).unwrap();
        assert_eq!(final_block.instructions.len(), 2);
        assert_eq!(final_block.instructions[0].destination, Some(r0));
        assert_eq!(final_block.instructions[1].operation, LIROperation::Call);
    }

    #[test]
    fn test_liveness_from_branch_condition() {
        // r0 = 10;
        // r1 = 20;
        // r_cond = r0 < r1;
        // r_dead = 99;
        // branch r_cond, B1, B2
        let mut func = create_test_function(BlockID(0));
        let r0 = RegisterID(0);
        let r1 = RegisterID(1);
        let r_cond = RegisterID(2);
        let r_dead = RegisterID(3);

        let mut block = LIRBlock::with_id(BlockID(0));
        block.terminator = Some(LIRTerminator::Branch {
            condition: LIRValue::Register(r_cond),
            if_true: BlockID(1),
            if_false: BlockID(2),
            position: SourcePosition(0, 0),
        });

        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r0),
            operands: vec![LIRValue::IntConst(10)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r1),
            operands: vec![LIRValue::IntConst(20)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::IntLt,
            destination: Some(r_cond),
            operands: vec![LIRValue::Register(r0), LIRValue::Register(r1)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r_dead),
            operands: vec![LIRValue::IntConst(99)],
            source_position: SourcePosition(0, 0),
        });

        func.blocks.insert(BlockID(0), block);
        func.block_order.push(BlockID(0));

        let mut dce = DeadCodeElimination::new();
        let changed = dce.run_on_function(&mut func);

        assert!(changed);
        let final_block = func.blocks.get(&BlockID(0)).unwrap();
        assert_eq!(final_block.instructions.len(), 3);
        for instr in &final_block.instructions {
            assert_ne!(instr.destination, Some(r_dead));
        }
    }

    #[test]
    fn test_liveness_propagates_through_phi_node() {
        // B0: r0 = 10; jump B2
        // B1: r1 = 20; jump B2
        // B2: r_phi = phi [r0, B0], [r1, B1]; return r_phi
        // Every instruction should be live.
        let mut func = create_test_function(BlockID(0));
        let b0 = BlockID(0);
        let b1 = BlockID(1);
        let b2 = BlockID(2);
        let r0 = RegisterID(0);
        let r1 = RegisterID(1);
        let r_phi = RegisterID(2);

        // Block 0
        let mut block0 = LIRBlock::with_id(b0);
        block0.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r0),
            operands: vec![LIRValue::IntConst(10)],
            source_position: SourcePosition(0, 0),
        });
        block0.terminator = Some(LIRTerminator::Jump {
            target: b2,
            position: SourcePosition(0, 0),
        });

        // Block 1
        let mut block1 = LIRBlock::with_id(b1);
        block1.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r1),
            operands: vec![LIRValue::IntConst(20)],
            source_position: SourcePosition(0, 0),
        });
        block1.terminator = Some(LIRTerminator::Jump {
            target: b2,
            position: SourcePosition(0, 0),
        });

        // Block 2
        let mut block2 = returning_block(b2, Some(LIRValue::Register(r_phi)));
        block2.instructions.push(LIRInstruction {
            operation: LIROperation::Phi,
            destination: Some(r_phi),
            operands: vec![
                LIRValue::Register(r0),
                LIRValue::Block(b0),
                LIRValue::Register(r1),
                LIRValue::Block(b1),
            ],
            source_position: SourcePosition(0, 0),
        });

        func.blocks.insert(b0, block0);
        func.blocks.insert(b1, block1);
        func.blocks.insert(b2, block2);
        func.block_order = vec![b0, b1, b2];

        let mut dce = DeadCodeElimination::new();
        let changed = dce.run_on_function(&mut func);

        assert!(!changed, "No code should be eliminated as all is live");
        assert_eq!(func.blocks.get(&b0).unwrap().instructions.len(), 1);
        assert_eq!(func.blocks.get(&b1).unwrap().instructions.len(), 1);
        assert_eq!(func.blocks.get(&b2).unwrap().instructions.len(), 1);
    }
}
