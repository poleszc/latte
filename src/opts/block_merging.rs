/// Block Merging:
/// Merge blocks if:
/// - Source ends with a simple jump to dest
/// - Dest has only one predecessor - source
use crate::{lir::*, opts::OptimizationPass};

pub struct BlockMerging;

impl BlockMerging {
    pub fn new() -> Self {
        Self
    }

    fn merge_blocks(&self, func: &mut LIRFunction) -> bool {
        for src_bid in &func.block_order {
            let dst_bid = if let Some(block) = func.blocks.get(src_bid)
                && let Some(LIRTerminator::Jump { target, .. }) = &block.terminator
            {
                *target
            } else {
                continue;
            };
            let dst_block = func.blocks.get(&dst_bid).unwrap();
            if dst_block.predecessors.len() != 1 || dst_block.predecessors[0] != *src_bid {
                continue;
            }

            // If we got here, it's okay to merge the blocks.
            let mut dst_block = func.blocks.remove(&dst_bid).unwrap(); // Remove dest from the block map.
            let src_block = func.blocks.get_mut(src_bid).unwrap();
            src_block.instructions.append(&mut dst_block.instructions);
            src_block.terminator = dst_block.terminator;

            // Change Phis in successors of dest so they reference src.
            if let Some(term) = &src_block.terminator {
                for succ_id in term.successors() {
                    if let Some(succ_block) = func.blocks.get_mut(&succ_id) {
                        for p in &mut succ_block.predecessors {
                            if *p == dst_bid {
                                *p = *src_bid;
                            }
                        }

                        for instr in &mut succ_block.instructions {
                            if instr.operation == LIROperation::Phi {
                                for chunk in instr.operands.chunks_exact_mut(2) {
                                    if let LIRValue::Block(id) = &mut chunk[1]
                                        && *id == dst_bid
                                    {
                                        *id = *src_bid;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            func.block_order.retain(|&id| id != dst_bid);
            return true;
        }
        false
    }
}

impl OptimizationPass for BlockMerging {
    fn name(&self) -> String {
        String::from("Block Merging")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        let mut total_changed = false;
        while self.merge_blocks(function) {
            total_changed = true;
        }
        total_changed
    }
}

impl Default for BlockMerging {
    fn default() -> Self {
        Self::new()
    }
}
