use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

pub trait CFG {
    type BID: Copy + Hash + Eq + Ord;

    fn entry_block(&self) -> Self::BID;
    fn block_order(&self) -> Vec<Self::BID>;
    fn successors(&self, block_id: Self::BID) -> Vec<Self::BID>;
}

/// BlockID -> list of predecessors.
pub type PredecessorMap<BID> = HashMap<BID, Vec<BID>>;
/// BlockID -> list of successors.
pub type SuccessorMap<BID> = HashMap<BID, Vec<BID>>;
pub type DominatorTree<BID> = HashMap<BID, Vec<BID>>;
pub type DominanceFrontier<BID> = HashMap<BID, HashSet<BID>>;
pub type ImmediateDominators<BID> = HashMap<BID, BID>;
pub type ReversePostOrder<BID> = Vec<BID>;

/// A POD storing the results of the CFG analysis
pub struct GraphAnalysis<BID> {
    pub predecessors: PredecessorMap<BID>,
    pub successors: SuccessorMap<BID>,
    pub dominance_frontier: DominanceFrontier<BID>,
    pub dominator_tree: DominatorTree<BID>,
}

impl<BID: Copy + Hash + Eq + Ord> GraphAnalysis<BID> {
    /// Perform analysis of the CFG graph and return all data needed for SSA transformation.
    pub fn analyze<G: CFG<BID = BID>>(graph: &G) -> Self {
        let (preds, succs) = compute_predecessors_and_successors(graph);
        let rpo = compute_reverse_post_order(graph.entry_block(), &succs);
        let idom = compute_immediate_dominators(&rpo, &preds);
        let df = compute_dominance_frontier(&rpo, &idom, &preds);
        let dom_tree = build_dom_tree(&idom);

        Self {
            predecessors: preds,
            successors: succs,
            dominance_frontier: df,
            dominator_tree: dom_tree,
        }
    }
}

/// Calculates a predecessors and successors for each block in the MIR function.
fn compute_predecessors_and_successors<G: CFG>(
    func: &G,
) -> (PredecessorMap<G::BID>, SuccessorMap<G::BID>) {
    let mut preds = PredecessorMap::new();
    let mut succ = SuccessorMap::new();

    let order = func.block_order();
    for &block_id in &order {
        preds.insert(block_id, Vec::new());
        succ.insert(block_id, Vec::new());
    }

    for &block_id in &order {
        let successors_list = func.successors(block_id);
        // Insert THIS block as a predecessor of all it's successors.
        for &s in &successors_list {
            preds.get_mut(&s).unwrap().push(block_id);
        }
        succ.insert(block_id, successors_list);
    }
    (preds, succ)
}

/// Computes the reverse post order of the blocks needed for computing the
/// immediate dominators and dominance frontiers with fast convergence.
fn compute_reverse_post_order<BID: Copy + Hash + Eq>(entry: BID, succs: &SuccessorMap<BID>) -> ReversePostOrder<BID> {
    let mut post_order = Vec::new();
    let mut visited = HashSet::new();

    fn walk<BID: Copy + Hash + Eq>(
        curr: BID,
        succs: &SuccessorMap<BID>,
        visited: &mut HashSet<BID>,
        post_order: &mut Vec<BID>,
    ) {
        visited.insert(curr);
        if let Some(children) = succs.get(&curr) {
            for &child in children {
                if !visited.contains(&child) {
                    walk(child, succs, visited, post_order);
                }
            }
        }
        post_order.push(curr);
    }
    walk(entry, succs, &mut visited, &mut post_order);

    post_order.reverse();
    post_order
}

/// Computes the immediate dominator for each block.
/// Reverse post order is needed for fast convergence.
/// https://www.cs.tufts.edu/~nr/cs257/archive/keith-cooper/dom14.pdf
fn compute_immediate_dominators<BID: Copy + Hash + Eq + Ord>(
    rpo_order: &ReversePostOrder<BID>,
    preds: &PredecessorMap<BID>,
) -> ImmediateDominators<BID> {
    let mut idom = ImmediateDominators::new();
    let entry = rpo_order[0];
    idom.insert(entry, entry); // Entry dominates itself.

    // Helper mapping from block id to it's index in the rpo order.
    let rpo_map: HashMap<BID, usize> = rpo_order.iter().enumerate().map(|(i, &b)| (b, i)).collect();

    let mut changed = true;
    while changed {
        changed = false;
        // Skip the entry block, as it has been handled.
        for &block in rpo_order.iter().filter(|&&b| b != entry) {
            let block_preds = &preds[&block];

            // Pick the first processed predecessor of this block.
            let mut new_idom = *block_preds
                .iter()
                .find(|p| idom.contains_key(p))
                .expect("Every block should have a processed predecessor");

            // For all other predecessors of the block look for the common closest common predecessor of it's predecessors.
            for &p in block_preds {
                // Handle only the block for which the idom is already calculated. Others will be calculated in next iteration.
                if p != new_idom && idom.contains_key(&p) {
                    new_idom = intersect(new_idom, p, &idom, &rpo_map);
                }
            }

            if idom.get(&block) != Some(&new_idom) {
                idom.insert(block, new_idom);
                changed = true;
            }
        }
    }
    idom
}

/// Find the closest common predecessor for two blocks.
/// https://www.cs.tufts.edu/~nr/cs257/archive/keith-cooper/dom14.pdf
fn intersect<BID: Copy + Hash + Eq>(
    mut b1: BID,
    mut b2: BID,
    idom: &ImmediateDominators<BID>,
    rpo_map: &HashMap<BID, usize>, // Mapping from block ID to it's index in the reverse post order.
) -> BID {
    while b1 != b2 {
        // If b1 is "later" in the graph then b2, we take a step upwards with b1.
        while rpo_map[&b1] > rpo_map[&b2] {
            b1 = idom[&b1];
        }
        // If b2 is "later" in the graph then b1, we take a step upwards with b2.
        while rpo_map[&b2] > rpo_map[&b1] {
            b2 = idom[&b2];
        }
    }
    b1
}

/// Calculates the dominance frontier for all blocks.
/// Returns a mapping from a block_id, to blocks where the variables defined in them die.
/// Reverse post order is needed for fast convergence.
/// https://www.cs.tufts.edu/~nr/cs257/archive/keith-cooper/dom14.pdf
fn compute_dominance_frontier<BID: Copy + Hash + Eq + Ord>(
    rpo: &ReversePostOrder<BID>,
    idom: &ImmediateDominators<BID>,
    preds: &PredecessorMap<BID>,
) -> DominanceFrontier<BID> {
    let mut df = DominanceFrontier::new();
    for &b in rpo {
        df.insert(b, HashSet::new());
    }

    for &b in rpo {
        if let Some(preds) = preds.get(&b)
            && preds.len() >= 2
        {
            for &p in preds {
                let mut runner = p;
                while runner != idom[&b] {
                    df.get_mut(&runner).unwrap().insert(b);
                    let next_runner = idom.get(&runner);

                    if next_runner.is_none() || next_runner == Some(&runner) {
                        break;
                    }
                    runner = *next_runner.unwrap();
                }
            }
        }
    }
    df
}

/// Reverse the idom mapping of (child -> parent) to (parent -> list of children)
/// https://pfalcon.github.io/ssabook/latest/book-full.pdf
fn build_dom_tree<BID: Copy + Hash + Eq + Ord>(idom: &ImmediateDominators<BID>) -> DominatorTree<BID> {
    let mut tree = DominatorTree::new();
    for (&b, &p) in idom {
        if b != p {
            tree.entry(p).or_default().push(b);
        }
    }

    for children in tree.values_mut() {
        children.sort();
    }

    tree
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourcePosition;
    use crate::mir::{BlockID as BID, MIRBlock, MIRFunction, MIRTerminator, MIRValue};
    use crate::parser::ast::{Type, TypeKind};
    use crate::utils::StrID;

    fn create_mock_func(num_blocks: usize) -> MIRFunction {
        MIRFunction {
            name: StrID::from("test"),
            return_type: Type {
                kind: TypeKind::Void,
                position: SourcePosition(0, 0),
            },
            parameter_types: vec![],
            locals: vec![],
            temps: vec![],
            blocks: (0..num_blocks)
                .map(|i| {
                    (
                        BID(i),
                        MIRBlock {
                            id: BID(i),
                            instructions: vec![],
                            terminator: None,
                        },
                    )
                })
                .collect(),
            block_order: (0..num_blocks).map(BID).collect(),
            entry_block: BID(0),
            source_position: SourcePosition(0, 0),
        }
    }

    #[test]
    fn test_cfg_analysis_diamond() {
        // 0 -> 1, 2; 1 -> 3; 2 -> 3
        let mut func = create_mock_func(4);
        func.blocks.get_mut(&BID(0)).unwrap().terminator = Some(MIRTerminator::Branch {
            condition: crate::mir::MIRValue::BoolConst(true),
            if_true: BID(1),
            if_false: BID(2),
            position: SourcePosition(0, 0),
        });
        func.blocks.get_mut(&BID(1)).unwrap().terminator = Some(MIRTerminator::Jump {
            target: BID(3),
            position: SourcePosition(0, 0),
        });
        func.blocks.get_mut(&BID(2)).unwrap().terminator = Some(MIRTerminator::Jump {
            target: BID(3),
            position: SourcePosition(0, 0),
        });

        let analysis = GraphAnalysis::analyze(&func);

        assert_eq!(analysis.predecessors[&BID(3)].len(), 2);
        assert!(analysis.dominance_frontier[&BID(1)].contains(&BID(3)));
        assert!(analysis.dominance_frontier[&BID(2)].contains(&BID(3)));
    }

    #[test]
    fn test_dominance_frontier_while() {
        // 0 (entry) -> 1 (header);
        // 1 -> 2 (body), 3 (exit);
        // 2 -> 1 (back-edge)
        let mut func = crate::lir::lowering::cfg::tests::create_mock_func(4);

        // 0 -> 1
        func.blocks.get_mut(&BID(0)).unwrap().terminator = Some(MIRTerminator::Jump {
            target: BID(1),
            position: SourcePosition(0, 0),
        });

        // 1 -> 2, 3
        func.blocks.get_mut(&BID(1)).unwrap().terminator = Some(MIRTerminator::Branch {
            condition: MIRValue::BoolConst(true),
            if_true: BID(2),
            if_false: BID(3),
            position: SourcePosition(0, 0),
        });

        // 2 -> 1
        func.blocks.get_mut(&BID(2)).unwrap().terminator = Some(MIRTerminator::Jump {
            target: BID(1),
            position: SourcePosition(0, 0),
        });

        let analysis = GraphAnalysis::analyze(&func);

        // 1 should dominate over 2.
        // Dominance frontier of 2 should be 1.
        assert!(analysis.dominance_frontier[&BID(2)].contains(&BID(1)));

        // Dominance frontier of 1 should be 1
        assert!(analysis.dominance_frontier[&BID(1)].contains(&BID(1)));
        assert_eq!(analysis.dominance_frontier[&BID(1)].len(), 1);
    }
}
