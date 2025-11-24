use crate::errors::SemanticError;
use crate::parser::ast::{Ident, SourcePosition, Type, TypeKind};
use crate::semantics::symbol_table::SymbolTable;
use crate::utils::StrID;

/// Type checker responsible for type comparison and validation logic.
pub struct TypeChecker<'a> {
    symbol_table: &'a SymbolTable,
}

impl<'a> TypeChecker<'a> {
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        TypeChecker { symbol_table }
    }

    /// Check if two types are equal.
    /// Error types are considered equal to everything to prevent cascading errors
    /// when performing error recovery.
    pub fn types_equal(t1: &Type, t2: &Type) -> bool {
        match (&t1.kind, &t2.kind) {
            (TypeKind::Error, _) | (_, TypeKind::Error) => true,
            (TypeKind::Int, TypeKind::Int) => true,
            (TypeKind::String, TypeKind::String) => true,
            (TypeKind::Bool, TypeKind::Bool) => true,
            (TypeKind::Void, TypeKind::Void) => true,
            (TypeKind::Class(i1), TypeKind::Class(i2)) => i1.name == i2.name,
            (TypeKind::Array(inner1), TypeKind::Array(inner2)) => Self::types_equal(inner1, inner2),
            _ => false,
        }
    }

    /// Check if `subtype` is a subtype of `supertype`.
    /// This includes:
    /// - Same types
    /// - Subclasses in inheritance hierarchy
    /// - Null as subtype of all classes and arrays. TODO: Probably change that and expect explicitly types nulls.
    pub fn is_subtype(&self, subtype: &Type, supertype: &Type) -> bool {
        // Ignore error types to prevent cascading errors
        if matches!(subtype.kind, TypeKind::Error) || matches!(supertype.kind, TypeKind::Error) {
            return true;
        }

        if matches!(subtype.kind, TypeKind::Null) {
            return matches!(supertype.kind, TypeKind::Class(_) | TypeKind::Array(_));
        }

        if TypeChecker::types_equal(subtype, supertype) {
            return true;
        }

        if let (TypeKind::Class(sub_class), TypeKind::Class(super_class)) =
            (&subtype.kind, &supertype.kind)
        {
            return self.is_class_subtype(sub_class.name, super_class.name);
        }

        if let (TypeKind::Array(sub_inner), TypeKind::Array(super_inner)) =
            (&subtype.kind, &supertype.kind)
        {
            return self.is_subtype(sub_inner, super_inner);
        }

        false
    }

    /// Check if `subclass` is a subclass of `superclass` in the inheritance hierarchy.
    fn is_class_subtype(&self, subclass: StrID, superclass: StrID) -> bool {
        if subclass == superclass {
            return true;
        }

        let mut current = subclass;
        loop {
            // TODO: Remove temporary ident creation? Why is that even here xd
            let current_ident = Ident {
                name: current,
                position: SourcePosition(0, 0),
            };

            if let Ok(class_info) = self.symbol_table.lookup_class(&current_ident) {
                if let Some(parent) = &class_info.parent {
                    if parent.name == superclass {
                        return true;
                    }
                    current = parent.name;
                } else {
                    return false;
                }
            } else {
                // Should be caught earlier, but safely return false
                return false;
            }
        }
    }

    /// Check if a type is legal for a binary operation.
    pub fn type_legal_for_bin_op(&self, op: &crate::parser::ast::BinaryOp, typ: &Type) -> bool {
        use crate::parser::ast::BinaryOp;

        match op {
            BinaryOp::Add => matches!(typ.kind, TypeKind::Int | TypeKind::String),
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                matches!(typ.kind, TypeKind::Int)
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                matches!(typ.kind, TypeKind::Int)
            }
            BinaryOp::Eq | BinaryOp::Neq => true,
            BinaryOp::And | BinaryOp::Or => {
                matches!(typ.kind, TypeKind::Bool)
            }
        }
    }

    /// Check if a type is legal for a unary operation.
    /// Returns the expected type for the operand, or None if the operation is not valid.
    pub fn type_legal_for_unary_op(&self, op: &crate::parser::ast::UnaryOp, typ: &Type) -> bool {
        use crate::parser::ast::UnaryOp;

        match op {
            UnaryOp::Neg => matches!(typ.kind, TypeKind::Int),
            UnaryOp::Not => matches!(typ.kind, TypeKind::Bool),
        }
    }

    /// Get the result type of a unary operation.
    /// Assumes the operand type is valid for the operation.
    pub fn unary_op_result_type(
        &self,
        op: &crate::parser::ast::UnaryOp,
        position: SourcePosition,
    ) -> Type {
        use crate::parser::ast::UnaryOp;

        match op {
            UnaryOp::Neg => Type {
                kind: TypeKind::Int,
                position,
            },
            UnaryOp::Not => Type {
                kind: TypeKind::Bool,
                position,
            },
        }
    }

    /// Get the result type of a binary operation.
    /// Assumes operand types are valid for the operation.
    pub fn binary_op_result_type(
        &self,
        op: &crate::parser::ast::BinaryOp,
        lhs_type: &Type,
        position: SourcePosition,
    ) -> Type {
        use crate::parser::ast::BinaryOp;

        match op {
            // Result type is same as operand type
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => Type {
                kind: lhs_type.kind.clone(),
                position,
            },
            // Result type is always bool
            BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Eq
            | BinaryOp::Neq
            | BinaryOp::And
            | BinaryOp::Or => Type {
                kind: TypeKind::Bool,
                position,
            },
        }
    }

    /// Validate a type used for variable or field declaration.
    /// Void and Null types are not allowed for declarations.
    pub fn validate_variable_type(&self, typ: &Type) -> Result<(), SemanticError> {
        let expected_types = vec![
            StrID::from("int"),
            StrID::from("bool"),
            StrID::from("string"),
            StrID::from("{ClassType}"),
            StrID::from("{ArrayType}"),
        ];

        match &typ.kind {
            TypeKind::Void | TypeKind::Null => {
                return Err(SemanticError::InvalidVariableType {
                    found: StrID::from(typ.to_string()),
                    expected: expected_types,
                    position: typ.position,
                });
            }
            TypeKind::Class(ident) => {
                if !self.symbol_table.class_exists(ident.name) {
                    return Err(SemanticError::UndefinedClass {
                        name: ident.name,
                        position: ident.position,
                    });
                }
            }
            TypeKind::Array(inner) => {
                if let TypeKind::Void = inner.kind {
                    return Err(SemanticError::InvalidVoidArrayType {
                        expected: expected_types,
                        position: inner.position,
                    });
                } else {
                    // Validate inner type.
                    self.validate_variable_type(inner)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate a type used for function return type declaration.
    /// Void is allowed, but Null is not.
    pub fn validate_function_return_type(&self, typ: &Type) -> Result<(), SemanticError> {
        let expected_types = vec![
            StrID::from("void"),
            StrID::from("int"),
            StrID::from("bool"),
            StrID::from("string"),
            StrID::from("{ClassType}"),
            StrID::from("{ArrayType}"),
        ];

        match &typ.kind {
            TypeKind::Null => {
                return Err(SemanticError::InvalidVariableType {
                    found: StrID::from(typ.to_string()),
                    expected: expected_types,
                    position: typ.position,
                });
            }
            TypeKind::Class(ident) => {
                if !self.symbol_table.class_exists(ident.name) {
                    return Err(SemanticError::UndefinedClass {
                        name: ident.name,
                        position: ident.position,
                    });
                }
            }
            TypeKind::Array(inner) => {
                if let TypeKind::Void = inner.kind {
                    return Err(SemanticError::InvalidVoidArrayType {
                        expected: expected_types,
                        position: inner.position,
                    });
                } else {
                    // Validate inner type.
                    self.validate_variable_type(inner)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Create an error type at the given position.
    pub fn error_type(&self, pos: SourcePosition) -> Type {
        Type {
            kind: TypeKind::Error,
            position: pos,
        }
    }
}
