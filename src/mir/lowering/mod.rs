use std::collections::HashMap;

use crate::{
    SourcePosition,
    mir::{builders::*, *},
    parser::ast::{BinaryOp, Type, TypeKind},
    semantics::{
        symbol_table::SymbolTable,
        tast::{TClassEntryKind, TProgram, TTopDefKind},
    },
    utils::StrID,
};

mod declaration_lowering;
mod expression_lowering;
mod lvalue_lowering;
mod statement_lowering;

pub struct MIRLoweringContext {
    symbol_table: SymbolTable,
    variable_map: HashMap<StrID, LocalID>,
    current_function: Option<MIRFunctionBuilder>,
    current_class_fields: HashMap<StrID, (usize, Type)>,
}

// ======================
// Basic functionality.
// ======================
impl MIRLoweringContext {
    pub fn new(symbol_table: SymbolTable) -> Self {
        MIRLoweringContext {
            symbol_table,
            variable_map: HashMap::new(),
            current_function: None,
            current_class_fields: HashMap::new(),
        }
    }

    /// Main function performing the lowering for the whole program.
    pub fn lower_program(&mut self, program: &TProgram) -> MIRProgram {
        let mut builder = MIRProgramBuilder::new();

        // Lower all classes to MIR.
        for top_def in &program.defs {
            if let TTopDefKind::Class(class_def) = &top_def.kind {
                let mir_class = self.lower_class(class_def);
                builder.add_class(mir_class);
            }
        }

        // Lower all functions to MIR.
        for top_def in &program.defs {
            if let TTopDefKind::Func(func_def) = &top_def.kind {
                let mir_function = self.lower_function(func_def);
                builder.add_function(mir_function);
            }
        }

        // Lower all methods to MIR.
        for top_def in &program.defs {
            if let TTopDefKind::Class(class_def) = &top_def.kind {
                // Insert all class fields in the scope to have access to them in methods.
                let class_fields = self
                    .symbol_table
                    .get_class_field_layout(class_def.name.name);
                for (index, (name, typ, _)) in class_fields.iter().enumerate() {
                    self.current_class_fields
                        .insert(*name, (index, typ.clone()));
                }

                for entry in &class_def.entries {
                    if let TClassEntryKind::Method(method) = &entry.kind {
                        let mir_function = self.lower_method(class_def, method);
                        builder.add_function(mir_function);
                    }
                }
                self.current_class_fields.clear();
            }
        }
        builder.build()
    }
}

// ======================
// Helpers.
// ======================
impl MIRLoweringContext {
    /// Get a mutable reference to the current function builder.
    /// Panic if not set, since this should never happen.
    fn builder(&mut self) -> &mut MIRFunctionBuilder {
        self.current_function
            .as_mut()
            .expect("Function builder not set")
    }

    /// Variable name to LocalId.
    fn resolve_variable(&self, name: StrID) -> Option<LocalID> {
        self.variable_map.get(&name).copied()
    }

    /// Field name to (offset, type)
    fn resolve_field(&self, name: StrID) -> Option<(usize, Type)> {
        self.current_class_fields.get(&name).cloned()
    }

    /// Get 'this' local variable.
    /// Panic if not found, since those errors should be checked by the type checker.
    fn get_this_local(&self) -> LocalID {
        self.variable_map
            .get(&StrID::from("this"))
            .cloned()
            .expect("'this' not found")
    }

    fn mangle_method_name(&self, class_name: StrID, method_name: StrID) -> StrID {
        StrID::from(format!("{}_{}", class_name, method_name))
    }

    fn is_reference_counted_type(&self, typ: &Type) -> bool {
        matches!(typ.kind, TypeKind::String)
    }

    fn emit_drops(&mut self, locals: &[LocalID], pos: SourcePosition) {
        for local_id in locals.iter().rev() {
            let info = self.builder().get_local_info(*local_id).clone();
            if self.is_reference_counted_type(&info.typ) {
                self.builder().emit_drop(*local_id, pos);
            }
        }
    }

    fn binary_op_to_mir(&self, op: &BinaryOp, lhs_type: &Type) -> MIROperation {
        match op {
            BinaryOp::Add => {
                if matches!(lhs_type.kind, TypeKind::String) {
                    MIROperation::StringConcat
                } else {
                    MIROperation::IntAdd
                }
            }
            BinaryOp::Sub => MIROperation::IntSub,
            BinaryOp::Mul => MIROperation::IntMul,
            BinaryOp::Div => MIROperation::IntDiv,
            BinaryOp::Mod => MIROperation::IntMod,
            BinaryOp::Eq => {
                if matches!(lhs_type.kind, TypeKind::String) {
                    MIROperation::StringEq
                } else {
                    MIROperation::IntEq
                }
            }
            BinaryOp::Neq => {
                if matches!(lhs_type.kind, TypeKind::String) {
                    MIROperation::StringNeq
                } else {
                    MIROperation::IntNeq
                }
            }
            BinaryOp::Lt => MIROperation::IntLt,
            BinaryOp::Le => MIROperation::IntLe,
            BinaryOp::Gt => MIROperation::IntGt,
            BinaryOp::Ge => MIROperation::IntGe,
            _ => unreachable!("Logic ops should be handled separately"),
        }
    }

    fn binary_op_result_type(
        &self,
        op: &BinaryOp,
        lhs_type: &Type,
        position: SourcePosition,
    ) -> Type {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                lhs_type.clone()
            }
            _ => Type {
                kind: TypeKind::Bool,
                position,
            },
        }
    }
}
