/// Dead Block elimination:
/// Eliminates unreachable blocks and fixes up the phi functions after elimination.
/// Unreachable block might have occurred when performing branch folding.
use crate::{lir::*, opts::OptimizationPass};
use std::collections::{HashMap, HashSet};

pub struct DeadBlockElimination {
    reachable: HashSet<BlockID>,
}

impl DeadBlockElimination {
    pub fn new() -> Self {
        Self {
            reachable: HashSet::new(),
        }
    }

    /// Performs a BFS starting from the entry block and populates the `reachable` set.
    fn find_reachable_blocks(&mut self, function: &LIRFunction) {
        self.reachable.clear();
        let mut worklist = vec![function.entry_block];

        while let Some(block_id) = worklist.pop() {
            // Skip already visited and insert visited.
            if !self.reachable.insert(block_id) {
                continue;
            }

            if let Some(block) = function.blocks.get(&block_id)
                && let Some(term) = &block.terminator
            {
                match term {
                    LIRTerminator::Jump { target, .. } => {
                        worklist.push(*target);
                    }
                    LIRTerminator::Branch {
                        if_true, if_false, ..
                    } => {
                        worklist.push(*if_true);
                        worklist.push(*if_false);
                    }
                    LIRTerminator::Return { .. } | LIRTerminator::Unreachable { .. } => {}
                }
            }
        }
    }

    /// Removes all blocks which don't exist in the `reachable` map.
    /// Returns true if changes where made, false otherwise.
    fn remove_unreachable_blocks(&self, function: &mut LIRFunction) -> bool {
        let old_block_count = function.blocks.len();

        // Keep only the block which are reachable.
        function
            .blocks
            .retain(|block_id, _| self.reachable.contains(block_id));
        function
            .block_order
            .retain(|block_id| self.reachable.contains(block_id));
        old_block_count != function.blocks.len()
    }

    /// Fixes the predecessor list and phi functions by checking actual CFG edges.
    /// Returns true if changes where made, false otherwise.
    fn fix_cfg(&self, function: &mut LIRFunction) -> bool {
        let mut changed = false;

        // Build the edge map after some edges have been removed, after changing branches to jumps.
        let mut actual_edges: HashMap<BlockID, HashSet<BlockID>> = HashMap::new();
        for (bid, block) in &function.blocks {
            if let Some(term) = &block.terminator {
                match term {
                    LIRTerminator::Jump { target, .. } => {
                        actual_edges.entry(*target).or_default().insert(*bid);
                    }
                    LIRTerminator::Branch {
                        if_true, if_false, ..
                    } => {
                        actual_edges.entry(*if_true).or_default().insert(*bid);
                        actual_edges.entry(*if_false).or_default().insert(*bid);
                    }
                    _ => {}
                }
            }
        }

        // For each block, fix the predecessor list and phi functions for them to not include removed blocks.
        for block_id in &function.block_order.clone() {
            let actual_preds = actual_edges.get(block_id);

            if let Some(block) = function.blocks.get_mut(block_id) {
                let old_pred_len = block.predecessors.len();
                block.predecessors.retain(|p| {
                    // Predecessor must be reachable and must contain an edge to 'this" block.
                    actual_preds.is_some_and(|set| set.contains(p))
                });
                if block.predecessors.len() != old_pred_len {
                    changed = true;
                }

                // Remove removed blocks from phi arguments.
                let current_preds: HashSet<BlockID> = block.predecessors.iter().cloned().collect();
                for instr in &mut block.instructions {
                    if instr.operation != LIROperation::Phi {
                        continue;
                    }

                    let old_phi_len = instr.operands.len();
                    let mut new_ops = Vec::new();

                    for chunk in instr.operands.chunks(2) {
                        if let LIRValue::Block(pred_id) = chunk[1] {
                            // Leave the argument if the block is an actual predecessor.
                            if current_preds.contains(&pred_id) {
                                new_ops.push(chunk[0].clone());
                                new_ops.push(chunk[1].clone());
                            }
                        }
                    }

                    if new_ops.len() != old_phi_len {
                        instr.operands = new_ops;
                        changed = true;
                    }
                }
            }
        }
        changed
    }
}

impl OptimizationPass for DeadBlockElimination {
    fn name(&self) -> String {
        String::from("Dead Block Elimination")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        self.find_reachable_blocks(function);
        let block_removed = self.remove_unreachable_blocks(function);
        let cfg_fixed = self.fix_cfg(function);

        block_removed || cfg_fixed
    }
}

impl Default for DeadBlockElimination {
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

    #[test]
    fn test_unreachable_detection() {
        // B0 -> B1 -> B3
        // B2 is unreachable

        let mut func = LIRFunction {
            name: StrID::from("test"),
            return_type: Type {
                kind: TypeKind::Void,
                position: SourcePosition(0, 0),
            },
            parameter_types: vec![],
            parameter_registers: vec![],
            registers: vec![],
            blocks: HashMap::new(),
            block_order: vec![BlockID(0), BlockID(1), BlockID(2), BlockID(3)],
            entry_block: BlockID(0),
            source_position: SourcePosition(0, 0),
        };

        func.blocks.insert(
            BlockID(0),
            LIRBlock {
                id: BlockID(0),
                instructions: vec![],
                terminator: Some(LIRTerminator::Jump {
                    target: BlockID(1),
                    position: SourcePosition(0, 0),
                }),
                predecessors: vec![],
            },
        );

        func.blocks.insert(
            BlockID(1),
            LIRBlock {
                id: BlockID(1),
                instructions: vec![],
                terminator: Some(LIRTerminator::Jump {
                    target: BlockID(3),
                    position: SourcePosition(0, 0),
                }),
                predecessors: vec![BlockID(0)],
            },
        );

        func.blocks.insert(
            BlockID(2),
            LIRBlock {
                id: BlockID(2),
                instructions: vec![],
                terminator: Some(LIRTerminator::Jump {
                    target: BlockID(3),
                    position: SourcePosition(0, 0),
                }),
                predecessors: vec![],
            },
        );

        func.blocks.insert(
            BlockID(3),
            LIRBlock {
                id: BlockID(3),
                instructions: vec![],
                terminator: Some(LIRTerminator::Return {
                    value: None,
                    position: SourcePosition(0, 0),
                }),
                predecessors: vec![BlockID(1), BlockID(2)],
            },
        );

        let mut uce = DeadBlockElimination::new();
        let changed = uce.run_on_function(&mut func);

        assert!(changed);
        assert_eq!(func.blocks.len(), 3);
        assert!(!func.blocks.contains_key(&BlockID(2)));

        let b3 = func.blocks.get(&BlockID(3)).unwrap();
        assert_eq!(b3.predecessors.len(), 1);
        assert_eq!(b3.predecessors[0], BlockID(1));
    }
}
