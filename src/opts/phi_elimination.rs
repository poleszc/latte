use crate::{lir::*, opts::OptimizationPass};

pub struct PhiElimination;

impl PhiElimination {
    pub fn new() -> Self {
        Self
    }

    /// Checks if a PHI instruction can be simplified to a single Assign.
    fn try_simplify_phi(instr: &LIRInstruction) -> Option<LIRValue> {
        if instr.operation != LIROperation::Phi || instr.operands.is_empty() {
            return None;
        }

        let mut common_value: Option<LIRValue> = None;

        for chunk in instr.operands.chunks(2) {
            let val = &chunk[0];
            
            if matches!(val, LIRValue::Undefined) {
                return None;
            }

            // Two different values, can't be simplified.
            if let Some(ref existing) = common_value {
                if existing != val {
                    return None;
                }
            } else {
                common_value = Some(val.clone());
            }
        }
        common_value
    }
}

impl OptimizationPass for PhiElimination {
    fn name(&self) -> String {
        String::from("Phi Elimination")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        let mut changed = false;

        for block_id in &function.block_order {
            if let Some(block) = function.blocks.get_mut(block_id) {
                for instr in &mut block.instructions {
                    if let Some(redundant_value) = Self::try_simplify_phi(instr) {
                        instr.operation = LIROperation::Assign;
                        instr.operands = vec![redundant_value];
                        changed = true;
                    }
                }
            }
        }

        changed
    }
}

impl Default for PhiElimination {
    fn default() -> Self {
        Self::new()
    }
}