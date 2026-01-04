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
