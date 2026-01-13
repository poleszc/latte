/// Empty Block elimination:
/// Eliminates blocks which only contain a jump to the next block.
use crate::{lir::*, opts::OptimizationPass};
use std::collections::{HashMap, HashSet};

pub struct EmptyBlockElimination;

impl EmptyBlockElimination {
    pub fn new() -> Self {
        Self
    }

    /// Finds block which can be removed. Have empty instruction set and a terminator.
    fn find_stupid_blocks(&self, func: &LIRFunction) -> HashMap<BlockID, BlockID> {
        let mut redirection = HashMap::new();

        for &id in &func.block_order {
            // Skip the entry block, since it can't be a jump target.
            if id == func.entry_block {
                continue;
            }

            if let Some(block) = func.blocks.get(&id) {
                if block.instructions.is_empty() {
                    if let Some(LIRTerminator::Jump { target, .. }) = &block.terminator {
                        if *target != id {
                            redirection.insert(id, *target);
                        }
                    }
                }
            }
        }

        // Compress the paths so block chains like B1 -> B2 -> B3 -> B4 are handled in one iteration.
        let mut final_map = HashMap::new();
        for &start_node in redirection.keys() {
            let mut current = start_node;
            let mut visited = HashSet::new();

            // Detect cycles cause back edges may cause ones to appear.
            while let Some(&next) = redirection.get(&current) {
                if !visited.insert(current) {
                    break;
                }
                current = next;
            }

            if current != start_node {
                final_map.insert(start_node, current);
            }
        }
        final_map
    }

    /// Change the values in block terminators to new blocks, after removal of the empty ones.
    fn apply_redirections(
        &self,
        func: &mut LIRFunction,
        redirections: &HashMap<BlockID, BlockID>,
    ) -> bool {
        let mut changed = false;

        for src_id in func.block_order.clone() {
            if let Some(block) = func.blocks.get_mut(&src_id) {
                if let Some(term) = &mut block.terminator {
                    match term {
                        LIRTerminator::Jump { target, .. } => {
                            if let Some(&new_target) = redirections.get(target) {
                                *target = new_target;
                                changed = true;
                            }
                        }
                        LIRTerminator::Branch {
                            if_true,
                            if_false,
                            position,
                            ..
                        } => {
                            if let Some(&new_t) = redirections.get(if_true) {
                                *if_true = new_t;
                                changed = true;
                            }
                            if let Some(&new_f) = redirections.get(if_false) {
                                *if_false = new_f;
                                changed = true;
                            }
                            // If both branches lead to the same block, change the branch to a jump.
                            if *if_true == *if_false {
                                *term = LIRTerminator::Jump {
                                    target: *if_true,
                                    position: *position,
                                };
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        changed
    }

    // Fixes the phi arguments in block, since new predecessors may
    // have appeared when relinking blocks and some may have disappeared.
    fn fix_phis(
        &self,
        func: &mut LIRFunction,
        to_remove: &HashSet<BlockID>,
        original_preds: &HashMap<BlockID, Vec<BlockID>>,
    ) -> bool {
        let mut changed = false;

        for block in func.blocks.values_mut() {
            for instr in &mut block.instructions {
                if instr.operation == LIROperation::Phi {
                    let mut new_ops = Vec::new();
                    let mut instr_changed = false;

                    for chunk in instr.operands.chunks_exact(2) {
                        let val = chunk[0].clone();
                        let pred_id = match chunk[1] {
                            LIRValue::Block(id) => id,
                            _ => unreachable!(),
                        };

                        // If an arguments contains a removed block, we change it to the nearest alive predecessor.
                        if to_remove.contains(&pred_id) {
                            instr_changed = true;
                            let mut worklist = vec![pred_id];
                            let mut resolved = HashSet::new();
                            let mut visited = HashSet::new();

                            // Walk up the predecessor path and look for the nearest alive predecessor.
                            while let Some(curr) = worklist.pop() {
                                if !to_remove.contains(&curr) {
                                    resolved.insert(curr);
                                } else if visited.insert(curr) {
                                    if let Some(preds) = original_preds.get(&curr) {
                                        worklist.extend(preds);
                                    }
                                }
                            }

                            for res_id in resolved {
                                new_ops.push(val.clone());
                                new_ops.push(LIRValue::Block(res_id));
                            }
                        } else {
                            new_ops.push(val);
                            new_ops.push(LIRValue::Block(pred_id));
                        }
                    }

                    if instr_changed {
                        // Remove duplicate arguments which might have appeared with diamond like CFGs.
                        instr.operands = self.deduplicate_phi_args(new_ops);
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    /// Removes duplicate phi arguments - coming from the same block.
    /// This might have occurred when having a diamond like CFG.
    fn deduplicate_phi_args(&self, ops: Vec<LIRValue>) -> Vec<LIRValue> {
        let mut seen = HashSet::new();
        let mut final_ops = Vec::new();
        for chunk in ops.chunks_exact(2).rev() {
            if let LIRValue::Block(id) = chunk[1] {
                if seen.insert(id) {
                    final_ops.push(chunk[1].clone());
                    final_ops.push(chunk[0].clone());
                }
            }
        }
        final_ops.reverse();
        final_ops
    }

    /// Rebuild predecessor list for blocks once blocks have been removed.
    fn rebuild_predecessors(&self, func: &mut LIRFunction) {
        let mut new_preds: HashMap<BlockID, Vec<BlockID>> = HashMap::new();

        for &block_id in func.blocks.keys() {
            new_preds.insert(block_id, Vec::new());
        }

        for (&pred_id, block) in &func.blocks {
            if let Some(term) = &block.terminator {
                for successor_id in term.successors() {
                    if let Some(preds) = new_preds.get_mut(&successor_id) {
                        preds.push(pred_id);
                    }
                }
            }
        }

        for (id, block) in &mut func.blocks {
            let mut preds = new_preds.remove(id).unwrap_or_default();
            preds.sort();
            block.predecessors = preds;
        }
    }

    fn remove_dead_blocks(&self, func: &mut LIRFunction, redirects: &HashMap<BlockID, BlockID>) {
        let blocks_to_remove: HashSet<BlockID> = redirects.keys().cloned().collect();
        if blocks_to_remove.is_empty() {
            return;
        }
        func.blocks.retain(|id, _| !blocks_to_remove.contains(id));
        func.block_order.retain(|id| !blocks_to_remove.contains(id));
    }
}

impl OptimizationPass for EmptyBlockElimination {
    fn name(&self) -> String {
        String::from("Empty Block Elimination")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        // First find empty blocks.
        let stupid_blocks = self.find_stupid_blocks(function);
        if stupid_blocks.is_empty() {
            return false;
        }
        let to_remove: HashSet<BlockID> = stupid_blocks.keys().cloned().collect();

        // Save the original predecessors for them to be used later on when fixing phis.
        let original_preds: HashMap<BlockID, Vec<BlockID>> = function
            .blocks
            .iter()
            .map(|(id, b)| (*id, b.predecessors.clone()))
            .collect();

        // Change the values in terminators.
        let changed1 = self.apply_redirections(function, &stupid_blocks);
        // Fix the phi functions of the blocks that we're targets of the remove block.
        let changed2 = self.fix_phis(function, &to_remove, &original_preds);
        self.remove_dead_blocks(function, &stupid_blocks);
        // Rebuild the predecessor lists in LIRBlocks.
        self.rebuild_predecessors(function);

        changed1 || changed2
    }
}

impl Default for EmptyBlockElimination {
    fn default() -> Self {
        Self::new()
    }
}
