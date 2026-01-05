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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SSAId {
    Local(LocalID),
    Temp(TempID),
}

/// LocalID to a stack fo registers where it's last used.
type RegisterStack = HashMap<SSAId, Vec<RegisterID>>;

type PhiPlaceholders = HashMap<LirBID, HashMap<SSAId, RegisterID>>;
type PhiIncoming = HashMap<LirBID, HashMap<RegisterID, Vec<(LirBID, LIRValue)>>>;

pub struct LIRFunctionBuilder {
    /// Index of the next free register.
    register_index: usize,
    /// Mapping from MIR locals and temporaries to a stack of registers. Used when renaming the instructions with DFS.
    ssa_stack: RegisterStack,
    /// Stores the information about the type of register. RegisterID is the index in this vector.
    registers_info: Vec<RegisterInfo>,
    /// For each LIR block, stores a mapping of it's locals to the corresponding registers where it's first use if defined.
    phi_placeholders: PhiPlaceholders,
    /// For each LIR block, stores a mapping of it's locals to the corresponding registers where it's first use if defined.
    phi_incoming: PhiIncoming,

    locals_cache: HashMap<SSAId, (Type, Option<StrID>)>,
}

impl LIRFunctionBuilder {
    pub fn new() -> Self {
        Self {
            register_index: 0,
            ssa_stack: HashMap::new(),
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
        //
        for l in &mir_function.locals {
            let id = SSAId::Local(l.id);
            self.locals_cache.insert(id, (l.typ.clone(), l.name));
        }
        for t in &mir_function.temps {
            let id = SSAId::Temp(t.id);
            self.locals_cache.insert(id, (t.typ.clone(), None));
        }

        let analysis = GraphAnalysis::analyze(mir_function);

        // Insert placeholder phi functions which will be filled later.
        self.insert_phi_functions(mir_function, &analysis.dominance_frontier);

        // Insert all function parameters to the variable stacks.
        let mut param_registers = Vec::new();
        for param in mir_function.locals.iter().filter(|l| l.is_parameter) {
            let reg = self.new_register(param.typ.clone(), param.name);
            param_registers.push(reg);
            self.ssa_stack
                .entry(SSAId::Local(param.id))
                .or_default()
                .push(reg);
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
        let mut local_defs: HashMap<SSAId, HashSet<MirBID>> = HashMap::new();
        for block_ix in &func.block_order {
            let block = &func.blocks[block_ix];
            for instr in &block.instructions {
                if let Some(dest) = &instr.destination {
                    let ssa_id = match dest {
                        MIRPlace::Local(id) => SSAId::Local(*id),
                        MIRPlace::Temp(_) => continue,
                        _ => unreachable!(),
                    };
                    local_defs.entry(ssa_id).or_default().insert(block.id);
                }
            }
        }

        let mut sorted_ids: Vec<_> = local_defs.keys().cloned().collect();
        sorted_ids.sort_by_key(|id| match id {
            SSAId::Local(l) => (0, l.0),
            SSAId::Temp(l) => (1, l.0),
        });

        // For each variable in the program insert PHI for it.
        for ssa_id in sorted_ids {
            let def_blocks = &local_defs[&ssa_id];
            let mut worklist: VecDeque<MirBID> = def_blocks.iter().cloned().collect(); // Set of basic blocks that contain definitions of this local. (W)
            let mut visited = HashSet::new(); // Set of blocks where phi is added. (F)

            while let Some(n) = worklist.pop_front() {
                if let Some(frontier) = df.get(&n) {
                    for &f in frontier {
                        if !visited.contains(&f) {
                            // Add a placeholder phi to the block.
                            let lir_block = LirBID(f.0);
                            let typ = self.locals_cache[&ssa_id].0.clone();
                            let reg = self.new_register(typ, None);

                            // If entry for this block didn't exist we create it, if not we insert into the existing.
                            self.phi_placeholders
                                .entry(lir_block)
                                .or_default()
                                .insert(ssa_id, reg);

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
        let mut pushed_ssa_registers: Vec<SSAId> = Vec::new();
        let mut instructions = Vec::new();

        if let Some(phis_here) = self.phi_placeholders.get(&lir_id) {
            let mut sorted_phi_ids: Vec<_> = phis_here.keys().collect();
            sorted_phi_ids.sort_by_key(|id| match id {
                SSAId::Local(l) => (0, l.0),
                SSAId::Temp(t) => (1, t.0),
            });

            for ssa_id in sorted_phi_ids {
                let phi_reg = phis_here[ssa_id];
                self.ssa_stack.entry(*ssa_id).or_default().push(phi_reg);
                pushed_ssa_registers.push(*ssa_id);

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
                let ssa_id = match dest {
                    MIRPlace::Local(local_id) => SSAId::Local(*local_id),
                    MIRPlace::Temp(temp_id) => SSAId::Temp(*temp_id),
                    _ => unreachable!(),
                };

                let info = self.locals_cache[&ssa_id].clone();
                let new_reg = self.new_register(info.0.clone(), info.1);
                self.ssa_stack.entry(ssa_id).or_default().push(new_reg);
                pushed_ssa_registers.push(ssa_id);

                instructions.push(LIRInstruction {
                    operation: op,
                    destination: Some(new_reg),
                    operands,
                    source_position: instruction.source_position,
                });
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
            let mut sorted_successors = block_successors.clone();
            sorted_successors.sort_by_key(|bid| bid.0);

            for successor in sorted_successors {
                let succ_lir_id = LirBID(successor.0);
                if let Some(phis_in_succ) = self.phi_placeholders.get(&succ_lir_id) {
                    for (&ssa_id, &phi_reg) in phis_in_succ {
                        let current_val = match self.ssa_stack.get(&ssa_id).and_then(|s| s.last()) {
                            Some(&reg) => LIRValue::Register(reg),
                            None => LIRValue::Undefined,
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
            let mut sorted_children = children.clone();
            sorted_children.sort_by_key(|bid| bid.0);
            for child in sorted_children {
                self.rename_recursive(child, mir_func, cfg_analysis, lir_blocks);
            }
        }

        // Remove locals pushed in this DFS run.
        for loc_id in pushed_ssa_registers {
            self.ssa_stack.get_mut(&loc_id).unwrap().pop();
        }

        // Populate a LIR block with a list of it's predecessors.
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
            MIRTerminator::Unreachable { position } => LIRTerminator::Unreachable {
                position: *position,
            },
        }
    }

    fn lower_value(&self, val: &MIRValue) -> LIRValue {
        match val {
            MIRValue::IntConst(n) => LIRValue::IntConst(*n),
            MIRValue::BoolConst(b) => LIRValue::BoolConst(*b),
            MIRValue::Local(local_id) => {
                let ssa_id = SSAId::Local(*local_id);
                LIRValue::Register(*self.ssa_stack[&ssa_id].last().unwrap())
            }
            MIRValue::Temp(temp_id) => {
                let ssa_id = SSAId::Temp(*temp_id);
                LIRValue::Register(*self.ssa_stack[&ssa_id].last().unwrap())
            }
            MIRValue::StringConst(s) => LIRValue::StringConst(s.clone()),
            MIRValue::Null => LIRValue::Null,
            MIRValue::Block(block_id) => LIRValue::Block(LirBID(block_id.0)),
            MIRValue::Function(func_name) => LIRValue::Function(*func_name),
            MIRValue::Class(class_name) => LIRValue::Class(*class_name),
            MIRValue::Unit => panic!("Tried to lower a unit MIRValue"),
        }
    }
}

#[cfg(test)]
mod ssa_tests {
    use super::*;
    use crate::SourcePosition;
    use crate::mir::builders::MIRFunctionBuilder;
    use crate::mir::*;
    use crate::parser::ast::{Type, TypeKind};

    fn setup_builder() -> MIRFunctionBuilder {
        MIRFunctionBuilder::new(
            StrID::from("test"),
            Type {
                kind: TypeKind::Void,
                position: SourcePosition(0, 0),
            },
            vec![],
            SourcePosition(0, 0),
        )
    }

    #[test]
    fn test_ssa_variable_renaming() {
        // x = 1; x = x + 1; -> %r0 = 1; %r1 = %r0 + 1;
        let mut mir_builder = setup_builder();
        let ty_int = Type {
            kind: TypeKind::Int,
            position: SourcePosition(0, 0),
        };
        let x = mir_builder.create_local(Some(StrID::from("x")), ty_int.clone(), false, None);

        // x = 1
        mir_builder.emit_assign(
            MIRPlace::Local(x),
            MIRValue::IntConst(1),
            SourcePosition(0, 0),
        );
        // x = x + 1
        mir_builder.emit_binary(
            MIROperation::IntAdd,
            MIRPlace::Local(x),
            MIRValue::Local(x),
            MIRValue::IntConst(1),
            SourcePosition(0, 0),
        );
        mir_builder.emit_return(None, SourcePosition(0, 0));

        let mir_func = mir_builder.build();
        let mut lir_builder = LIRFunctionBuilder::new();
        let lir_func = lir_builder.lower(&mir_func);

        let entry_block = &lir_func.blocks[&lir_func.entry_block];

        // At least two instructions to two different registers should appear.
        let dest_reg: Vec<_> = entry_block
            .instructions
            .iter()
            .filter_map(|i| i.destination)
            .collect();

        assert!(dest_reg.len() >= 2);
        assert_ne!(dest_reg[0], dest_reg[1], "Duplicate register usage");
    }

    #[test]
    fn test_phi_insertion_in_diamond_cfg() {
        // if (c) { x = 1 } else { x = 2 }; print(x)
        // Phi with two args should appear
        let mut mir_builder = setup_builder();
        let ty_int = Type {
            kind: TypeKind::Int,
            position: SourcePosition(0, 0),
        };
        let x = mir_builder.create_local(Some(StrID::from("x")), ty_int.clone(), false, None);

        let true_b = mir_builder.create_block();
        let false_b = mir_builder.create_block();
        let merge_b = mir_builder.create_block();

        mir_builder.emit_branch(
            MIRValue::BoolConst(true),
            true_b,
            false_b,
            SourcePosition(0, 0),
        );

        mir_builder.set_current_block(true_b);
        mir_builder.emit_assign(
            MIRPlace::Local(x),
            MIRValue::IntConst(1),
            SourcePosition(0, 0),
        );
        mir_builder.emit_jump(merge_b, SourcePosition(0, 0));

        mir_builder.set_current_block(false_b);
        mir_builder.emit_assign(
            MIRPlace::Local(x),
            MIRValue::IntConst(2),
            SourcePosition(0, 0),
        );
        mir_builder.emit_jump(merge_b, SourcePosition(0, 0));

        mir_builder.set_current_block(merge_b);
        mir_builder.emit_call(
            None,
            StrID::from("printInt"),
            vec![MIRValue::Local(x)],
            SourcePosition(0, 0),
        );
        mir_builder.emit_return(None, SourcePosition(0, 0));

        let mir_func = mir_builder.build();
        let mut lir_builder = LIRFunctionBuilder::new();
        let lir_func = lir_builder.lower(&mir_func);

        let merge_lir_id = LirBID(merge_b.0);
        let merge_block = &lir_func.blocks[&merge_lir_id];

        let has_phi = merge_block
            .instructions
            .iter()
            .any(|i| i.operation == LIROperation::Phi);
        assert!(has_phi, "Merge block should contain a phi instruction");
    }
}
