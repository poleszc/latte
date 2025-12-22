use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    lir::{
        BlockID as LirBID, LIRBlock, LIRFunction, LIRInstruction, LIROperation, LIRTerminator,
        LIRValue, RegisterID, RegisterInfo,
        lowering::cfg::{DominanceFrontier, GraphAnalysis},
    },
    mir::{
        BlockID as MirBID, LocalID, MIRFunction, MIROperation, MIRPlace, MIRTerminator, MIRValue,
        TempID,
    },
    parser::Type,
    utils::StrID,
};

/// LocalID to a stack fo registers where it's last used.
type LocalsRegisterStack = HashMap<LocalID, Vec<RegisterID>>;

type PhiPlaceholders = HashMap<LirBID, HashMap<LocalID, RegisterID>>;
type PhiIncoming = HashMap<LirBID, HashMap<RegisterID, Vec<(LirBID, LIRValue)>>>;

pub struct LIRFunctionBuilder {
    /// Index of the next free register.
    register_index: usize,
    /// Mapping from MIR Local ids to a stack of registers. Used when renaming the instructions with DFS.
    locals_stack: LocalsRegisterStack,
    /// Temporaries are already in the SSA form and do not leak through blocks, so we treat them separately)
    temps: HashMap<TempID, RegisterID>,
    /// Stores the information about the type of register. RegisterID is the index in this vector.
    registers_info: Vec<RegisterInfo>,
    /// For each LIR block, stores a mapping of it's locals to the corresponding registers where it's first use if defined.
    phi_placeholders: PhiPlaceholders,
    /// For each LIR block, stores a mapping of it's locals to the corresponding registers where it's first use if defined.
    phi_incoming: PhiIncoming,

    locals_cache: HashMap<LocalID, (Type, Option<StrID>)>,
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
            locals_cache: HashMap::new(),
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
        // Create a locals cache for faster lookup.
        self.locals_cache = mir_function
            .locals
            .iter()
            .map(|l| (l.id, (l.typ.clone(), None)))
            .collect();

        // Handle temporaries first, since they are already in the SSA form and don't leak through blocks.
        for temp in &mir_function.temps {
            let reg = self.new_register(temp.typ.clone(), None);
            self.temps.insert(temp.id, reg);
        }

        let analysis = GraphAnalysis::analyze(mir_function);

        // Insert placeholder phi functions which will be filled later.
        self.insert_phi_functions(mir_function, &analysis.dominance_frontier);

        // Insert all function parameters to the variable stacks.
        let mut param_registers = Vec::new();
        for param in mir_function.locals.iter().filter(|l| l.is_parameter) {
            let reg = self.new_register(param.typ.clone(), param.name);
            param_registers.push(reg);
            self.locals_stack.entry(param.id).or_default().push(reg);
        }

        // Perform renaming of all instructions.
        let mut lir_blocks = HashMap::new();
        self.rename_recursive(
            mir_function.entry_block,
            mir_function,
            &analysis,
            &mut lir_blocks,
        );

        // Now that all instructions are changed, we fill the phi instructions.
        self.finalize_phis(&mut lir_blocks);

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
                .map(|b| LirBID(b.0))
                .collect(),
            entry_block: LirBID(mir_function.entry_block.0),
            source_position: mir_function.source_position,
        }
    }

    /// Fill the placeholder phis with arguments.
    fn finalize_phis(&self, blocks: &mut HashMap<LirBID, LIRBlock>) {
        for (lir_id, block) in blocks {
            for instr in &mut block.instructions {
                if instr.operation != LIROperation::Phi {
                    continue;
                }

                if let Some(dest_reg) = instr.destination
                    && let Some(incoming) =
                        self.phi_incoming.get(lir_id).and_then(|m| m.get(&dest_reg))
                {
                    for pred_id in &block.predecessors {
                        let val = incoming
                            .iter()
                            .find(|(p, _)| p == pred_id)
                            .map(|(_, v)| v.clone())
                            .unwrap();
                        instr.operands.push(val);
                        instr.operands.push(LIRValue::Block(*pred_id));
                    }
                }
            }
        }
    }

    /// Insert needed phi functions based on the dominance frontier.
    /// https://pfalcon.github.io/ssabook/latest/book-full.pdf (Algorithm 3.1)
    fn insert_phi_functions(&mut self, func: &MIRFunction, df: &DominanceFrontier) {
        // Look for all variables name in the program and create a mapping to all blocks containing them.
        let mut local_defs: HashMap<LocalID, HashSet<MirBID>> = HashMap::new();
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
            let mut worklist: VecDeque<MirBID> = def_blocks.iter().cloned().collect(); // Set of basic blocks that contain definitions of this local. (W)
            let mut visited = HashSet::new(); // Set of blocks where phi is added. (F)

            while let Some(n) = worklist.pop_front() {
                if let Some(frontier) = df.get(&n) {
                    for &f in frontier {
                        if !visited.contains(&f) {
                            // Add a placeholder phi to the block.
                            let lir_block = LirBID(f.0);
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
        block_id: MirBID,
        mir_func: &MIRFunction,
        cfg_analysis: &GraphAnalysis,
        lir_blocks: &mut HashMap<LirBID, LIRBlock>,
    ) {
        let lir_id = LirBID(block_id.0);
        let mut pushed_locals = Vec::new();
        let mut instructions = Vec::new();

        if let Some(phis_here) = self.phi_placeholders.get(&lir_id) {
            for (&loc_id, &phi_reg) in phis_here {
                self.locals_stack.entry(loc_id).or_default().push(phi_reg);
                pushed_locals.push(loc_id);

                instructions.push(LIRInstruction {
                    operation: LIROperation::Phi,
                    destination: Some(phi_reg),
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
                            destination: Some(new_reg),
                            operands,
                            source_position: instruction.source_position,
                        });
                    }
                    MIRPlace::Temp(temp_id) => {
                        instructions.push(LIRInstruction {
                            operation: op,
                            destination: Some(self.temps[temp_id]),
                            operands,
                            source_position: instruction.source_position,
                        });
                    }
                    _ => panic!("Field access and array access unhandled"),
                }
            } else {
                instructions.push(LIRInstruction {
                    operation: op,
                    destination: None,
                    operands,
                    source_position: instruction.source_position,
                });
            }
        }

        let lowered_terminator = mir_block
            .terminator
            .as_ref()
            .map(|t| self.lower_terminator(t));

        if let Some(block_successors) = cfg_analysis.successors.get(&block_id) {
            for &successor in block_successors {
                let succ_lir_id = LirBID(successor.0);
                if let Some(phis_in_succ) = self.phi_placeholders.get(&succ_lir_id) {
                    for (&loc_id, &phi_reg) in phis_in_succ {
                        let current_val =
                            match self.locals_stack.get(&loc_id).and_then(|s| s.last()) {
                                Some(&reg) => LIRValue::Register(reg),
                                None => LIRValue::Null, // TODO: This is bugged
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

        if let Some(children) = cfg_analysis.dominator_tree.get(&block_id) {
            for &child in children {
                self.rename_recursive(child, mir_func, cfg_analysis, lir_blocks);
            }
        }

        // Remove locals pushed in this DFS run.
        for loc_id in pushed_locals {
            self.locals_stack.get_mut(&loc_id).unwrap().pop();
        }

        // Populate a LIR block with a list of it's predecessors.
        // TODO: Maybe not needed?
        let mut final_preds = Vec::new();
        if let Some(p_list) = cfg_analysis.predecessors.get(&block_id) {
            for p in p_list {
                final_preds.push(LirBID(p.0));
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
                target: LirBID(target.0),
                position: *position,
            },
            MIRTerminator::Branch {
                condition,
                if_true,
                if_false,
                position,
            } => LIRTerminator::Branch {
                condition: self.lower_value(condition),
                if_true: LirBID(if_true.0),
                if_false: LirBID(if_false.0),
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
            MIRValue::Block(block_id) => LIRValue::Block(LirBID(block_id.0)),
            MIRValue::Function(func_name) => LIRValue::Function(*func_name),
            MIRValue::Class(class_name) => LIRValue::Class(*class_name),
            MIRValue::Unit => todo!("Unit"),
        }
    }
}
