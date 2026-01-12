use std::collections::HashMap;

/// Global Common Sub-expression Elimination:
///
use crate::{lir::*, opts::OptimizationPass};

#[derive(Debug, Clone, Hash)]
struct Expression {
    operation: LIROperation,
    operands: Vec<LIRValue>,
}

impl Expression {
    /// Constructs a valid expression object from a LIR Instruction.
    /// Returns None if the LIR Instruction is not a valid expression (f.e. has side effects).
    fn from_lir_instruction(instr: &LIRInstruction) -> Option<Self> {
        if Self::has_side_effects(&instr.operation) {
            return None;
        }

        // If the instruction is commutative, we sort the operands so expressions like these are reduced:
        // x = a + b
        // y = b + a
        let mut operands = instr.operands.clone();
        if Self::is_commutative(&instr.operation) && operands.len() == 2 {
            operands.sort();
        }
        Some(Expression {
            operation: instr.operation.clone(),
            operands,
        })
    }

    fn has_side_effects(op: &LIROperation) -> bool {
        matches!(
            op,
            LIROperation::Call
                | LIROperation::VCall
                | LIROperation::NewArray
                | LIROperation::NewObject
                | LIROperation::ArraySet
                | LIROperation::FieldSet
                | LIROperation::Phi
                | LIROperation::Assign
        )
    }

    fn is_commutative(op: &LIROperation) -> bool {
        matches!(
            op,
            LIROperation::IntAdd
                | LIROperation::IntMul
                | LIROperation::IntEq
                | LIROperation::IntNeq
                | LIROperation::BoolAnd
                | LIROperation::BoolOr
        )
    }

    /// Whether an expression uses a given register in arguments.
    fn uses_register(&self, reg: RegisterID) -> bool {
        self.operands
            .iter()
            .any(|op| matches!(op, LIRValue::Register(r) if *r == reg))
    }
}

pub struct GCSE {
    /// Expression -> RegisterID where it was first calculated.
    expressions: HashMap<Expression, RegisterID>,
    /// Which registers should be replaced, by the already calculated values.
    replacements: HashMap<RegisterID, RegisterID>,
}

impl GCSE {
    pub fn new() -> Self {
        Self {
            expressions: HashMap::new(),
            replacements: HashMap::new(),
        }
    }

    pub fn analyze_lir(&self, function: &LIRFunction) -> i64 {
        todo!()
    }

    pub fn collect_expressions(&mut self, function: &LIRFunction) -> i64 {
        todo!()
    }

    pub fn eliminate_common_subexpressions(&self, function: &mut LIRFunction) -> i64 {
        todo!()
    }
}

impl OptimizationPass for GCSE {
    fn name(&self) -> String {
        String::from("Global Common Subexpressions Elimination")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        let analysis = self.analyze_lir(function);

        let mut changed = false;
        self.collect_expressions(function);
        if !self.replacements.is_empty() {
            self.eliminate_common_subexpressions(function);
            changed = true;
        }
        changed
    }
}

impl Default for GCSE {
    fn default() -> Self {
        Self::new()
    }
}
