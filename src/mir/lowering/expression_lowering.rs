use super::MIRLoweringContext;
use crate::{
    SourcePosition,
    mir::*,
    parser::ast::{BinaryOp, Ident, Type, TypeKind, UnaryOp},
    semantics::tast::{TExpr, TExprKind},
    utils::StrID,
};

// ======================
// Expression lowering.
// ======================
impl MIRLoweringContext {
    pub(super) fn lower_expr(&mut self, expr: &TExpr) -> MIRValue {
        match &expr.kind {
            TExprKind::LiteralInt(val) => MIRValue::IntConst(*val),
            TExprKind::LiteralTrue => MIRValue::BoolConst(true),
            TExprKind::LiteralFalse => MIRValue::BoolConst(false),
            TExprKind::This => {
                // This should be inserted into the variable map if we analyze a method.
                let local_id = self.variable_map.get(&StrID::from("this")).unwrap();
                MIRValue::Local(*local_id)
            }
            TExprKind::LiteralString(str) => MIRValue::StringConst(str.clone()),
            TExprKind::LiteralNull | TExprKind::TypedNull(_) => MIRValue::Null,
            TExprKind::Var(_) => {
                let place = self.lower_expr_as_place(expr);
                self.lower_place_as_value(&place, &expr.typ)
            }
            TExprKind::UnaryOp { op, expr } => self.lower_unary_op(op, expr),
            TExprKind::BinaryOp { op, lhs, rhs } => self.lower_binary_op(op, lhs, rhs),
            TExprKind::App { callee, args } => self.lower_call(callee, args, expr.position),
            TExprKind::NewObject(typ) => self.lower_new_object(typ, expr.position),
            TExprKind::NewArray { element_type, size } => {
                self.lower_new_array(element_type, size, expr.position)
            }
            TExprKind::ArrayAccess { .. } => {
                let place = self.lower_expr_as_place(expr);
                self.lower_place_as_value(&place, &expr.typ)
            }
            TExprKind::FieldAccess { object, field_name } => {
                self.lower_field_access(object, *field_name)
            }
            TExprKind::Method {
                object,
                method_name,
                args,
            } => self.lower_method_call(object, method_name.name, args, expr.position, &expr.typ),
            TExprKind::Paren(expr) => self.lower_expr(expr),
        }
    }

    pub(super) fn lower_place_as_value(&mut self, place: &MIRPlace, typ: &Type) -> MIRValue {
        match place {
            MIRPlace::Local(id) => MIRValue::Local(*id),
            MIRPlace::Temp(id) => MIRValue::Temp(*id),
            MIRPlace::Field {
                base,
                field_index,
                class_name,
                ..
            } => {
                let base_type = Type {
                    kind: TypeKind::Class(Ident {
                        name: *class_name,
                        position: typ.position,
                    }),
                    position: typ.position,
                };

                let obj_value = self.lower_place_as_value(base, &base_type);
                let temp_dest = self.builder().create_temp(typ.clone());
                self.builder().emit_field_get(
                    MIRPlace::Temp(temp_dest),
                    obj_value,
                    *field_index,
                    typ.position,
                );
                MIRValue::Temp(temp_dest)
            }
            MIRPlace::Index { base, index } => {
                let array_value = self.lower_place_as_value(base, typ);
                let temp_dest = self.builder().create_temp(typ.clone());
                self.builder().emit_array_get(
                    MIRPlace::Temp(temp_dest),
                    array_value,
                    *index.clone(),
                    typ.position,
                );
                MIRValue::Temp(temp_dest)
            }
        }
    }

    fn lower_string_length(&mut self, string_expr: &TExpr) -> MIRValue {
        let string_val = self.lower_expr(string_expr);
        let temp = self.builder().create_temp(Type {
            kind: TypeKind::Int,
            position: string_expr.position,
        });
        self.builder()
            .emit_string_length(MIRPlace::Temp(temp), string_val, string_expr.position);
        MIRValue::Temp(temp)
    }

    fn lower_array_length(&mut self, array_expr: &TExpr) -> MIRValue {
        let array_val = self.lower_expr(array_expr);
        let temp = self.builder().create_temp(Type {
            kind: TypeKind::Int,
            position: array_expr.position,
        });
        self.builder()
            .emit_array_length(MIRPlace::Temp(temp), array_val, array_expr.position);
        MIRValue::Temp(temp)
    }

    fn lower_field_access(&mut self, base_expr: &TExpr, field_name: Ident) -> MIRValue {
        match &base_expr.typ.kind {
            TypeKind::String => {
                if field_name.name == StrID::from("length") {
                    self.lower_string_length(base_expr)
                } else {
                    panic!("String field access got invalid field: {}", field_name.name);
                }
            }
            TypeKind::Array(_) => {
                if field_name.name == StrID::from("length") {
                    self.lower_array_length(base_expr)
                } else {
                    panic!("Array field access got invalid field: {}", field_name.name);
                }
            }
            TypeKind::Class(_) => {
                let object_place = self.lower_expr_as_place(base_expr);
                let class_name = if let TypeKind::Class(ident) = &base_expr.typ.kind {
                    ident.name
                } else {
                    panic!(
                        "Field access on non-class type. This should be caught by the semantic checker"
                    );
                };

                let field_layout = self.symbol_table.get_class_field_layout(class_name);
                let (field_index, (field_name, typ, _)) = field_layout
                    .iter()
                    .enumerate()
                    .find(|(_, (name, _, _))| *name == field_name.name).expect("Field name not found during lowering. This should be caught by the semantic checker.");

                let field_place = MIRPlace::Field {
                    base: Box::new(object_place),
                    class_name,
                    field_name: *field_name,
                    field_index,
                };

                self.lower_place_as_value(&field_place, typ)
            }
            _ => {
                panic!("Field access got invalid type: {}", base_expr.typ);
            }
        }
    }

    fn lower_logic_op(&mut self, op: &BinaryOp, lhs: &TExpr, rhs: &TExpr) -> MIRValue {
        let res_temp = self.builder().create_temp(Type {
            kind: TypeKind::Bool,
            position: lhs.position,
        });

        // Lower the left side.
        let lhs_val = self.lower_expr(lhs);

        let eval_rhs_block = self.builder().create_block();
        let end_block = self.builder().create_block();

        match op {
            BinaryOp::And => {
                // Emit the default value for short-circuit AND.
                self.builder().emit_assign(
                    MIRPlace::Temp(res_temp),
                    MIRValue::BoolConst(false),
                    lhs.position,
                );

                // If expression is true, we move to the next block. If false we skip the rhs.
                self.builder()
                    .emit_branch(lhs_val, eval_rhs_block, end_block, lhs.position);
            }
            BinaryOp::Or => {
                // Emit the default value for short-circuit OR.
                self.builder().emit_assign(
                    MIRPlace::Temp(res_temp),
                    MIRValue::BoolConst(true),
                    lhs.position,
                );
                // If expression is true, we move to the end block. If false we move to the rhs.
                self.builder()
                    .emit_branch(lhs_val, end_block, eval_rhs_block, rhs.position);
            }
            _ => unreachable!("Non logic op in lower logic op"),
        }

        // Now we evaluate the right side.
        self.builder().set_current_block(eval_rhs_block);
        let rhs_val = self.lower_expr(rhs);
        self.builder()
            .emit_assign(MIRPlace::Temp(res_temp), rhs_val, rhs.position);
        self.builder().emit_jump(end_block, rhs.position);
        self.builder().set_current_block(end_block);
        MIRValue::Temp(res_temp)
    }

    fn lower_binary_op(&mut self, op: &BinaryOp, lhs: &TExpr, rhs: &TExpr) -> MIRValue {
        // If a logic op is encountered we have to lower it lazily.
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            return self.lower_logic_op(op, lhs, rhs);
        }

        // Otherwise it's simple.
        let lhs_val = self.lower_expr(lhs);
        let rhs_val = self.lower_expr(rhs);
        let mir_op = self.binary_op_to_mir(op, &lhs.typ);
        let result_type = self.binary_op_result_type(op, &lhs.typ, lhs.position);

        let temp = self.builder().create_temp(result_type);
        self.builder()
            .emit_binary(mir_op, MIRPlace::Temp(temp), lhs_val, rhs_val, lhs.position);
        MIRValue::Temp(temp)
    }

    fn lower_unary_op(&mut self, op: &UnaryOp, expr: &TExpr) -> MIRValue {
        let value = self.lower_expr(expr);
        let mir_op = match op {
            UnaryOp::Neg => MIROperation::IntNeg,
            UnaryOp::Not => MIROperation::BoolNot,
        };

        let temp = self.builder().create_temp(expr.typ.clone());
        self.builder()
            .emit_unary(mir_op, MIRPlace::Temp(temp), value, expr.position);

        MIRValue::Temp(temp)
    }

    fn lower_call(&mut self, callee: &TExpr, args: &[TExpr], position: SourcePosition) -> MIRValue {
        let func_ident = match &callee.kind {
            TExprKind::Var(ident) => ident,
            _ => panic!("MIR Lowering: only direct function calls are supported"),
        };

        // Lower arguments.
        let arg_values: Vec<MIRValue> = args.iter().map(|arg| self.lower_expr(arg)).collect();

        if !matches!(callee.typ.kind, TypeKind::Fun(..)) {
            panic!("Callee is not a function type");
        }
        let return_type = if let TypeKind::Fun(ret, _) = &callee.typ.kind {
            (*ret).clone()
        } else {
            unreachable!()
        };

        if !matches!(return_type.kind, TypeKind::Void) {
            let temp = self.builder().create_temp(*return_type);
            self.builder().emit_call(
                Some(MIRPlace::Temp(temp)),
                func_ident.name,
                arg_values,
                position,
            );
            MIRValue::Temp(temp)
        } else {
            self.builder()
                .emit_call(None, func_ident.name, arg_values, position);
            MIRValue::Unit
        }
    }

    fn lower_method_call(
        &mut self,
        object: &TExpr,
        method_name: StrID,
        args: &[TExpr],
        position: SourcePosition,
        return_type: &Type,
    ) -> MIRValue {
        let object_val = self.lower_expr(object);
        let arg_values: Vec<MIRValue> = args.iter().map(|arg| self.lower_expr(arg)).collect();

        let class_name = if let TypeKind::Class(ident) = &object.typ.kind {
            ident.name
        } else {
            panic!("Method call on non-class type");
        };

        let vtable = self.symbol_table.get_class_vtable_layout(class_name);
        let (vtable_index, _) = vtable
            .iter()
            .enumerate()
            .find(|(_, func_info)| func_info.ident.name == method_name)
            .expect("Method not found in vtable");

        if !matches!(return_type.kind, TypeKind::Void) {
            let temp = self.builder().create_temp(return_type.clone());
            self.builder().emit_virtual_call(
                Some(MIRPlace::Temp(temp)),
                object_val,
                vtable_index,
                arg_values,
                position,
            );
            MIRValue::Temp(temp)
        } else {
            self.builder()
                .emit_virtual_call(None, object_val, vtable_index, arg_values, position);
            MIRValue::Unit
        }
    }

    fn lower_new_object(&mut self, typ: &Type, position: SourcePosition) -> MIRValue {
        let class_name = match &typ.kind {
            TypeKind::Class(ident) => ident.name,
            _ => panic!("New object requires class type"),
        };

        let temp = self.builder().create_temp(typ.clone());
        self.builder()
            .emit_new_object(MIRPlace::Temp(temp), class_name, position);
        MIRValue::Temp(temp)
    }

    fn lower_new_array(
        &mut self,
        element_type: &Type,
        size: &TExpr,
        position: SourcePosition,
    ) -> MIRValue {
        let size_val = self.lower_expr(size);

        let array_type = Type {
            kind: TypeKind::Array(Box::new(element_type.clone())),
            position,
        };

        let temp = self.builder().create_temp(array_type);
        self.builder()
            .emit_new_array(MIRPlace::Temp(temp), size_val, position);

        MIRValue::Temp(temp)
    }
}

#[cfg(test)]
mod expr_logic_tests {
    use super::*;
    use crate::mir::builders::MIRFunctionBuilder;
    use crate::parser::ast::{BinaryOp, SourcePosition, Type, TypeKind};
    use crate::semantics::symbol_table::SymbolTable;
    use crate::semantics::tast::*;

    #[test]
    fn test_short_circuit() {
        // a && b
        let mut ctx = MIRLoweringContext::new(SymbolTable::new());
        let ty_bool = Type {
            kind: TypeKind::Bool,
            position: SourcePosition(0, 0),
        };

        ctx.current_function = Some(MIRFunctionBuilder::new(
            StrID::from("f"),
            ty_bool.clone(),
            vec![],
            SourcePosition(0, 0),
        ));

        let expr_a = TExpr {
            kind: TExprKind::LiteralTrue,
            typ: ty_bool.clone(),
            position: SourcePosition(0, 0),
        };
        let expr_b = TExpr {
            kind: TExprKind::LiteralFalse,
            typ: ty_bool.clone(),
            position: SourcePosition(0, 0),
        };

        ctx.lower_logic_op(&BinaryOp::And, &expr_a, &expr_b);

        let mir_func = ctx.current_function.unwrap().build();
        assert!(
            mir_func.blocks.len() >= 3,
            "Simple boolean op should short circuit and create 3 blocks"
        );

        let entry_block = &mir_func.blocks[&mir_func.entry_block];
        assert!(
            matches!(entry_block.terminator, Some(MIRTerminator::Branch { .. })),
            "First block should end with a branch"
        );
    }
}
