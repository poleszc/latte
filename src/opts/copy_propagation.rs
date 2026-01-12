use std::collections::HashMap;

/// Global Common Sub-expression Elimination:
use crate::{lir::*, opts::OptimizationPass};

pub struct CopyPropagation {
    /// Register to its original value.
    copies: HashMap<RegisterID, LIRValue>,
}

impl CopyPropagation {
    pub fn new() -> Self {
        Self {
            copies: HashMap::new(),
        }
    }

    /// Finds the original source for the register. Handles code like:
    /// %2 = %1
    /// %3 = %2
    /// %5 = %4
    /// %6 = add %5, 1
    /// And replaces %5 with %1 straight away.
    fn find_source(&self, value: &LIRValue) -> LIRValue {
        if let LIRValue::Register(reg) = value {
            if let Some(original) = self.copies.get(reg) {
                return self.find_source(original);
            }
        }
        value.clone()
    }

    fn replace_value(&self, value: &mut LIRValue) -> bool {
        if let LIRValue::Register(reg) = value {
            if let Some(original) = self.copies.get(reg) {
                *value = self.find_source(original);
                return true;
            }
        }
        false
    }
    fn replace_terminator(&self, term: &mut LIRTerminator) -> bool {
        match term {
            LIRTerminator::Branch { condition, .. } => self.replace_value(condition),
            LIRTerminator::Return {
                value: Some(val), ..
            } => self.replace_value(val),
            _ => false,
        }
    }
}

impl OptimizationPass for CopyPropagation {
    fn name(&self) -> String {
        String::from("Copy Propagation")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        self.copies.clear();
        let mut changed = false;

        for block_id in &function.block_order {
            if let Some(block) = function.blocks.get(block_id) {
                for instr in &block.instructions {
                    if matches!(instr.operation, LIROperation::Assign) {
                        if let Some(dest) = instr.destination {
                            let src = &instr.operands[0];
                            self.copies.insert(dest, src.clone());
                        }
                    }
                }
            }
        }

        if self.copies.is_empty() {
            return false;
        }

        for block_id in &function.block_order {
            if let Some(block) = function.blocks.get_mut(block_id) {
                for instr in &mut block.instructions {
                    for operand in &mut instr.operands {
                        if self.replace_value(operand) {
                            changed = true;
                        }
                    }
                }
                if let Some(term) = &mut block.terminator {
                    if self.replace_terminator(term) {
                        changed = true;
                    }
                }
            }
        }

        // Remove unused registers from the list - mark them with usize::MAX.
        for reg_info in &mut function.registers {
            if self.copies.contains_key(&reg_info.id) {
                reg_info.id = RegisterID(usize::MAX);
            }
        }

        changed
    }
}

impl Default for CopyPropagation {
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

    fn create_test_function() -> LIRFunction {
        LIRFunction {
            name: StrID::from("test"),
            return_type: Type {
                kind: TypeKind::Int,
                position: SourcePosition(0, 0),
            },
            parameter_types: vec![],
            parameter_registers: vec![],
            registers: vec![
                RegisterInfo {
                    id: RegisterID(0),
                    typ: Type {
                        kind: TypeKind::Int,
                        position: SourcePosition(0, 0),
                    },
                    name: None,
                },
                RegisterInfo {
                    id: RegisterID(1),
                    typ: Type {
                        kind: TypeKind::Int,
                        position: SourcePosition(0, 0),
                    },
                    name: None,
                },
                RegisterInfo {
                    id: RegisterID(2),
                    typ: Type {
                        kind: TypeKind::Int,
                        position: SourcePosition(0, 0),
                    },
                    name: None,
                },
                RegisterInfo {
                    id: RegisterID(3),
                    typ: Type {
                        kind: TypeKind::Int,
                        position: SourcePosition(0, 0),
                    },
                    name: None,
                },
            ],
            blocks: HashMap::new(),
            block_order: vec![],
            entry_block: BlockID(0),
            source_position: SourcePosition(0, 0),
        }
    }

    #[test]
    fn test_simple_copy_propagation() {
        // %r0 = 10
        // %r1 = Assign %r0
        // ret %r1
        let mut function = create_test_function();
        let mut block = LIRBlock::with_id(BlockID(0));

        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(RegisterID(0)),
            operands: vec![LIRValue::IntConst(10)],
            source_position: SourcePosition(0, 0),
        });

        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(RegisterID(1)),
            operands: vec![LIRValue::Register(RegisterID(0))],
            source_position: SourcePosition(0, 0),
        });

        block.terminator = Some(LIRTerminator::Return {
            value: Some(LIRValue::Register(RegisterID(1))),
            position: SourcePosition(0, 0),
        });

        function.blocks.insert(BlockID(0), block);
        function.block_order.push(BlockID(0));

        let mut pass = CopyPropagation::new();
        let changed = pass.run_on_function(&mut function);

        assert!(changed);
        let final_block = function.blocks.get(&BlockID(0)).unwrap();

        dbg!(&final_block.terminator);
        match &final_block.terminator {
            Some(LIRTerminator::Return {
                value: Some(LIRValue::IntConst(10)),
                ..
            }) => {}
            _ => panic!("Expected Return with Register(0)"),
        }
    }

    #[test]
    fn test_transitive_copy_propagation() {
        // %r0 = call ...
        // %r1 = Assign %r0
        // %r2 = Assign %r1
        // %r3 = Add %r2, %r2
        let mut function = create_test_function();
        let mut block = LIRBlock::with_id(BlockID(0));

        block.instructions.push(LIRInstruction {
            operation: LIROperation::Call,
            destination: Some(RegisterID(0)),
            operands: vec![LIRValue::Function(StrID::from("getExternal"))],
            source_position: SourcePosition(0, 0),
        });

        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(RegisterID(1)),
            operands: vec![LIRValue::Register(RegisterID(0))],
            source_position: SourcePosition(0, 0),
        });

        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(RegisterID(2)),
            operands: vec![LIRValue::Register(RegisterID(1))],
            source_position: SourcePosition(0, 0),
        });

        block.instructions.push(LIRInstruction {
            operation: LIROperation::IntAdd,
            destination: Some(RegisterID(3)),
            operands: vec![
                LIRValue::Register(RegisterID(2)),
                LIRValue::Register(RegisterID(2)),
            ],
            source_position: SourcePosition(0, 0),
        });

        block.terminator = Some(LIRTerminator::Return {
            value: None,
            position: SourcePosition(0, 0),
        });

        function.blocks.insert(BlockID(0), block);
        function.block_order.push(BlockID(0));

        let mut pass = CopyPropagation::new();
        let changed = pass.run_on_function(&mut function);

        assert!(changed);
        let final_block = function.blocks.get(&BlockID(0)).unwrap();
        let add_instr = &final_block.instructions[3];

        assert_eq!(add_instr.operands[0], LIRValue::Register(RegisterID(0)));
        assert_eq!(add_instr.operands[1], LIRValue::Register(RegisterID(0)));
    }

    #[test]
    fn test_propagate_constant_through_assign() {
        let mut function = create_test_function();
        let mut block = LIRBlock::with_id(BlockID(0));

        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(RegisterID(0)),
            operands: vec![LIRValue::IntConst(42)],
            source_position: SourcePosition(0, 0),
        });

        block.instructions.push(LIRInstruction {
            operation: LIROperation::IntAdd,
            destination: Some(RegisterID(1)),
            operands: vec![LIRValue::Register(RegisterID(0)), LIRValue::IntConst(1)],
            source_position: SourcePosition(0, 0),
        });

        block.terminator = Some(LIRTerminator::Return {
            value: None,
            position: SourcePosition(0, 0),
        });
        function.blocks.insert(BlockID(0), block);
        function.block_order.push(BlockID(0));

        let mut pass = CopyPropagation::new();
        pass.run_on_function(&mut function);

        let final_block = function.blocks.get(&BlockID(0)).unwrap();
        let add_instr = &final_block.instructions[1];

        assert_eq!(add_instr.operands[0], LIRValue::IntConst(42));
    }
}
