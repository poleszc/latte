use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    lir::*,
    mir::{self, *},
    parser::Type,
    utils::StrID,
};

pub struct LIRFunctionBuilder {
    /// Index of the next free register.
    register_index: usize,
    /// Mapping from MIR Local ids to a stack of registers.
    locals_stack: HashMap<LocalID, Vec<RegisterID>>,
    /// Temporaries are already in the SSA form and do not leak through blocks, so we treat them separately)
    temps: HashMap<TempID, RegisterID>,
    /// RegisterID is the index in this vector. Stores the information about the type of register.
    registers_info: Vec<RegisterInfo>,

    phi_placeholders: HashMap<crate::lir::BlockID, HashMap<LocalID, RegisterID>>,
    phi_incoming:
        HashMap<crate::lir::BlockID, HashMap<RegisterID, Vec<(crate::lir::BlockID, LIRValue)>>>,
}

impl LIRFunctionBuilder {
    pub fn new() -> Self {
        Self {
            register_index: 0,
            locals_stack: HashMap::new(),
            temps: HashMap::new(),
            registers_info: Vec::new(),
            phi_placeholders: HashMap::new(),
            phi_incoming: HashMap::new(),
        }
    }

    fn new_register(&mut self, typ: Type, name: Option<StrID>) -> RegisterID {
        let id = RegisterID(self.register_index);
        self.register_index += 1;
        self.registers_info.push(RegisterInfo { id, typ, name });
        id
    }

    /// Lowers a MIR function to LIR function. Transforms the code to a minimal SSA form.
    /// https://pfalcon.github.io/ssabook/latest/book-full.pdf
    pub fn lower(&mut self, mir_function: &MIRFunction) -> LIRFunction {
        // Handle temporaries first, since they are already in the SSA form and don't leak through blocks.
        for temp in &mir_function.temps {
            let reg = self.new_register(temp.typ.clone(), None);
            self.temps.insert(temp.id, reg);
        }

        let (preds, succs) = compute_predecessors_and_successors(&mir_function);
        let rpo_order = compute_reverse_post_order(mir_function.entry_block, &succs);
        let idom = compute_immediate_dominators(&rpo_order, &preds);
        let dominance_frontier = compute_dominance_frontier(&rpo_order, &idom, &preds);

        // Insert placeholder phi functions which will be filled later.
        self.insert_phi_functions(&mir_function, &dominance_frontier);

        let dom_tree = build_dom_tree(&idom);
        let mut lir_blocks = HashMap::new();

        // Insert all function parameters to the variable stacks.
        let mut param_registers = Vec::new();
        for param in mir_function.locals.iter().filter(|l| l.is_parameter) {
            let reg = self.new_register(param.typ.clone(), param.name);
            param_registers.push(reg);
            self.locals_stack.entry(param.id).or_default().push(reg);
        }

        // Perform renaming of all instructions.
        self.rename_recursive(
            mir_function.entry_block,
            &mir_function,
            &dom_tree,
            &succs,
            &preds,
            &mut lir_blocks,
        );

        // Now fill all phi function arguments with the actual values.
        for (lir_id, block) in &mut lir_blocks {
            let final_preds = &block.predecessors;
            for instr in &mut block.instructions {
                if instr.operation == LIROperation::Phi {
                    if let Some(incoming_map) = self.phi_incoming.get(lir_id) {
                        if let Some(args) = incoming_map.get(&instr.destination) {
                            for pred_id in final_preds {
                                let val = args
                                    .iter()
                                    .find(|(p, _)| p == pred_id)
                                    .map(|(_, v)| v.clone())
                                    .unwrap_or(LIRValue::IntConst(0));
                                instr.operands.push(val);
                            }
                        }
                    }
                }
            }
        }

        LIRFunction {
            name: mir_function.name,
            return_type: mir_function.return_type.clone(),
            parameter_types: mir_function.parameter_types.clone(),
            parameter_registers: param_registers,
            registers: self.registers_info.clone(),
            blocks: lir_blocks,
            block_order: mir_function
                .block_order
                .iter()
                .map(|b| crate::lir::BlockID(b.0))
                .collect(),
            entry_block: crate::lir::BlockID(mir_function.entry_block.0),
            source_position: mir_function.source_position,
        }
    }

    /// Insert needed phi functions based on the dominance frontier.
    /// https://pfalcon.github.io/ssabook/latest/book-full.pdf (Algorithm 3.1)
    fn insert_phi_functions(
        &mut self,
        func: &MIRFunction,
        df: &HashMap<crate::mir::BlockID, HashSet<crate::mir::BlockID>>,
    ) {
        // Look for all variables name in the program and create a mapping to all blocks containing them.
        let mut local_defs: HashMap<LocalID, HashSet<crate::mir::BlockID>> = HashMap::new();
        for block_ix in &func.block_order {
            let block = &func.blocks[block_ix];
            for instr in &block.instructions {
                if let Some(MIRPlace::Local(id)) = &instr.destination {
                    local_defs.entry(*id).or_default().insert(block.id);
                }
            }
        }

        // For each variable in the program insert PHI for it.
        for (local_id, def_blocks) in local_defs {
            let mut worklist: VecDeque<crate::mir::BlockID> = def_blocks.iter().cloned().collect(); // Set of basic blocks that contain definitions of this local. (W)
            let mut visited = HashSet::new(); // Set of blocks where phi is added. (F)

            while let Some(n) = worklist.pop_front() {
                if let Some(frontier) = df.get(&n) {
                    for &f in frontier {
                        if !visited.contains(&f) {
                            // Add a placeholder phi to the block.
                            let lir_block = crate::lir::BlockID(f.0);
                            let typ = func
                                .locals
                                .iter()
                                .find(|l| l.id == local_id)
                                .unwrap()
                                .typ
                                .clone();
                            let reg = self.new_register(typ, None);
                            // If entry for this block didn't exist we create it, if not we insert into the existing.
                            self.phi_placeholders
                                .entry(lir_block)
                                .or_default()
                                .insert(local_id, reg);

                            // Mark the block as visited.
                            visited.insert(f);
                            if !def_blocks.contains(&f) {
                                worklist.push_back(f);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Performs lowering and a recursive rename for all instructions, starting from the `block_id`
    /// https://pfalcon.github.io/ssabook/latest/book-full.pdf
    fn rename_recursive(
        &mut self,
        block_id: crate::mir::BlockID,
        mir_func: &MIRFunction,
        dom_tree: &HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
        succs: &HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
        preds: &HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
        lir_blocks: &mut HashMap<crate::lir::BlockID, LIRBlock>,
    ) {
        let lir_id = crate::lir::BlockID(block_id.0);
        let mut pushed_locals = Vec::new();
        let mut instructions = Vec::new();

        if let Some(phis_here) = self.phi_placeholders.get(&lir_id) {
            for (&loc_id, &phi_reg) in phis_here {
                self.locals_stack.entry(loc_id).or_default().push(phi_reg);
                pushed_locals.push(loc_id);

                instructions.push(LIRInstruction {
                    operation: LIROperation::Phi,
                    destination: phi_reg,
                    operands: Vec::new(),
                    source_position: mir_func.source_position,
                });
            }
        }
        let mir_block = &mir_func.blocks[&block_id];
        for instruction in &mir_block.instructions {
            let operands = instruction
                .operands
                .iter()
                .map(|v| self.lower_value(v))
                .collect();
            let op = self.lower_operation(&instruction.operation);

            if let Some(dest) = &instruction.destination {
                match dest {
                    MIRPlace::Local(local_id) => {
                        let info = mir_func.locals.iter().find(|l| l.id == *local_id).unwrap();
                        let new_reg = self.new_register(info.typ.clone(), info.name);

                        self.locals_stack
                            .entry(*local_id)
                            .or_default()
                            .push(new_reg);
                        pushed_locals.push(*local_id);

                        instructions.push(LIRInstruction {
                            operation: op,
                            destination: new_reg,
                            operands,
                            source_position: instruction.source_position,
                        });
                    }
                    MIRPlace::Temp(temp_id) => {
                        instructions.push(LIRInstruction {
                            operation: op,
                            destination: self.temps[temp_id],
                            operands,
                            source_position: instruction.source_position,
                        });
                    }
                    _ => panic!("Field access and array access unhandled"),
                }
            } else {
                instructions.push(LIRInstruction {
                    operation: op,
                    destination: RegisterID(0),
                    operands,
                    source_position: instruction.source_position,
                });
            }
        }

        let lowered_terminator = mir_block
            .terminator
            .as_ref()
            .map(|t| self.lower_terminator(t));

        if let Some(block_successors) = succs.get(&block_id) {
            for &successor in block_successors {
                let succ_lir_id = crate::lir::BlockID(successor.0);
                if let Some(phis_in_succ) = self.phi_placeholders.get(&succ_lir_id) {
                    for (&loc_id, &phi_reg) in phis_in_succ {
                        let current_val =
                            match self.locals_stack.get(&loc_id).and_then(|s| s.last()) {
                                Some(&reg) => LIRValue::Register(reg),
                                None => LIRValue::IntConst(0), // TODO: This is bugged
                            };

                        self.phi_incoming
                            .entry(succ_lir_id)
                            .or_default()
                            .entry(phi_reg)
                            .or_default()
                            .push((lir_id, current_val));
                    }
                }
            }
        }

        if let Some(children) = dom_tree.get(&block_id) {
            for &child in children {
                self.rename_recursive(child, mir_func, dom_tree, succs, preds, lir_blocks);
            }
        }

        for loc_id in pushed_locals {
            self.locals_stack.get_mut(&loc_id).unwrap().pop();
        }

        let mut final_preds = Vec::new();
        if let Some(p_list) = preds.get(&block_id) {
            for p in p_list {
                final_preds.push(crate::lir::BlockID(p.0));
            }
        }

        lir_blocks.insert(
            lir_id,
            LIRBlock {
                id: lir_id,
                instructions,
                terminator: lowered_terminator,
                predecessors: final_preds,
            },
        );
    }
}

// ======================
// Lowering
// ======================
impl LIRFunctionBuilder {
    fn lower_operation(&self, op: &MIROperation) -> LIROperation {
        match op {
            MIROperation::Assign => LIROperation::Assign,
            MIROperation::Call => LIROperation::Call,
            MIROperation::VCall => LIROperation::VCall,
            MIROperation::IntAdd => LIROperation::IntAdd,
            MIROperation::IntSub => LIROperation::IntSub,
            MIROperation::IntMul => LIROperation::IntMul,
            MIROperation::IntDiv => LIROperation::IntDiv,
            MIROperation::IntMod => LIROperation::IntMod,
            MIROperation::IntNeg => LIROperation::IntNeg,
            MIROperation::IntLt => LIROperation::IntLt,
            MIROperation::IntLe => LIROperation::IntLe,
            MIROperation::IntGt => LIROperation::IntGt,
            MIROperation::IntGe => LIROperation::IntGe,
            MIROperation::IntEq => LIROperation::IntEq,
            MIROperation::IntNeq => LIROperation::IntNeq,
            MIROperation::BoolAnd => LIROperation::BoolAnd,
            MIROperation::BoolOr => LIROperation::BoolOr,
            MIROperation::BoolNot => LIROperation::BoolNot,
            MIROperation::StringConcat => LIROperation::StringConcat,
            MIROperation::StringEq => LIROperation::StringEq,
            MIROperation::StringNeq => LIROperation::StringNeq,
            MIROperation::StringLength => LIROperation::StringLength,
            MIROperation::NewArray => LIROperation::NewArray,
            MIROperation::ArrayGet => LIROperation::ArrayGet,
            MIROperation::ArraySet => LIROperation::ArraySet,
            MIROperation::ArrayLength => LIROperation::ArrayLength,
            MIROperation::NewObject => LIROperation::NewObject,
            MIROperation::FieldGet => LIROperation::FieldGet,
            MIROperation::FieldSet => LIROperation::FieldSet,
            MIROperation::IsNull => LIROperation::IsNull,
        }
    }

    fn lower_terminator(&self, term: &MIRTerminator) -> LIRTerminator {
        match term {
            MIRTerminator::Jump { target, position } => LIRTerminator::Jump {
                target: crate::lir::BlockID(target.0),
                position: *position,
            },
            MIRTerminator::Branch {
                condition,
                if_true,
                if_false,
                position,
            } => LIRTerminator::Branch {
                condition: self.lower_value(condition),
                if_true: crate::lir::BlockID(if_true.0),
                if_false: crate::lir::BlockID(if_false.0),
                position: *position,
            },
            MIRTerminator::Return { value, position } => LIRTerminator::Return {
                value: value.as_ref().map(|v| self.lower_value(v)),
                position: *position,
            },
        }
    }

    fn lower_value(&self, val: &MIRValue) -> LIRValue {
        match val {
            MIRValue::IntConst(n) => LIRValue::IntConst(*n),
            MIRValue::BoolConst(b) => LIRValue::BoolConst(*b),
            MIRValue::Local(local_id) => {
                LIRValue::Register(*self.locals_stack[local_id].last().unwrap())
            }
            MIRValue::Temp(temp_id) => LIRValue::Register(self.temps[temp_id]),
            MIRValue::StringConst(s) => LIRValue::StringConst(s.clone()),
            MIRValue::Null => LIRValue::Null,
            // TODO: Remove those?
            MIRValue::Block(block_id) => LIRValue::Block(crate::lir::BlockID(block_id.0)),
            MIRValue::Function(func_name) => LIRValue::Function(*func_name),
            MIRValue::Class(class_name) => LIRValue::Class(*class_name),
            MIRValue::Unit => todo!("Unit"),
        }
    }
}

// ======================
// Graph utils
// ======================
/// Calculates a predecessors and successors for each block in the MIR function.
fn compute_predecessors_and_successors(
    func: &MIRFunction,
) -> (
    HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
    HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
) {
    let mut preds = HashMap::new();
    let mut succ = HashMap::new();

    for &block_id in &func.block_order {
        preds.insert(block_id, Vec::new());
        succ.insert(block_id, Vec::new());
    }

    for block in func.blocks.values() {
        // Insert all successors for the block.
        let mut successors_list = Vec::new();
        if let Some(term) = &block.terminator {
            match term {
                MIRTerminator::Jump { target, .. } => {
                    successors_list.push(*target);
                }
                MIRTerminator::Branch {
                    if_true, if_false, ..
                } => {
                    successors_list.push(*if_true);
                    successors_list.push(*if_false);
                }

                _ => {}
            }
        }

        // Insert THIS block as a predecessor of all it's successors.
        for &s in &successors_list {
            preds.get_mut(&s).unwrap().push(block.id);
        }

        succ.insert(block.id, successors_list);
    }
    (preds, succ)
}

/// Reverse the idom mapping of (child -> parent) to (parent -> list of children)
/// https://pfalcon.github.io/ssabook/latest/book-full.pdf
fn build_dom_tree(
    idom: &HashMap<mir::BlockID, mir::BlockID>,
) -> HashMap<mir::BlockID, Vec<mir::BlockID>> {
    let mut tree: HashMap<mir::BlockID, Vec<mir::BlockID>> = HashMap::new();
    for (&b, &p) in idom {
        if b != p {
            tree.entry(p).or_default().push(b);
        }
    }
    tree
}

/// Calculates the dominance frontier for all blocks.
/// Returns a mapping from a block_id, to blocks where the variables defined in them die.
/// Reverse post order is needed for fast convergence.
/// https://www.cs.tufts.edu/~nr/cs257/archive/keith-cooper/dom14.pdf
fn compute_dominance_frontier(
    rpo_order: &Vec<mir::BlockID>,
    idom: &HashMap<crate::mir::BlockID, crate::mir::BlockID>,
    preds: &HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
) -> HashMap<crate::mir::BlockID, HashSet<crate::mir::BlockID>> {
    let mut df = HashMap::new();
    for &b in rpo_order {
        df.insert(b, HashSet::new());
    }

    for &b in rpo_order {
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

/// Computes the immediate dominator for each block.
/// Reverse post order is needed for fast convergence.
/// https://www.cs.tufts.edu/~nr/cs257/archive/keith-cooper/dom14.pdf
fn compute_immediate_dominators(
    rpo_order: &Vec<mir::BlockID>,
    preds: &HashMap<mir::BlockID, Vec<mir::BlockID>>,
) -> HashMap<mir::BlockID, mir::BlockID> {
    let mut idom = HashMap::new();
    let entry = rpo_order[0];
    idom.insert(entry, entry); // Entry dominates itself.

    // Helper mapping from block id to it's index in the rpo order.
    let rpo_map: HashMap<mir::BlockID, usize> =
        rpo_order.iter().enumerate().map(|(i, &b)| (b, i)).collect();

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

            if let None = idom.get(&block) {
                idom.insert(block, new_idom);
                changed = true;
            }
        }
    }
    idom
}

/// Find the closest common predecessor for two blocks.
/// https://www.cs.tufts.edu/~nr/cs257/archive/keith-cooper/dom14.pdf
fn intersect(
    mut b1: crate::mir::BlockID,
    mut b2: crate::mir::BlockID,
    idom: &HashMap<crate::mir::BlockID, crate::mir::BlockID>,
    rpo_map: &HashMap<mir::BlockID, usize>, // Mapping from block ID to it's index in the reverse post order.
) -> crate::mir::BlockID {
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

/// Computes the reverse post order of the blocks needed for computing the
/// immediate dominators and dominance frontiers with fast convergence.
fn compute_reverse_post_order(
    entry: mir::BlockID,
    succs: &HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
) -> Vec<mir::BlockID> {
    let mut post_order = Vec::new();
    let mut visited = HashSet::new();

    fn walk(
        curr: mir::BlockID,
        succs: &HashMap<mir::BlockID, Vec<mir::BlockID>>,
        visited: &mut HashSet<mir::BlockID>,
        post_order: &mut Vec<mir::BlockID>,
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
