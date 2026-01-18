use super::MIRLoweringContext;
use crate::{
    mir::*,
    parser::ast::{Ident, TypeKind},
    semantics::tast::{TExpr, TExprKind},
};

// ======================
// LValue lowering.
// ======================
impl MIRLoweringContext {
    /// Lower expression and return it's place.
    pub(super) fn lower_expr_as_place(&mut self, expr: &TExpr) -> MIRPlace {
        match &expr.kind {
            TExprKind::Var(ident) => self.lower_variable_as_place(ident),
            TExprKind::ArrayAccess { array, index } => {
                self.lower_array_access_as_place(array, index)
            }
            TExprKind::FieldAccess { object, field_name } => {
                self.lower_field_access_as_place(object, *field_name)
            }
            // I assume a parenthesis expression is an lvalue. This means code like this: '(object).field' is considered valid.
            TExprKind::Paren(inner) => self.lower_expr_as_place(inner),
            TExprKind::This => MIRPlace::Local(self.get_this_local()),
            _ => panic!("Expression is not an lvalue. This should be caught by semantic checker"),
        }
    }

    /// Lower a "variable" and returns it's place.
    /// Also handles class members accessed directly, not through "this".
    fn lower_variable_as_place(&mut self, ident: &Ident) -> MIRPlace {
        // First look for the identifier in the local variables.
        if let Some(local_id) = self.resolve_variable(ident.name) {
            return MIRPlace::Local(local_id);
        }

        // If not found look in the current class fields.
        if let Some((field_index, _)) = self.resolve_field(ident.name) {
            // Retrieve 'this' from locals. It should always exist when lowering methods.
            let this_local_id = self.get_this_local();
            let this_place = MIRPlace::Local(this_local_id);

            // Now find the field place.
            let this_info = self.builder().get_local_info(this_local_id);
            let class_name = if let TypeKind::Class(class_ident) = &this_info.typ.kind {
                class_ident.name
            } else {
                panic!("'this' is not a class type")
            };

            return MIRPlace::Field {
                base: Box::new(this_place),
                class_name,
                field_name: ident.name,
                field_index,
            };
        }
        panic!(
            "Variable {} not found in locals and class fields",
            ident.name
        );
    }

    /// Lower an array access expression and return it's place.
    fn lower_array_access_as_place(&mut self, array: &TExpr, index: &TExpr) -> MIRPlace {
        let array_val = self.lower_expr(array);
        let array_place = match array_val {
            MIRValue::Local(local_id) => MIRPlace::Local(local_id),
            MIRValue::Temp(temp_id) => MIRPlace::Temp(temp_id),
            _ => panic!("Array base must be a Local or Temp variable"),
        };
        let index_val = self.lower_expr(index);

        MIRPlace::Index {
            base: Box::new(array_place),
            index: Box::new(index_val),
        }
    }

    /// Lower an field access expression and return it's place.
    fn lower_field_access_as_place(&mut self, object: &TExpr, field_name: Ident) -> MIRPlace {
        let object_val = self.lower_expr(object);
        let object_place = match object_val {
            MIRValue::Local(local_id) => MIRPlace::Local(local_id),
            MIRValue::Temp(temp_id) => MIRPlace::Temp(temp_id),
            _ => panic!("Object base must be a Local or Temp variable"),
        };

        let class_name = if let TypeKind::Class(ident) = &object.typ.kind {
            ident.name
        } else {
            panic!("Field access on non-class type");
        };
        let field_layout = self.symbol_table.get_class_field_layout(class_name);
        let (field_index, _) = field_layout
            .iter()
            .enumerate()
            .find(|(_, (name, _, _))| *name == field_name.name)
            .unwrap();

        MIRPlace::Field {
            base: Box::new(object_place),
            class_name,
            field_name: field_name.name,
            field_index,
        }
    }
}
#[cfg(test)]
mod lvalue_tests {
    use super::*;
    use crate::mir::builders::MIRFunctionBuilder;
    use crate::parser::ast::*;
    use crate::semantics::symbol_table::SymbolTable;
    use crate::semantics::tast::*;
    use crate::utils::StrID;

    fn setup_context() -> MIRLoweringContext {
        let st = SymbolTable::new();
        MIRLoweringContext::new(st, false)
    }

    #[test]
    fn test_lower_local_variable_as_place() {
        let mut ctx = setup_context();
        let ty_int = Type {
            kind: TypeKind::Int,
            position: SourcePosition(0, 0),
        };

        let mut builder = MIRFunctionBuilder::new(
            StrID::from("f"),
            ty_int.clone(),
            vec![],
            SourcePosition(0, 0),
        );
        let local_id = builder.create_local(Some(StrID::from("x")), ty_int.clone(), false, None);
        ctx.current_function = Some(builder);
        ctx.variable_map.insert(StrID::from("x"), local_id);

        let var_expr = TExpr {
            kind: TExprKind::Var(Ident {
                name: StrID::from("x"),
                position: SourcePosition(0, 0),
            }),
            typ: ty_int,
            position: SourcePosition(0, 0),
        };

        let place = ctx.lower_expr_as_place(&var_expr);

        match place {
            MIRPlace::Local(id) => assert_eq!(id, local_id),
            _ => panic!("Expected MIRPlace::Local"),
        }
    }

    #[test]
    fn test_lower_implicit_field_access_as_place() {
        let mut ctx = setup_context();
        let ty_int = Type {
            kind: TypeKind::Int,
            position: SourcePosition(0, 0),
        };
        let class_ident = Ident {
            name: StrID::from("klaz"),
            position: SourcePosition(0, 0),
        };
        let ty_class = Type {
            kind: TypeKind::Class(class_ident),
            position: SourcePosition(0, 0),
        };

        let mut builder = MIRFunctionBuilder::new(
            StrID::from("method"),
            ty_int.clone(),
            vec![],
            SourcePosition(0, 0),
        );
        let this_id =
            builder.create_local(Some(StrID::from("this")), ty_class.clone(), true, Some(0));
        ctx.current_function = Some(builder);
        ctx.variable_map.insert(StrID::from("this"), this_id);

        ctx.current_class_fields
            .insert(StrID::from("metod"), (0, ty_int.clone()));

        let field_expr = TExpr {
            kind: TExprKind::Var(Ident {
                name: StrID::from("metod"),
                position: SourcePosition(0, 0),
            }),
            typ: ty_int,
            position: SourcePosition(0, 0),
        };

        let place = ctx.lower_expr_as_place(&field_expr);

        match place {
            MIRPlace::Field {
                base,
                field_name,
                field_index,
                ..
            } => {
                assert_eq!(field_name, StrID::from("metod"));
                assert_eq!(field_index, 0);
                match *base {
                    MIRPlace::Local(id) => assert_eq!(id, this_id),
                    _ => panic!("Base of implicit field access must be 'this' (Local)"),
                }
            }
            _ => panic!("Expected MIRPlace::Field for implicit field access"),
        }
    }

    #[test]
    fn test_lower_array_access_as_place() {
        let mut ctx = setup_context();
        let ty_int = Type {
            kind: TypeKind::Int,
            position: SourcePosition(0, 0),
        };
        let ty_arr = Type {
            kind: TypeKind::Array(Box::new(ty_int.clone())),
            position: SourcePosition(0, 0),
        };

        let mut builder = MIRFunctionBuilder::new(
            StrID::from("f"),
            ty_int.clone(),
            vec![],
            SourcePosition(0, 0),
        );
        let arr_id = builder.create_local(Some(StrID::from("arr")), ty_arr.clone(), false, None);
        ctx.current_function = Some(builder);
        ctx.variable_map.insert(StrID::from("arr"), arr_id);

        let arr_expr = Box::new(TExpr {
            kind: TExprKind::Var(Ident {
                name: StrID::from("arr"),
                position: SourcePosition(0, 0),
            }),
            typ: ty_arr,
            position: SourcePosition(0, 0),
        });
        let index_expr = Box::new(TExpr {
            kind: TExprKind::LiteralInt(5),
            typ: ty_int.clone(),
            position: SourcePosition(0, 0),
        });

        let array_access_expr = TExpr {
            kind: TExprKind::ArrayAccess {
                array: arr_expr,
                index: index_expr,
            },
            typ: ty_int,
            position: SourcePosition(0, 0),
        };

        let place = ctx.lower_expr_as_place(&array_access_expr);

        match place {
            MIRPlace::Index { base, index } => {
                match *base {
                    MIRPlace::Local(id) => assert_eq!(id, arr_id),
                    _ => panic!("Expected Local base for array"),
                }
                match *index {
                    MIRValue::IntConst(v) => assert_eq!(v, 5),
                    _ => panic!("Expected IntConst index"),
                }
            }
            _ => panic!("Expected MIRPlace::Index"),
        }
    }
}
