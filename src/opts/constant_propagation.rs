/// Constant folding and constant propagation
/// Calculates the constant values known in compile time and replaces
/// the register usages with the calculated constant values.
use crate::{lir::*, opts::OptimizationPass};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
enum ConstantValue {
    Int(i32),
    Bool(bool),
    String(String),
    Null,
    Unknown,
}

pub struct ConstantFoldingAndPropagation {
    constants: HashMap<RegisterID, ConstantValue>,
}

impl ConstantFoldingAndPropagation {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
        }
    }
}

// ======================
// Constant Folding.
// ======================
impl ConstantFoldingAndPropagation {
    // Performs constant folding and updates the constant map.
    fn analyze_instruction(&mut self, instr: &LIRInstruction) {
        if let Some(dest) = instr.destination {
            let value = match instr.operation {
                LIROperation::Assign => {
                    assert!(instr.operands.len() == 1);
                    self.evaluate_value(&instr.operands[0])
                }
                LIROperation::IntAdd => self.fold_binary_int(&instr.operands, |a, b| a + b),
                LIROperation::IntSub => self.fold_binary_int(&instr.operands, |a, b| a - b),
                LIROperation::IntMul => self.fold_binary_int(&instr.operands, |a, b| a * b),
                LIROperation::IntDiv => self.fold_binary_int(&instr.operands, |a, b| {
                    assert!(b != 0, "Zero division in constant folding");
                    a / b
                }),
                LIROperation::IntMod => self.fold_binary_int(&instr.operands, |a, b| {
                    assert!(b != 0, "Zero mod in constant folding");
                    a % b
                }),
                LIROperation::IntNeg => self.fold_unary_int(&instr.operands, |n| -n),
                LIROperation::IntLt => self.fold_comparison(&instr.operands, |a, b| a < b),
                LIROperation::IntLe => self.fold_comparison(&instr.operands, |a, b| a <= b),
                LIROperation::IntGt => self.fold_comparison(&instr.operands, |a, b| a > b),
                LIROperation::IntGe => self.fold_comparison(&instr.operands, |a, b| a >= b),
                LIROperation::IntEq => self.fold_comparison(&instr.operands, |a, b| a == b),
                LIROperation::IntNeq => self.fold_comparison(&instr.operands, |a, b| a != b),
                LIROperation::BoolAnd => self.fold_bool_and(&instr.operands),
                LIROperation::BoolOr => self.fold_bool_or(&instr.operands),
                LIROperation::BoolNot => self.fold_bool_not(&instr.operands),
                LIROperation::StringEq => self.fold_string_eq(&instr.operands),
                LIROperation::StringNeq => self.fold_string_neq(&instr.operands),
                LIROperation::StringConcat => self.fold_string_concat(&instr.operands),
                LIROperation::StringLength => self.fold_string_length(&instr.operands),
                _ => ConstantValue::Unknown,
            };
            self.constants.insert(dest, value);
        }
    }

    fn evaluate_value(&self, value: &LIRValue) -> ConstantValue {
        match value {
            LIRValue::IntConst(n) => ConstantValue::Int(*n),
            LIRValue::BoolConst(b) => ConstantValue::Bool(*b),
            LIRValue::StringConst(s) => ConstantValue::String(s.clone()),
            LIRValue::Null => ConstantValue::Null,
            LIRValue::Register(reg) => self
                .constants
                .get(reg)
                .cloned()
                .unwrap_or(ConstantValue::Unknown),
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_binary_int<F>(&self, operands: &[LIRValue], op: F) -> ConstantValue
    where
        F: Fn(i32, i32) -> i32,
    {
        assert!(operands.len() == 2);
        match (
            self.evaluate_value(&operands[0]),
            self.evaluate_value(&operands[1]),
        ) {
            (ConstantValue::Int(l), ConstantValue::Int(r)) => ConstantValue::Int(op(l, r)),
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_unary_int<F>(&self, operands: &[LIRValue], op: F) -> ConstantValue
    where
        F: Fn(i32) -> i32,
    {
        assert!(operands.len() == 1);
        match self.evaluate_value(&operands[0]) {
            ConstantValue::Int(n) => ConstantValue::Int(op(n)),
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_comparison<F>(&self, operands: &[LIRValue], op: F) -> ConstantValue
    where
        F: Fn(i32, i32) -> bool,
    {
        assert!(operands.len() == 2);
        match (
            self.evaluate_value(&operands[0]),
            self.evaluate_value(&operands[1]),
        ) {
            (ConstantValue::Int(l), ConstantValue::Int(r)) => ConstantValue::Bool(op(l, r)),
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_bool_not(&self, operands: &[LIRValue]) -> ConstantValue {
        assert!(operands.len() == 1);
        match self.evaluate_value(&operands[0]) {
            ConstantValue::Bool(b) => ConstantValue::Bool(!b),
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_bool_and(&self, operands: &[LIRValue]) -> ConstantValue {
        assert!(operands.len() == 2);
        match (
            self.evaluate_value(&operands[0]),
            self.evaluate_value(&operands[1]),
        ) {
            // TODO: Isn't this to much? What with opts with side effects
            // false && something() => false
            (ConstantValue::Bool(false), _) | (_, ConstantValue::Bool(false)) => {
                ConstantValue::Bool(false)
            }
            (ConstantValue::Bool(l), ConstantValue::Bool(r)) => ConstantValue::Bool(l && r),
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_bool_or(&self, operands: &[LIRValue]) -> ConstantValue {
        assert!(operands.len() == 2);
        match (
            self.evaluate_value(&operands[0]),
            self.evaluate_value(&operands[1]),
        ) {
            // TODO: Isn't this to much? What with opts with side effects
            // true || something() => true
            (ConstantValue::Bool(true), _) | (_, ConstantValue::Bool(true)) => {
                ConstantValue::Bool(true)
            }
            (ConstantValue::Bool(l), ConstantValue::Bool(r)) => ConstantValue::Bool(l || r),
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_string_concat(&self, operands: &[LIRValue]) -> ConstantValue {
        assert!(operands.len() == 2);
        match (
            self.evaluate_value(&operands[0]),
            self.evaluate_value(&operands[1]),
        ) {
            (ConstantValue::String(l), ConstantValue::String(r)) => {
                ConstantValue::String(format!("{}{}", l, r))
            }
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_string_length(&self, operands: &[LIRValue]) -> ConstantValue {
        assert!(operands.len() == 1);
        match self.evaluate_value(&operands[0]) {
            ConstantValue::String(str) => ConstantValue::Int(str.len() as i32),
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_string_eq(&self, operands: &[LIRValue]) -> ConstantValue {
        assert!(operands.len() == 2);
        match (
            self.evaluate_value(&operands[0]),
            self.evaluate_value(&operands[1]),
        ) {
            (ConstantValue::String(a), ConstantValue::String(b)) => ConstantValue::Bool(a == b),
            _ => ConstantValue::Unknown,
        }
    }

    fn fold_string_neq(&self, operands: &[LIRValue]) -> ConstantValue {
        assert!(operands.len() == 2);
        match (
            self.evaluate_value(&operands[0]),
            self.evaluate_value(&operands[1]),
        ) {
            (ConstantValue::String(a), ConstantValue::String(b)) => ConstantValue::Bool(a != b),
            _ => ConstantValue::Unknown,
        }
    }
}

// ======================
// Constant Propagation.
// ======================
impl ConstantFoldingAndPropagation {
    // Changes the instruction to use the previously calculated constants.
    fn propagate_constants_in_instructions(&self, instr: &mut LIRInstruction) -> bool {
        let mut changed = false;
        for operand in &mut instr.operands {
            if let LIRValue::Register(reg_id) = operand
                && let Some(const_val) = self.constants.get(reg_id)
            {
                let new_value = match const_val {
                    ConstantValue::Int(n) => Some(LIRValue::IntConst(*n)),
                    ConstantValue::Bool(b) => Some(LIRValue::BoolConst(*b)),
                    ConstantValue::String(s) => Some(LIRValue::StringConst(s.clone())),
                    ConstantValue::Null => Some(LIRValue::Null),
                    _ => None,
                };
                if let Some(new_val) = new_value {
                    *operand = new_val;
                    changed = true;
                }
            }
        }
        changed
    }

    // Changes the terminators to use the previously calculated constants.
    fn propagate_constants_in_terminator(&self, term: &mut LIRTerminator) -> bool {
        match term {
            LIRTerminator::Branch { condition, .. } => {
                if let LIRValue::Register(reg_id) = condition
                    && let Some(ConstantValue::Bool(b)) = self.constants.get(reg_id)
                {
                    *condition = LIRValue::BoolConst(*b);
                    return true;
                }
            }
            LIRTerminator::Return {
                value: Some(val), ..
            } => {
                if let LIRValue::Register(reg_id) = val
                    && let Some(const_val) = self.constants.get(reg_id)
                {
                    let new_value = match const_val {
                        ConstantValue::Int(n) => Some(LIRValue::IntConst(*n)),
                        ConstantValue::Bool(b) => Some(LIRValue::BoolConst(*b)),
                        ConstantValue::String(s) => Some(LIRValue::StringConst(s.clone())),
                        ConstantValue::Null => Some(LIRValue::Null),
                        _ => None,
                    };

                    if let Some(new_val) = new_value {
                        *val = new_val;
                        return true;
                    }
                }
            }
            _ => {}
        }
        false
    }
}

impl OptimizationPass for ConstantFoldingAndPropagation {
    fn name(&self) -> String {
        String::from("Constant Propagation")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        self.constants.clear();
        let mut changed = false;

        // First perform the constant calculation on the whole function.
        for block_id in &function.block_order {
            if let Some(block) = function.blocks.get(block_id) {
                for instr in &block.instructions {
                    self.analyze_instruction(instr);
                }
            }
        }

        // Now that the constants have been evaluated, change the CTE registers to constant values.
        for block_id in &function.block_order {
            if let Some(block) = function.blocks.get_mut(block_id) {
                for instr in &mut block.instructions {
                    if self.propagate_constants_in_instructions(instr) {
                        changed = true;
                    }
                }

                if let Some(term) = &mut block.terminator
                    && self.propagate_constants_in_terminator(term)
                {
                    changed = true;
                }
            }
        }
        changed
    }
}

impl Default for ConstantFoldingAndPropagation {
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

    // Helper to create a basic LIRFunction for testing.
    // Zakładam, że LIRBlock::new istnieje i przyjmuje BlockID. Jeśli używasz `with_id`, zmień `new` na `with_id`.
    fn create_test_function() -> LIRFunction {
        LIRFunction {
            name: StrID::from("test"),
            return_type: Type {
                kind: TypeKind::Void,
                position: SourcePosition(0, 0),
            },
            parameter_types: vec![],
            parameter_registers: vec![],
            registers: vec![],
            blocks: HashMap::new(),
            block_order: vec![],
            entry_block: BlockID(0),
            source_position: SourcePosition(0, 0),
        }
    }

    #[test]
    fn test_simple_integer_folding_and_propagation() {
        // r0 = 10
        // r1 = 20
        // r2 = r0 + r1
        // r3 = r2 * 2
        let mut function = create_test_function();
        let r0 = RegisterID(0);
        let r1 = RegisterID(1);
        let r2 = RegisterID(2);
        let r3 = RegisterID(3);

        let mut block = LIRBlock::with_id(BlockID(0));
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
            operation: LIROperation::IntAdd,
            destination: Some(r2),
            operands: vec![LIRValue::Register(r0), LIRValue::Register(r1)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::IntMul,
            destination: Some(r3),
            operands: vec![LIRValue::Register(r2), LIRValue::IntConst(2)],
            source_position: SourcePosition(0, 0),
        });

        function.blocks.insert(BlockID(0), block);
        function.block_order.push(BlockID(0));

        let mut pass = ConstantFoldingAndPropagation::new();
        let changed = pass.run_on_function(&mut function);

        assert!(changed);
        let final_block = function.blocks.get(&BlockID(0)).unwrap();
        let add_instr = &final_block.instructions[2];
        let mul_instr = &final_block.instructions[3];

        // Check that r0 and r1 were propagated into the add instruction
        assert_eq!(add_instr.operands[0], LIRValue::IntConst(10));
        assert_eq!(add_instr.operands[1], LIRValue::IntConst(20));

        // Check that the result of the add (r2=30) was propagated into the mul instruction
        assert_eq!(mul_instr.operands[0], LIRValue::IntConst(30));
        assert_eq!(mul_instr.operands[1], LIRValue::IntConst(2));
    }

    #[test]
    fn test_propagation_to_terminator() {
        // r0 = 5 > 3
        // branch r0, B1, B2
        let mut function = create_test_function();
        let r0 = RegisterID(0);
        let b0 = BlockID(0);
        let b1 = BlockID(1);
        let b2 = BlockID(2);

        let mut block = LIRBlock::with_id(b0);
        block.instructions.push(LIRInstruction {
            operation: LIROperation::IntGt,
            destination: Some(r0),
            operands: vec![LIRValue::IntConst(5), LIRValue::IntConst(3)],
            source_position: SourcePosition(0, 0),
        });
        block.terminator = Some(LIRTerminator::Branch {
            condition: LIRValue::Register(r0),
            if_true: b1,
            if_false: b2,
            position: SourcePosition(0, 0),
        });

        function.blocks.insert(b0, block);
        function.block_order.push(b0);

        let mut pass = ConstantFoldingAndPropagation::new();
        let changed = pass.run_on_function(&mut function);

        assert!(changed);

        let final_block = function.blocks.get(&b0).unwrap();
        match &final_block.terminator {
            Some(LIRTerminator::Branch { condition, .. }) => {
                assert_eq!(*condition, LIRValue::BoolConst(true));
            }
            _ => panic!("Expected Branch terminator"),
        }
    }

    #[test]
    fn test_boolean_short_circuit_and() {
        // r0 = false
        // r1 = some_unknown_val
        // r2 = r0 && r1 // Should evaluate to false without knowing r1
        // r3 = r2
        let mut function = create_test_function();
        let r0 = RegisterID(0);
        let r1 = RegisterID(1);
        let r2 = RegisterID(2);
        let r3 = RegisterID(3);

        let mut block = LIRBlock::with_id(BlockID(0));
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r0),
            operands: vec![LIRValue::BoolConst(false)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::BoolAnd,
            destination: Some(r2),
            operands: vec![LIRValue::Register(r0), LIRValue::Register(r1)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r3),
            operands: vec![LIRValue::Register(r2)],
            source_position: SourcePosition(0, 0),
        });

        function.blocks.insert(BlockID(0), block);
        function.block_order.push(BlockID(0));

        let mut pass = ConstantFoldingAndPropagation::new();
        let changed = pass.run_on_function(&mut function);

        assert!(changed);
        let final_block = function.blocks.get(&BlockID(0)).unwrap();
        let final_assign = &final_block.instructions[2];
        // The value of r2 (false) should be propagated to the final assignment.
        assert_eq!(final_assign.operands[0], LIRValue::BoolConst(false));
    }

    #[test]
    fn test_string_operations() {
        // r0 = "hello"
        // r1 = " world"
        // r2 = r0 + r1     // "hello world"
        // r3 = length(r2)  // 11
        let mut function = create_test_function();
        let r0 = RegisterID(0);
        let r1 = RegisterID(1);
        let r2 = RegisterID(2);
        let r3 = RegisterID(3);

        let mut block = LIRBlock::with_id(BlockID(0));
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r0),
            operands: vec![LIRValue::StringConst("hello".to_string())],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::Assign,
            destination: Some(r1),
            operands: vec![LIRValue::StringConst(" world".to_string())],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::StringConcat,
            destination: Some(r2),
            operands: vec![LIRValue::Register(r0), LIRValue::Register(r1)],
            source_position: SourcePosition(0, 0),
        });
        block.instructions.push(LIRInstruction {
            operation: LIROperation::StringLength,
            destination: Some(r3),
            operands: vec![LIRValue::Register(r2)],
            source_position: SourcePosition(0, 0),
        });

        function.blocks.insert(BlockID(0), block);
        function.block_order.push(BlockID(0));

        let mut pass = ConstantFoldingAndPropagation::new();
        let changed = pass.run_on_function(&mut function);

        assert!(changed);
        let final_block = function.blocks.get(&BlockID(0)).unwrap();
        let length_instr = &final_block.instructions[3];

        assert_eq!(
            length_instr.operands[0],
            LIRValue::StringConst("hello world".to_string())
        );
        assert_eq!(pass.constants.get(&r3), Some(&ConstantValue::Int(11)));
    }

    #[test]
    fn test_no_changes_on_non_constants() {
        // r2 = r0 + r1 (r0 and r1 are unknown (e.g., function parameters))
        let mut function = create_test_function();
        let r0 = RegisterID(0);
        let r1 = RegisterID(1);
        let r2 = RegisterID(2);

        // Pretend r0 and r1 are function parameters
        function.parameter_registers.push(r0);
        function.parameter_registers.push(r1);

        let mut block = LIRBlock::with_id(BlockID(0));
        let original_instr = LIRInstruction {
            operation: LIROperation::IntAdd,
            destination: Some(r2),
            operands: vec![LIRValue::Register(r0), LIRValue::Register(r1)],
            source_position: SourcePosition(0, 0),
        };
        block.instructions.push(original_instr.clone());

        function.blocks.insert(BlockID(0), block);
        function.block_order.push(BlockID(0));

        let mut pass = ConstantFoldingAndPropagation::new();
        let changed = pass.run_on_function(&mut function);

        assert!(!changed);
        let final_block = function.blocks.get(&BlockID(0)).unwrap();
        let final_instr = &final_block.instructions[0];

        // The instruction should be identical because no constants could be propagated.
        assert_eq!(final_instr.operands, original_instr.operands);
    }
}
