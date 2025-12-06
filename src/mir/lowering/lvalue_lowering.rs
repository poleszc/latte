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
                field_index: field_index,
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
        // TODO: Map for faster lookup?
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
