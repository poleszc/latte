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

        let mut final_map = HashMap::new();
        for &start_node in redirection.keys() {
            let mut current = start_node;
            let mut visited = HashSet::new();

            while let Some(&next) = redirection.get(&current) {
                if !visited.insert(current) {
                    break; // Cycle. TODO: Probably remove.
                }
                current = next;
            }

            if current != start_node {
                final_map.insert(start_node, current);
            }
        }

        final_map
    }

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

    fn fix_phis(&self, func: &mut LIRFunction, redirections: &HashMap<BlockID, BlockID>) -> bool {
        let mut changed = false;
        // Fix predecessor map
        let mut new_preds: HashMap<BlockID, Vec<BlockID>> = HashMap::new();
        for (&bid, block) in &func.blocks {
            if let Some(term) = &block.terminator {
                match term {
                    LIRTerminator::Jump { target, .. } => {
                        new_preds.entry(*target).or_default().push(bid);
                    }
                    LIRTerminator::Branch {
                        if_true, if_false, ..
                    } => {
                        new_preds.entry(*if_true).or_default().push(bid);
                        new_preds.entry(*if_false).or_default().push(bid);
                    }
                    _ => {}
                }
            }
        }

        // Fix Phis which may have arguments from the removed blocks.
        for &bid in &func.block_order {
            let current_predecessors = new_preds.get(&bid).cloned().unwrap_or_default();
            let pred_set: HashSet<BlockID> = current_predecessors.iter().cloned().collect();

            if let Some(block) = func.blocks.get_mut(&bid) {
                block.predecessors = current_predecessors;

                for instr in &mut block.instructions {
                    if instr.operation == LIROperation::Phi {
                        let mut new_phi_operands = Vec::new();
                        let mut processed_preds = HashSet::new();

                        for chunk in instr.operands.chunks_exact(2) {
                            let val = &chunk[0];
                            let old_label = match chunk[1] {
                                LIRValue::Block(id) => id,
                                _ => continue,
                            };

                            if pred_set.contains(&old_label) {
                                if processed_preds.insert(old_label) {
                                    new_phi_operands.push(val.clone());
                                    new_phi_operands.push(LIRValue::Block(old_label));
                                }
                            } else {
                                for &p in &block.predecessors {
                                    if !processed_preds.contains(&p) {
                                        new_phi_operands.push(val.clone());
                                        new_phi_operands.push(LIRValue::Block(p));
                                        processed_preds.insert(p);
                                    }
                                }
                            }
                        }

                        if new_phi_operands.len() == 2 {
                            instr.operation = LIROperation::Assign;
                            instr.operands = vec![new_phi_operands[0].clone()];
                            changed = true;
                        } else {
                            instr.operands = new_phi_operands;
                        }
                    }
                }
            }
        }
        changed
    }
}

impl OptimizationPass for EmptyBlockElimination {
    fn name(&self) -> String {
        String::from("Empty Block Elimination")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        let stupid_blocks = self.find_stupid_blocks(function);
        if stupid_blocks.is_empty() {
            return false;
        }
        dbg!(&stupid_blocks);
        let changed1 = self.apply_redirections(function, &stupid_blocks);

        println!("{}", &function);

        let changed2 = self.fix_phis(function, &stupid_blocks);
        println!("{}", &function);
        changed1 || changed2
    }

}

impl Default for EmptyBlockElimination {
    fn default() -> Self {
        Self::new()
    }
}
