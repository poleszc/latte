use crate::errors::SemanticError;
use crate::parser::ast::TypeKind;
use crate::parser::ast::*;
use crate::semantics::analysis::context::SemanticContext;
use crate::semantics::{FunctionInfo, tast};
use crate::utils::StrID;

pub struct ExpressionChecker<'a> {
    ctx: &'a mut SemanticContext,
}

// ======================
// Basic functionality.
// ======================
impl<'a> ExpressionChecker<'a> {
    pub fn new(ctx: &'a mut SemanticContext) -> Self {
        Self { ctx }
    }

    /// Perform a type check of an expression and return it's type.
    /// If expression if typed incorrectly we return an Error type
    /// to allow for error recovery.
    pub fn check(&mut self, expr: &Expr) -> tast::TExpr {
        self.visit_expr(expr)
    }
}

// ======================
// Utilities.
// ======================
impl<'a> ExpressionChecker<'a> {
    /// Validate that operands are valid for the binary operation.
    fn validate_binary_operands(
        &mut self,
        op: &BinaryOp,
        lhs_type: &Type,
        rhs_type: &Type,
        lhs_position: SourcePosition,
        rhs_position: SourcePosition,
    ) {
        // Check if left operand is valid for this operation.
        if !self.ctx.type_legal_for_bin_op(op, lhs_type) {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from(op.to_string()),
                operand: StrID::from(lhs_type.to_string()),
                position: lhs_position,
                type_position: lhs_type.position,
            });
        }

        // Check if right operand is valid for this operation.
        if !self.ctx.type_legal_for_bin_op(op, rhs_type) {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from(op.to_string()),
                operand: StrID::from(rhs_type.to_string()),
                position: rhs_position,
                type_position: rhs_type.position,
            });
        }

        // Check type compatibility between operands.
        self.validate_operand_compatibility(op, lhs_type, rhs_type);
    }

    /// Check that the two operands are compatible for the binary operation.
    fn validate_operand_compatibility(&mut self, op: &BinaryOp, lhs_type: &Type, rhs_type: &Type) {
        match op {
            // Equality operators allow subtype relationships.
            BinaryOp::Eq | BinaryOp::Neq => {
                let compatible = self.ctx.is_subtype(lhs_type, rhs_type)
                    || self.ctx.is_subtype(rhs_type, lhs_type);

                if !compatible {
                    self.ctx.add_error(SemanticError::BinaryOpTypeMismatch {
                        lhs_type: StrID::from(lhs_type.to_string()),
                        lhs_position: lhs_type.position,
                        rhs_type: StrID::from(rhs_type.to_string()),
                        rhs_position: rhs_type.position,
                    });
                }
            }
            // All other operators require exact type match.
            _ => {
                if !self.ctx.types_equal(lhs_type, rhs_type) {
                    self.ctx.add_error(SemanticError::BinaryOpTypeMismatch {
                        lhs_type: StrID::from(lhs_type.to_string()),
                        lhs_position: lhs_type.position,
                        rhs_type: StrID::from(rhs_type.to_string()),
                        rhs_position: rhs_type.position,
                    });
                }
            }
        }
    }
}

// ======================
// Expression visitors.
// ======================
impl<'a> ExpressionChecker<'a> {
    fn visit_expr(&mut self, outer_expr: &Expr) -> tast::TExpr {
        let position = outer_expr.position;
        let (kind, typ) = match &outer_expr.kind {
            ExprKind::Var(ident) => {
                let resolved_type = match self.ctx.symbol_table.lookup_variable(ident) {
                    Ok(t) => t,
                    Err(e) => {
                        self.ctx.add_error(e);
                        self.ctx.error_type(ident.position)
                    }
                };
                (tast::TExprKind::Var(*ident), resolved_type)
            }
            ExprKind::LiteralInt(val) => (
                tast::TExprKind::LiteralInt(*val),
                Type {
                    kind: TypeKind::Int,
                    position,
                },
            ),
            ExprKind::LiteralTrue => (
                tast::TExprKind::LiteralTrue,
                Type {
                    kind: TypeKind::Bool,
                    position,
                },
            ),
            ExprKind::LiteralFalse => (
                tast::TExprKind::LiteralFalse,
                Type {
                    kind: TypeKind::Bool,
                    position,
                },
            ),
            ExprKind::LiteralString(s) => (
                tast::TExprKind::LiteralString(s.clone()),
                Type {
                    kind: TypeKind::String,
                    position,
                },
            ),
            ExprKind::LiteralNull => (
                tast::TExprKind::LiteralNull,
                Type {
                    kind: TypeKind::Null,
                    position,
                },
            ),
            ExprKind::App { callee, args } => self.visit_application(callee, args, position),
            ExprKind::ArrayAccess { array, index } => {
                self.visit_array_access(array, index, position)
            }
            ExprKind::FieldAccess { object, field_name } => {
                self.visit_field_access(object, field_name, position)
            }
            ExprKind::Method {
                object,
                method_name,
                args,
            } => self.visit_method_call(object, method_name, args, position),
            ExprKind::Paren(inner) => {
                let typed_inner = self.visit_expr(inner);
                (
                    tast::TExprKind::Paren(Box::new(typed_inner.clone())),
                    typed_inner.typ,
                )
            }
            ExprKind::UnaryOp { op, expr } => self.visit_unary_operator(op, expr, position),
            ExprKind::BinaryOp { op, lhs, rhs } => {
                self.visit_binary_operator(op, lhs, rhs, position)
            }
            ExprKind::NewObject(typ) => {
                self.ctx.validate_variable_type(typ);
                (tast::TExprKind::NewObject(typ.clone()), typ.clone())
            }
            ExprKind::NewArray { element_type, size } => {
                self.ctx.validate_variable_type(element_type);
                let typed_size = self.visit_expr(size);
                if !matches!(typed_size.typ.kind, TypeKind::Int | TypeKind::Error) {
                    self.ctx.add_error(SemanticError::TypeMismatch {
                        expected: StrID::from("int"),
                        found: StrID::from(typed_size.typ.to_string()),
                        position: size.position,
                    });
                }
                let kind = tast::TExprKind::NewArray {
                    element_type: element_type.clone(),
                    size: Box::new(typed_size),
                };
                let typ = Type {
                    kind: TypeKind::Array(Box::new(element_type.clone())),
                    position,
                };
                (kind, typ)
            }
            ExprKind::TypedNull(typ) => {
                self.ctx.validate_variable_type(typ);
                if matches!(typ.kind, TypeKind::Bool | TypeKind::Int | TypeKind::Void) {
                    let expected = vec![
                        StrID::from("{ClassType}"),
                        StrID::from("{ArrayType}"),
                        StrID::from("{string}"),
                    ];
                    self.ctx.add_error(SemanticError::InvalidNullCast {
                        found: StrID::from(typ.to_string()),
                        expected,
                        type_position: typ.position,
                        expr_position: position,
                    });
                }
                (tast::TExprKind::TypedNull(typ.clone()), typ.clone())
            }
            ExprKind::ExprTypedNull(expr) => {
                let expected = vec![
                    StrID::from("{ClassType}"),
                    StrID::from("{ArrayType}"),
                    StrID::from("{string}"),
                ];

                let resolved_type = match &expr.kind {
                    ExprKind::Var(ident) => {
                        if !self.ctx.symbol_table.class_exists(ident.name) {
                            self.ctx.add_error(SemanticError::UndefinedClass {
                                name: ident.name,
                                position: ident.position,
                            });
                            self.ctx.error_type(position)
                        } else {
                            Type {
                                kind: TypeKind::Class(*ident),
                                position: ident.position,
                            }
                        }
                    }
                    _ => {
                        self.ctx.add_error(SemanticError::InvalidNullCastExpr {
                            expected,
                            cast_position: expr.position,
                            expr_position: position,
                        });
                        self.ctx.error_type(position)
                    }
                };

                (
                    tast::TExprKind::TypedNull(resolved_type.clone()),
                    resolved_type,
                )
            }
            ExprKind::This => {
                let typ = if let Some(class_name) = &self.ctx.current_class {
                    let class_ident = self.ctx.mock_identifier(*class_name);
                    Type {
                        kind: TypeKind::Class(class_ident),
                        position,
                    }
                } else {
                    self.ctx
                        .add_error(SemanticError::ThisUsedOutsideClass { position });
                    self.ctx.error_type(position)
                };
                (tast::TExprKind::This, typ)
            }
        };

        tast::TExpr {
            kind,
            position,
            typ,
        }
    }

    fn check_and_transform_arguments(
        &mut self,
        func_info: &FunctionInfo,
        args: &[Expr],
        call_position: SourcePosition,
    ) -> Vec<tast::TExpr> {
        let func_ident = func_info.ident;
        if args.len() != func_info.params.len() {
            self.ctx.add_error(SemanticError::WrongNumberOfArguments {
                name: func_ident.name,
                expected: func_info.params.len(),
                found: args.len(),
                call_position,
                function_position: func_info.position,
            });
        }

        let typed_args = args
            .iter()
            .zip(&func_info.params)
            .map(|(arg_expr, param_type)| {
                let typed_arg = self.visit_expr(arg_expr);
                if !self.ctx.is_subtype(&typed_arg.typ, param_type) {
                    self.ctx.add_error(SemanticError::ArgumentTypeMismatch {
                        expected: StrID::from(param_type.to_string()),
                        found: StrID::from(typed_arg.typ.to_string()),
                        position: typed_arg.position,
                        function_name: func_info.ident.name,
                        argument_position: param_type.position,
                    });
                }
                typed_arg
            })
            .collect();

        // If more arguments are provided, we type check them.
        if args.len() > func_info.params.len() {
            args[func_info.params.len()..].iter().for_each(|arg| {
                self.visit_expr(arg);
            });
        }

        typed_args
    }

    fn visit_application(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        position: SourcePosition,
    ) -> (tast::TExprKind, Type) {
        if let ExprKind::Var(name) = &callee.kind {
            match self.ctx.symbol_table.lookup_function(name) {
                Ok(func_info) => {
                    let typed_args = self.check_and_transform_arguments(&func_info, args, position);
                    let return_type = func_info.return_type.clone();
                    let typed_callee = tast::TExpr {
                        kind: tast::TExprKind::Var(*name),
                        position: callee.position,
                        typ: Type {
                            kind: TypeKind::Fun(Box::new(return_type.clone()), func_info.params),
                            position: callee.position,
                        },
                    };
                    let kind = tast::TExprKind::App {
                        callee: Box::new(typed_callee),
                        args: typed_args,
                    };
                    (kind, return_type)
                }
                Err(e) => {
                    self.ctx.add_error(e);
                    let typed_args = args.iter().map(|arg| self.visit_expr(arg)).collect();
                    let typed_callee = self.visit_expr(callee);
                    let kind = tast::TExprKind::App {
                        callee: Box::new(typed_callee),
                        args: typed_args,
                    };
                    (kind, self.ctx.error_type(position))
                }
            }
        } else {
            self.ctx.add_error(SemanticError::InvalidCallBase {
                position: callee.position,
            });
            let typed_callee = self.visit_expr(callee);
            let typed_args: Vec<tast::TExpr> =
                args.iter().map(|arg| self.visit_expr(arg)).collect();
            let kind = tast::TExprKind::App {
                callee: Box::new(typed_callee),
                args: typed_args,
            };
            (kind, self.ctx.error_type(position))
        }
    }

    fn visit_array_access(
        &mut self,
        array: &Expr,
        index: &Expr,
        position: SourcePosition,
    ) -> (tast::TExprKind, Type) {
        let typed_array = self.visit_expr(array);
        let typed_index = self.visit_expr(index);

        if !matches!(typed_index.typ.kind, TypeKind::Int | TypeKind::Error) {
            self.ctx.add_error(SemanticError::TypeMismatch {
                expected: StrID::from("int"),
                found: StrID::from(typed_index.typ.to_string()),
                position: index.position,
            });
        }

        let kind = tast::TExprKind::ArrayAccess {
            array: Box::new(typed_array.clone()),
            index: Box::new(typed_index),
        };

        let typ = match &typed_array.typ.kind {
            TypeKind::Array(elem_type) => (**elem_type).clone(),
            TypeKind::Error => self.ctx.error_type(position),
            _ => {
                self.ctx.add_error(SemanticError::InvalidOperands {
                    operator: StrID::from("[]"),
                    operand: StrID::from(typed_array.typ.to_string()),
                    position,
                    type_position: typed_array.typ.position,
                });
                self.ctx.error_type(position)
            }
        };
        (kind, typ)
    }

    fn visit_field_access(
        &mut self,
        object: &Expr,
        field_name: &Ident,
        position: SourcePosition,
    ) -> (tast::TExprKind, Type) {
        let typed_object = self.visit_expr(object);

        let kind = tast::TExprKind::FieldAccess {
            object: Box::new(typed_object.clone()),
            field_name: *field_name,
        };

        let typ = if let TypeKind::Class(class_name) = &typed_object.typ.kind {
            let all_fields = self.ctx.symbol_table.get_all_class_fields(class_name.name);
            if let Some((field_type, _)) = all_fields.get(&field_name.name) {
                field_type.clone()
            } else {
                self.ctx.add_error(SemanticError::UndefinedField {
                    class_name: class_name.name,
                    field_name: field_name.name,
                    position,
                    class_position: class_name.position,
                });
                self.ctx.error_type(position)
            }
        } else if let TypeKind::String | TypeKind::Array(_) = &typed_object.typ.kind {
            if field_name.name != StrID::from("length") {
                self.ctx.add_error(SemanticError::UndefinedField {
                    class_name: StrID::from(typed_object.typ.to_string()),
                    field_name: field_name.name,
                    position,
                    class_position: SourcePosition(0, 0),
                });
                self.ctx.error_type(position)
            } else {
                Type {
                    kind: TypeKind::Int,
                    position,
                }
            }
        } else if matches!(typed_object.typ.kind, TypeKind::Error) {
            self.ctx.error_type(position)
        } else {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from("."),
                operand: StrID::from(typed_object.typ.to_string()),
                position,
                type_position: typed_object.typ.position,
            });
            self.ctx.error_type(position)
        };
        (kind, typ)
    }

    fn visit_method_call(
        &mut self,
        callee: &Expr,
        method_name: &Ident,
        args: &[Expr],
        position: SourcePosition,
    ) -> (tast::TExprKind, Type) {
        let typed_callee = self.visit_expr(callee);

        let (kind, typ) = match &typed_callee.typ.kind {
            TypeKind::Class(class_name) => {
                let all_methods = self.ctx.symbol_table.get_all_class_methods(class_name.name);
                if let Some(method_info) = all_methods.get(&method_name.name) {
                    let method_info = method_info.clone();
                    let typed_args =
                        self.check_and_transform_arguments(&method_info, args, position);
                    let kind = tast::TExprKind::Method {
                        object: Box::new(typed_callee.clone()),
                        method_name: *method_name,
                        args: typed_args,
                    };
                    (kind, method_info.return_type.clone())
                } else {
                    self.ctx.add_error(SemanticError::UndefinedMethod {
                        class_name: class_name.name,
                        method_name: method_name.name,
                        position,
                        class_position: class_name.position,
                    });
                    let typed_args = args.iter().map(|arg| self.visit_expr(arg)).collect();
                    let kind = tast::TExprKind::Method {
                        object: Box::new(typed_callee.clone()),
                        method_name: *method_name,
                        args: typed_args,
                    };
                    (kind, self.ctx.error_type(position))
                }
            }
            TypeKind::Error => {
                let typed_args = args.iter().map(|arg| self.visit_expr(arg)).collect();
                let kind = tast::TExprKind::Method {
                    object: Box::new(typed_callee.clone()),
                    method_name: *method_name,
                    args: typed_args,
                };
                (kind, self.ctx.error_type(position))
            }
            _ => {
                self.ctx.add_error(SemanticError::InvalidOperands {
                    operator: StrID::from(format!("method call .{}()", method_name.name)),
                    operand: StrID::from(typed_callee.typ.to_string()),
                    position,
                    type_position: typed_callee.typ.position,
                });
                let typed_args = args.iter().map(|arg| self.visit_expr(arg)).collect();
                let kind = tast::TExprKind::Method {
                    object: Box::new(typed_callee.clone()),
                    method_name: *method_name,
                    args: typed_args,
                };
                (kind, self.ctx.error_type(position))
            }
        };
        (kind, typ)
    }

    fn visit_unary_operator(
        &mut self,
        op: &UnaryOp,
        expr: &Expr,
        position: SourcePosition,
    ) -> (tast::TExprKind, Type) {
        let typed_expr = self.visit_expr(expr);
        if !self.ctx.type_legal_for_unary_op(op, &typed_expr.typ)
            && !matches!(typed_expr.typ.kind, TypeKind::Error)
        {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from(op.to_string()),
                operand: StrID::from(typed_expr.typ.to_string()),
                position: expr.position,
                type_position: typed_expr.typ.position,
            });
        }
        let result_type = self.ctx.unary_op_result_type(op, position);
        let kind = tast::TExprKind::UnaryOp {
            op: op.clone(),
            expr: Box::new(typed_expr),
        };
        (kind, result_type)
    }

    fn visit_binary_operator(
        &mut self,
        op: &BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        position: SourcePosition,
    ) -> (tast::TExprKind, Type) {
        let typed_lhs = self.visit_expr(lhs);
        let typed_rhs = self.visit_expr(rhs);

        // Early return for error types to prevent cascading errors.
        if matches!(typed_lhs.typ.kind, TypeKind::Error)
            || matches!(typed_rhs.typ.kind, TypeKind::Error)
        {
            let kind = tast::TExprKind::BinaryOp {
                op: op.clone(),
                lhs: Box::new(typed_lhs.clone()),
                rhs: Box::new(typed_rhs),
            };
            let typ = self.ctx.binary_op_result_type(op, &typed_lhs.typ, position);
            return (kind, typ);
        }

        self.validate_binary_operands(
            op,
            &typed_lhs.typ,
            &typed_rhs.typ,
            lhs.position,
            rhs.position,
        );
        if matches!(op, BinaryOp::Div | BinaryOp::Mod)
            && let ExprKind::LiteralInt(0) = rhs.kind
        {
            self.ctx.add_error(SemanticError::ZeroDivision {
                position: rhs.position,
            });
        }
        let result_type = self.ctx.binary_op_result_type(op, &typed_lhs.typ, position);
        let kind = tast::TExprKind::BinaryOp {
            op: op.clone(),
            lhs: Box::new(typed_lhs),
            rhs: Box::new(typed_rhs),
        };
        (kind, result_type)
    }
}

#[cfg(test)]
mod expr_semantic_tests {
    use std::collections::HashMap;

    use super::*;
    use crate::semantics::{ClassInfo, analysis::context::SemanticContext};

    fn setup_complex_ctx() -> SemanticContext {
        let mut ctx = SemanticContext::new();

        let base_name = StrID::from("A");
        let derived_name = StrID::from("B");

        ctx.define_class(ClassInfo {
            ident: Ident {
                name: base_name,
                position: SourcePosition(0, 0),
            },
            parent: None,
            fields: HashMap::new(),
            methods: HashMap::new(),
            field_order: vec![],
            method_order: vec![],
            position: SourcePosition(0, 0),
        });

        ctx.define_class(ClassInfo {
            ident: Ident {
                name: derived_name,
                position: SourcePosition(0, 0),
            },
            parent: Some(Ident {
                name: base_name,
                position: SourcePosition(0, 0),
            }),
            fields: HashMap::new(),
            methods: HashMap::new(),
            field_order: vec![],
            method_order: vec![],
            position: SourcePosition(0, 0),
        });

        ctx.define_function(FunctionInfo {
            ident: Ident {
                name: StrID::from("foo"),
                position: SourcePosition(0, 0),
            },
            return_type: Type {
                kind: TypeKind::Void,
                position: SourcePosition(0, 0),
            },
            params: vec![
                Type {
                    kind: TypeKind::Int,
                    position: SourcePosition(0, 0),
                },
                Type {
                    kind: TypeKind::Class(Ident {
                        name: base_name,
                        position: SourcePosition(0, 0),
                    }),
                    position: SourcePosition(0, 0),
                },
            ],
            position: SourcePosition(0, 0),
        });

        ctx
    }

    #[test]
    fn test_null_assignment() {
        let mut ctx = SemanticContext::new();
        let mut checker = ExpressionChecker::new(&mut ctx);

        // (int)null
        let bad_cast = Expr {
            kind: ExprKind::TypedNull(Type {
                kind: TypeKind::Int,
                position: SourcePosition(0, 0),
            }),
            position: SourcePosition(0, 0),
        };
        checker.check(&bad_cast);
        assert!(!ctx.errors.is_empty());
        assert!(matches!(
            ctx.errors[0],
            SemanticError::InvalidNullCast { .. }
        ));
    }
    #[test]
    fn test_binary_op_mismatch() {
        let mut ctx = SemanticContext::new();
        let mut checker = ExpressionChecker::new(&mut ctx);

        // 5 + true (Int + Bool)
        let expr = Expr {
            kind: ExprKind::BinaryOp {
                op: BinaryOp::Add,
                lhs: Box::new(Expr {
                    kind: ExprKind::LiteralInt(5),
                    position: SourcePosition(0, 0),
                }),
                rhs: Box::new(Expr {
                    kind: ExprKind::LiteralTrue,
                    position: SourcePosition(0, 0),
                }),
            },
            position: SourcePosition(0, 0),
        };

        checker.check(&expr);
        assert!(!ctx.errors.is_empty());

        let has_mismatch = ctx.errors.iter().any(|e| {
            matches!(
                e,
                SemanticError::BinaryOpTypeMismatch { .. } | SemanticError::InvalidOperands { .. }
            )
        });
        assert!(has_mismatch);
    }

    #[test]
    fn test_string_concatenation_legal() {
        let mut ctx = SemanticContext::new();
        let mut checker = ExpressionChecker::new(&mut ctx);

        // "hello" + "world"
        let expr = Expr {
            kind: ExprKind::BinaryOp {
                op: BinaryOp::Add,
                lhs: Box::new(Expr {
                    kind: ExprKind::LiteralString("hello".into()),
                    position: SourcePosition(0, 0),
                }),
                rhs: Box::new(Expr {
                    kind: ExprKind::LiteralString("world".into()),
                    position: SourcePosition(0, 0),
                }),
            },
            position: SourcePosition(0, 0),
        };

        let typed_expr = checker.check(&expr);
        assert!(ctx.errors.is_empty());
        assert!(matches!(typed_expr.typ.kind, TypeKind::String));
    }

    #[test]
    fn test_subtyping_in_function_arguments() {
        let mut ctx = setup_complex_ctx();
        let mut checker = ExpressionChecker::new(&mut ctx);

        // testFunc(10, new Derived)
        let expr = Expr {
            kind: ExprKind::App {
                callee: Box::new(Expr {
                    kind: ExprKind::Var(Ident {
                        name: "foo".into(),
                        position: SourcePosition(0, 0),
                    }),
                    position: SourcePosition(0, 0),
                }),
                args: vec![
                    Expr {
                        kind: ExprKind::LiteralInt(10),
                        position: SourcePosition(0, 0),
                    },
                    Expr {
                        kind: ExprKind::NewObject(Type {
                            kind: TypeKind::Class(Ident {
                                name: "B".into(),
                                position: SourcePosition(0, 0),
                            }),
                            position: SourcePosition(0, 0),
                        }),
                        position: SourcePosition(0, 0),
                    },
                ],
            },
            position: SourcePosition(0, 0),
        };

        checker.check(&expr);
        assert!(
            ctx.errors.is_empty(),
            "B should be accepted where A is expected"
        );
    }

    #[test]
    fn test_builtin_length_field() {
        let mut ctx = SemanticContext::new();
        let mut checker = ExpressionChecker::new(&mut ctx);

        // "test".length
        let expr = Expr {
            kind: ExprKind::FieldAccess {
                object: Box::new(Expr {
                    kind: ExprKind::LiteralString("test".into()),
                    position: SourcePosition(0, 0),
                }),
                field_name: Ident {
                    name: "length".into(),
                    position: SourcePosition(0, 0),
                },
            },
            position: SourcePosition(0, 0),
        };

        let typed_expr = checker.check(&expr);
        assert!(ctx.errors.is_empty());
        assert!(matches!(typed_expr.typ.kind, TypeKind::Int));
    }

    #[test]
    fn test_invalid_field_access_on_primitive() {
        let mut ctx = SemanticContext::new();
        let mut checker = ExpressionChecker::new(&mut ctx);

        // 5.fld
        let expr = Expr {
            kind: ExprKind::FieldAccess {
                object: Box::new(Expr {
                    kind: ExprKind::LiteralInt(5),
                    position: SourcePosition(0, 0),
                }),
                field_name: Ident {
                    name: "fld".into(),
                    position: SourcePosition(0, 0),
                },
            },
            position: SourcePosition(0, 0),
        };

        checker.check(&expr);
        assert!(!ctx.errors.is_empty());
        assert!(
            ctx.errors
                .iter()
                .any(|e| matches!(e, SemanticError::InvalidOperands { .. }))
        );
    }
}
