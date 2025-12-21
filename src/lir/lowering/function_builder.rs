use core::panic;
use std::{collections::HashMap, hash::Hash};

use clap::builder::Str;

use crate::{
    SourcePosition,
    lir::{lowering::function_builder, *},
    mir::{self, *},
    parser::{Block, Type},
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
    phi_finalized: HashMap<crate::lir::BlockID, Vec<LIRInstruction>>,
}

impl LIRFunctionBuilder {
    pub fn new() -> Self {
        Self {
            register_index: 0,
            locals_stack: HashMap::new(),
            temps: HashMap::new(),
            registers_info: Vec::new(),
            phi_placeholders: HashMap::new(),
            phi_finalized: HashMap::new(),
        }
    }

    fn new_register(&mut self, typ: Type, name: Option<StrID>) -> RegisterID {
        let id = RegisterID(self.register_index);
        self.register_index += 1;
        self.registers_info.push(RegisterInfo { id, typ, name });
        id
    }

    pub fn lower(&mut self, mir_function: &MIRFunction) -> LIRFunction {
        // Handle temporaries first, since they are already in the SSA form and don't leak through blocks.
        for temp in &mir_function.temps {
            let reg = self.new_register(temp.typ.clone(), None);
            self.temps.insert(temp.id, reg);
        }

        let (preds, succs) = self.analyze_cfg(&mir_function);
        let idom = self.compute_immediate_dominators(&mir_function, &preds);


        LIRFunction {
            name: mir_function.name,
            return_type: mir_function.return_type.clone(),
            parameter_types: mir_function.parameter_types.clone(),
            parameter_registers: vec![],
            registers: self.registers_info.clone(),
            blocks: HashMap::new(),
            block_order: vec![],
            entry_block: crate::lir::BlockID(mir_function.entry_block.0),
            source_position: mir_function.source_position,
        }
    }
}

// ======================
// Lowering
// ======================
impl LIRFunctionBuilder {
    fn lower_operation(&self, op: &MIROperation) -> LIROperation {
        match op {
            MIROperation::Assign => LIROperation::Phi,
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
            MIRValue::Block(_block_id) => todo!("Block"),
            MIRValue::Function(_str_id) => todo!("Function"),
            MIRValue::Class(_str_id) => todo!("Class"),
            MIRValue::Unit => todo!("Unit"),
        }
    }
}

// ======================
// Graph algorithms
// ======================
impl LIRFunctionBuilder {
    /// Calculates a list of predecessors and successors for each block.
    fn analyze_cfg(
        &self,
        func: &MIRFunction,
    ) -> (
        HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
        HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
    ) {
        let mut preds = HashMap::new();
        let mut succ = HashMap::new();

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

            // Insert THIS block as a predecessor of it's all successors.
            for &s in &successors_list {
                preds.entry(s).or_insert_with(Vec::new).push(block.id);
            }

            succ.insert(block.id, successors_list);
        }
        (preds, succ)
    }

    // https://pages.cs.wisc.edu/~fischer/cs701.f08/lectures/Lecture19.4up.pdf
    fn compute_immediate_dominators(
        &self,
        mir_func: &MIRFunction,
        preds: &HashMap<crate::mir::BlockID, Vec<crate::mir::BlockID>>,
    ) -> HashMap<crate::mir::BlockID, crate::mir::BlockID> {
        let mut idom = HashMap::new();
        let entry = mir_func.entry_block;
        idom.insert(entry, entry); // Entry dominates itself.

        let mut changed = true;
        while changed {
            changed = false;
            for &block in &mir_func.block_order {
                if block == entry {
                    // Skip the entry block since it already has a calculated idom.
                    continue;
                }
                let block_preds = &preds[&block];
                let first_processed_pred = block_preds
                    .iter()
                    .find(|p| idom.contains_key(p))
                    .expect("Every block should have a processed predecessor");
                let mut new_idom = *first_processed_pred;

                // For all predecessors of the block look for the common closest common predecessor of it's predecessors.
                for i in 1..block_preds.len() {
                    let p = block_preds[i];
                    if idom.contains_key(&p) {
                        new_idom = self.intersect(new_idom, p, &idom, &mir_func.block_order);
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
    fn intersect(
        &self,
        mut b1: crate::mir::BlockID,
        mut b2: crate::mir::BlockID,
        idom: &HashMap<crate::mir::BlockID, crate::mir::BlockID>,
        order: &[crate::mir::BlockID],
    ) -> crate::mir::BlockID {
        // The `order` block list is in reverse post order. If a block is later on the list, then it's `later` in the DFS graph of the CFG.
        // TODO: Optimise so we have a map BlockId -> index in block order
        let pos = |id| order.iter().position(|&x| x == id).unwrap();

        while b1 != b2 {
            // If b2 is "later" in the graph then b1, we take a step upwards with b1.
            while pos(b1) > pos(b2) {
                b1 = idom[&b1];
            }
            // If b1 is "later" in the graph then b2, we take a step upwards with b2.
            while pos(b2) > pos(b1) {
                b2 = idom[&b2];
            }
        }
        b1
    }
}
