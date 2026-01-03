/// Branch folding:
/// If a branch instruction depending on a constant value exists in
/// the program, we change it to a jump.
use crate::{lir::*, opts::OptimizationPass};

pub struct BranchFolding;

impl BranchFolding {
    pub fn new() -> Self {
        Self
    }

    fn fold_branch(term: &mut LIRTerminator) -> bool {
        if let LIRTerminator::Branch {
            condition,
            if_true,
            if_false,
            position,
        } = term
        {
            let const_value = match condition {
                LIRValue::BoolConst(b) => Some(*b),
                _ => None,
            };

            if let Some(value) = const_value {
                let target = if value { *if_true } else { *if_false };
                *term = LIRTerminator::Jump {
                    target,
                    position: *position,
                }
            }
            return true;
        }
        false
    }
}
impl OptimizationPass for BranchFolding {
    fn name(&self) -> String {
        String::from("Branch Folding")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        let mut changed = false;

        for block_id in &function.block_order {
            if let Some(block) = function.blocks.get_mut(block_id) {
                if let Some(term) = &mut block.terminator {
                    if Self::fold_branch(term) {
                        changed = true;
                    }
                }
            }
        }

        changed
    }

    fn run_on_program(&mut self, program: &mut LIRProgram) -> bool {
        let mut changed = false;
        for func_name in &program.function_order {
            if let Some(function) = program.functions.get_mut(func_name) {
                if self.run_on_function(function) {
                    changed = true;
                }
            }
        }
        changed
    }
}

impl Default for BranchFolding {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourcePosition;

    #[test]
    fn test_branch_folding_true() {
        let mut term = LIRTerminator::Branch {
            condition: LIRValue::BoolConst(true),
            if_true: BlockID(1),
            if_false: BlockID(2),
            position: SourcePosition(0, 0),
        };

        let changed = BranchFolding::fold_branch(&mut term);

        assert!(changed);
        match term {
            LIRTerminator::Jump { target, .. } => {
                assert_eq!(target, BlockID(1));
            }
            _ => panic!("Expected Jump terminator"),
        }
    }

    #[test]
    fn test_branch_folding_false() {
        let mut term = LIRTerminator::Branch {
            condition: LIRValue::BoolConst(false),
            if_true: BlockID(1),
            if_false: BlockID(2),
            position: SourcePosition(0, 0),
        };

        let changed = BranchFolding::fold_branch(&mut term);

        assert!(changed);
        match term {
            LIRTerminator::Jump { target, .. } => {
                assert_eq!(target, BlockID(2));
            }
            _ => panic!("Expected Jump terminator"),
        }
    }

    #[test]
    fn test_branch_folding_no_constant() {
        let mut term = LIRTerminator::Branch {
            condition: LIRValue::Register(RegisterID(0)),
            if_true: BlockID(1),
            if_false: BlockID(2),
            position: SourcePosition(0, 0),
        };

        let changed = BranchFolding::fold_branch(&mut term);

        assert!(!changed);
        assert!(matches!(term, LIRTerminator::Branch { .. }));
    }
}
