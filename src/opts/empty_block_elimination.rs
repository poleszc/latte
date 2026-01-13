/// Empty Block elimination:
/// Eliminates blocks which only contain a jump to the next block.
use crate::{lir::*, opts::OptimizationPass};
use std::collections::{HashMap, HashSet};

pub struct EmptyBlockElimination;

impl EmptyBlockElimination {
    pub fn new() -> Self {
        Self
    }

    fn find_stupid_blocks(&self, func: &LIRFunction) -> HashMap<BlockID, BlockID> {
        let mut redirection = HashMap::new();

        for &id in &func.block_order {
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
                            if_true, if_false, ..
                        } => {
                            if let Some(&new_t) = redirections.get(if_true) {
                                *if_true = new_t;
                                changed = true;
                            }
                            if let Some(&new_f) = redirections.get(if_false) {
                                *if_false = new_f;
                                changed = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        changed
    }

    /// Build a mapping from (Removed Block) -> (List of its predecessors).
    /// Used later when fixing phi functions.
    fn build_source_map(
        &self,
        func: &LIRFunction,
        redirects: &HashMap<BlockID, BlockID>,
    ) -> HashMap<BlockID, Vec<BlockID>> {
        let mut source_map: HashMap<BlockID, Vec<BlockID>> = HashMap::new();
        let blocks_to_be_removed: HashSet<_> = redirects.keys().collect();

        for (&pred_id, block) in &func.blocks {
            if let Some(term) = &block.terminator {
                for successor in term.successors() {
                    if blocks_to_be_removed.contains(&successor) {
                        source_map.entry(successor).or_default().push(pred_id);
                    }
                }
            }
        }
        source_map
    }

    // Fixes the phi arguments in block, since new predecessors may
    // have appeared when relinking blocks.
    fn fix_phis(
        &self,
        func: &mut LIRFunction,
        source_map: &HashMap<BlockID, Vec<BlockID>>,
    ) -> bool {
        let mut changed = false;
        for block in func.blocks.values_mut() {
            for instr in &mut block.instructions {
                if matches!(instr.operation, LIROperation::Phi) {
                    let mut operands_to_add = Vec::new();
                    for chunk in instr.operands.chunks_exact(2) {
                        let value = &chunk[0];
                        if let LIRValue::Block(pred_id) = chunk[1] {
                            if let Some(original_sources) = source_map.get(&pred_id) {
                                // If the phi argument is the one referencing a removed block, then we add
                                // an argument for each of it's predecessors. We don't remove the removed
                                // block argument, this will be done by dead block elimination.
                                for &source_block_id in original_sources {
                                    operands_to_add.push(value.clone());
                                    operands_to_add.push(LIRValue::Block(source_block_id));
                                }
                            }
                        }
                    }

                    if !operands_to_add.is_empty() {
                        instr.operands.extend(operands_to_add);
                        changed = true;
                    }
                }
            }
        }
        changed
    }

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
        // Build a mapping from (Removed Block) -> (List of its predecessors)
        let source_map = self.build_source_map(function, &stupid_blocks);

        // Change the values in terminators.
        let changed1 = self.apply_redirections(function, &stupid_blocks);
        // Fix the phi functions of the blocks that we're targets of the remove block.
        let changed2 = self.fix_phis(function, &source_map);
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
