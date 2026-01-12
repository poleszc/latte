/// Global Common Sub-expression Elimination:
use crate::{lir::*, opts::OptimizationPass, utils::cfg::GraphAnalysis};
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
                | LIROperation::FieldGet
                | LIROperation::ArrayGet
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
}

pub struct GlobalCSE {
    /// Expression -> RegisterID where it was first calculated.
    expressions: HashMap<Expression, RegisterID>,
    /// Which registers should be replaced, by the already calculated values.
    replacements: HashMap<RegisterID, RegisterID>,
}

impl GlobalCSE {
    pub fn new() -> Self {
        Self {
            expressions: HashMap::new(),
            replacements: HashMap::new(),
        }
    }

    fn collect_expressions(
        &mut self,
        block_id: BlockID,
        dom_tree: &HashMap<BlockID, Vec<BlockID>>,
        function: &LIRFunction,
        changed: &mut bool,
    ) {
        let mut added_expressions = Vec::new();
        let block = &function.blocks[&block_id];

        for instr in &block.instructions {
            if let Some(expr) = Expression::from_lir_instruction(instr)
                && let Some(dest) = instr.destination
            {
                if let Some(&existing_reg) = self.expressions.get(&expr) {
                    // Duplicate expression found.
                    self.replacements.insert(dest, existing_reg);
                    *changed = true;
                } else {
                    // This ia a first occurrence of the expression.
                    self.expressions.insert(expr.clone(), dest);
                    added_expressions.push(expr);
                }
            }
        }

        // Visit all children in the dominator tree.
        if let Some(children) = dom_tree.get(&block_id) {
            for &child in children {
                self.collect_expressions(child, dom_tree, function, changed);
            }
        }

        // Remove the expression added in this block, so they don't spill.
        for expr in added_expressions {
            self.expressions.remove(&expr);
        }
    }

    fn eliminate_common_subexpressions(&self, function: &mut LIRFunction) -> bool {
        let mut changed = false;

        for block_id in &function.block_order {
            if let Some(block) = function.blocks.get_mut(block_id) {
                let old_len = block.instructions.len();

                // Remove the instructions of the registers we have already calculated.
                block.instructions.retain(|instr| {
                    if let Some(dest) = instr.destination {
                        !self.replacements.contains_key(&dest)
                    } else {
                        true
                    }
                });

                if block.instructions.len() != old_len {
                    changed = true;
                }

                // Replace all usages of the eliminated registers with new values.
                for instr in &mut block.instructions {
                    for operand in &mut instr.operands {
                        if self.replace_value(operand) {
                            changed = true;
                        }
                    }
                }

                if let Some(term) = &mut block.terminator
                    && self.replace_terminator(term)
                {
                    changed = true;
                }
            }
        }

        for reg_info in &mut function.registers {
            if self.replacements.contains_key(&reg_info.id) {
                reg_info.id = RegisterID(usize::MAX);
            }
        }
        changed
    }

    fn replace_value(&self, value: &mut LIRValue) -> bool {
        if let LIRValue::Register(reg) = value
            && let Some(new_reg) = self.replacements.get(reg)
        {
            *value = LIRValue::Register(*new_reg);
            return true;
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

impl OptimizationPass for GlobalCSE {
    fn name(&self) -> String {
        String::from("Global Common Subexpressions Elimination")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        let analysis = GraphAnalysis::analyze(function);
        let mut changed = false;
        self.expressions.clear();
        self.replacements.clear();

        self.collect_expressions(
            function.entry_block,
            &analysis.dominator_tree,
            function,
            &mut changed,
        );
        if !self.replacements.is_empty() {
            let eliminated = self.eliminate_common_subexpressions(function);
            changed = changed || eliminated;
        }
        changed
    }
}

impl Default for GlobalCSE {
    fn default() -> Self {
        Self::new()
    }
}
