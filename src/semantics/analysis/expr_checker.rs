use crate::errors::SemanticError;
use crate::parser::ast::TypeKind;
use crate::parser::ast::*;
use crate::semantics::FunctionInfo;
use crate::semantics::analysis::context::SemanticContext;
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
    pub fn check(&mut self, expr: &Expr) -> Type {
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
    fn visit_expr(&mut self, outer_expr: &Expr) -> Type {
        match &outer_expr.kind {
            ExprKind::Var(ident) => match self.ctx.symbol_table.lookup_variable(ident) {
                Ok(t) => t,
                Err(e) => {
                    self.ctx.add_error(e);
                    self.ctx.error_type(ident.position)
                }
            },
            ExprKind::LiteralInt(_) => Type {
                kind: TypeKind::Int,
                position: outer_expr.position,
            },
            ExprKind::LiteralTrue => Type {
                kind: TypeKind::Bool,
                position: outer_expr.position,
            },
            ExprKind::LiteralFalse => Type {
                kind: TypeKind::Bool,
                position: outer_expr.position,
            },
            ExprKind::LiteralString(_) => Type {
                kind: TypeKind::String,
                position: outer_expr.position,
            },
            ExprKind::LiteralNull => Type {
                kind: TypeKind::Null,
                position: outer_expr.position,
            },
            ExprKind::App { callee, args } => {
                self.visit_application(callee, args, outer_expr.position)
            }
            ExprKind::ArrayAccess { array, index } => {
                self.visit_array_access(array, index, outer_expr.position)
            }
            ExprKind::FieldAccess { object, field_name } => {
                self.visit_field_access(object, field_name, outer_expr.position)
            }
            ExprKind::Method {
                object,
                method_name,
                args,
            } => self.visit_method_call(object, method_name, args, outer_expr.position),
            ExprKind::Paren(inner) => self.visit_expr(inner),
            ExprKind::UnaryOp { op, expr } => {
                self.visit_unary_operator(op, expr, outer_expr.position)
            }
            ExprKind::BinaryOp { op, lhs, rhs } => {
                self.visit_binary_operator(op, lhs, rhs, outer_expr.position)
            }
            ExprKind::NewObject(typ) => {
                self.ctx.validate_variable_type(typ);
                typ.clone()
            }
            ExprKind::NewArray { element_type, size } => {
                self.ctx.validate_variable_type(element_type);
                let size_type = self.visit_expr(size);
                if !matches!(size_type.kind, TypeKind::Int | TypeKind::Error) {
                    self.ctx.add_error(SemanticError::TypeMismatch {
                        expected: StrID::from("int"),
                        found: StrID::from(size_type.to_string()),
                        position: size.position,
                    });
                }

                Type {
                    kind: TypeKind::Array(Box::new(element_type.clone())),
                    position: outer_expr.position,
                }
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
                        expr_position: outer_expr.position,
                    });
                }

                typ.clone()
            }
            ExprKind::ExprTypedNull(expr) => {
                let expected = vec![
                    StrID::from("{ClassType}"),
                    StrID::from("{ArrayType}"),
                    StrID::from("{string}"),
                ];
                match &expr.kind {
                    ExprKind::Var(ident) => {
                        if !self.ctx.symbol_table.class_exists(ident.name) {
                            self.ctx.add_error(SemanticError::UndefinedClass {
                                name: ident.name,
                                position: ident.position,
                            });
                            self.ctx.error_type(outer_expr.position)
                        } else {
                            Type {
                                kind: TypeKind::Class(ident.clone()),
                                position: ident.position,
                            }
                        }
                    }
                    ExprKind::LiteralString(_) => Type {
                        kind: TypeKind::String,
                        position: outer_expr.position,
                    },
                    _ => {
                        self.ctx.add_error(SemanticError::InvalidNullCastExpr {
                            expected,
                            cast_position: expr.position,
                            expr_position: outer_expr.position,
                        });
                        self.ctx.error_type(outer_expr.position)
                    }
                }
            }
            ExprKind::This => {
                if let Some(class_name) = &self.ctx.current_class {
                    let class_ident = Ident {
                        name: class_name.clone(),
                        position: outer_expr.position,
                    };

                    Type {
                        kind: TypeKind::Class(class_ident),
                        position: outer_expr.position,
                    }
                } else {
                    self.ctx.add_error(SemanticError::ThisUsedOutsideClass {
                        position: outer_expr.position,
                    });
                    self.ctx.error_type(outer_expr.position)
                }
            }
        }
    }

    fn visit_application(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        position: SourcePosition,
    ) -> Type {
        // TODO: For now we only support direct calls on function names. Change that to use function types.
        // TODO: implicit this.
        if let ExprKind::Var(name) = &callee.kind {
            match self.ctx.symbol_table.lookup_function(name) {
                Ok(func_info) => {
                    self.validate_function_call(&func_info, args, position);
                    Type {
                        kind: func_info.return_type.kind,
                        position,
                    }
                }
                Err(e) => {
                    self.ctx.add_error(e);
                    // Even if function is not found, we visit arguments to find errors in them.
                    for arg in args {
                        self.visit_expr(arg);
                    }
                    self.ctx.error_type(position)
                }
            }
        } else {
            // Fallback for generic expr (not handled by this parser logic usually)
            self.ctx.error_type(position)
        }
    }

    fn visit_array_access(&mut self, array: &Expr, index: &Expr, position: SourcePosition) -> Type {
        let array_type = self.visit_expr(array);
        let index_type = self.visit_expr(index);

        if !matches!(index_type.kind, TypeKind::Int | TypeKind::Error) {
            self.ctx.add_error(SemanticError::TypeMismatch {
                expected: StrID::from("int"),
                found: StrID::from(index_type.to_string()),
                position: index.position,
            });
        }

        match &array_type.kind {
            TypeKind::Array(elem_type) => (**elem_type).clone(),
            TypeKind::Error => self.ctx.error_type(position),
            _ => {
                self.ctx.add_error(SemanticError::InvalidOperands {
                    operator: StrID::from("[]"),
                    operand: StrID::from(array_type.to_string()),
                    position,
                    type_position: array_type.position,
                });
                self.ctx.error_type(position)
            }
        }
    }

    fn visit_field_access(
        &mut self,
        object: &Expr,
        field_name: &Ident,
        position: SourcePosition,
    ) -> Type {
        let obj_type = self.visit_expr(object);

        if let TypeKind::Class(class_name) = obj_type.kind {
            let all_fields = self.ctx.symbol_table.get_all_class_fields(class_name.name);

            if let Some((field_type, _)) = all_fields.get(&field_name.name) {
                field_type.clone()
            } else {
                self.ctx.add_error(SemanticError::UndefinedField {
                    class_name: class_name.name, 
                    field_name: field_name.name.clone(),
                    position,
                    class_position: class_name.position,
                });
                self.ctx.error_type(position)
            }
        } else if let TypeKind::String | TypeKind::Array(_) = obj_type.kind {
            if field_name.name != StrID::from("length") {
                self.ctx.add_error(SemanticError::UndefinedField {
                    class_name: StrID::from(obj_type.to_string()),
                    field_name: field_name.name.clone(),
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
        } else if matches!(obj_type.kind, TypeKind::Error) {
            self.ctx.error_type(position)
        } else {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from("."),
                operand: StrID::from(obj_type.to_string()),
                position,
                type_position: obj_type.position,
            });
            self.ctx.error_type(position)
        }
    }

    fn visit_method_call(
        &mut self,
        callee: &Expr,
        method_name: &Ident,
        args: &[Expr],
        position: SourcePosition,
    ) -> Type {
        let obj_type = self.visit_expr(callee);

        match &obj_type.kind {
            TypeKind::Class(class_name) => {
                let all_methods = self
                    .ctx
                    .symbol_table
                    .get_all_class_methods(class_name.name);

                if let Some(method_info) = all_methods.get(&method_name.name) {
                    let method_info = method_info.clone();
                    self.validate_function_call(&method_info, args, position);

                    Type {
                        kind: method_info.return_type.kind.clone(),
                        position,
                    }
                } else {
                    self.ctx.add_error(SemanticError::UndefinedMethod {
                        class_name: class_name.name.clone(),
                        method_name: method_name.name.clone(),
                        position,
                        class_position: class_name.position,
                    });
                    // Visit args anyway.
                    for arg in args {
                        self.visit_expr(arg);
                    }
                    self.ctx.error_type(position)
                }
            }
            TypeKind::Error => {
                // Visit args anyway.
                for arg in args {
                    self.visit_expr(arg);
                }
                self.ctx.error_type(position)
            }
            _ => {
                self.ctx.add_error(SemanticError::InvalidOperands {
                    operator: StrID::from(format!("method call .{}()", method_name.name)),
                    operand: StrID::from(obj_type.to_string()),
                    position,
                    type_position: obj_type.position,
                });
                for arg in args {
                    self.visit_expr(arg);
                }
                self.ctx.error_type(position)
            }
        }
    }

    fn visit_unary_operator(
        &mut self,
        op: &UnaryOp,
        expr: &Expr,
        position: SourcePosition,
    ) -> Type {
        let expr_type = self.visit_expr(expr);

        if !self.ctx.type_legal_for_unary_op(op, &expr_type)
            && !matches!(expr_type.kind, TypeKind::Error)
        {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from(op.to_string()),
                operand: StrID::from(expr_type.to_string()),
                position: expr.position,
                type_position: expr_type.position,
            });
        }
        self.ctx.unary_op_result_type(op, position)
    }

    fn visit_binary_operator(
        &mut self,
        op: &BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        position: SourcePosition,
    ) -> Type {
        let lhs_type = self.visit_expr(lhs);
        let rhs_type = self.visit_expr(rhs);

        // Early return for error types to prevent cascading errors.
        if matches!(lhs_type.kind, TypeKind::Error) || matches!(rhs_type.kind, TypeKind::Error) {
            return self.ctx.binary_op_result_type(op, &lhs_type, position);
        }

        self.validate_binary_operands(op, &lhs_type, &rhs_type, lhs.position, rhs.position);

        if matches!(op, BinaryOp::Div | BinaryOp::Mod)
            && let ExprKind::LiteralInt(0) = rhs.kind
        {
            self.ctx.add_error(SemanticError::ZeroDivision {
                position: rhs.position,
            });
        }

        self.ctx.binary_op_result_type(op, &lhs_type, position)
    }

    // TODO: Try to move that from here?
    fn validate_function_call(
        &mut self,
        func_info: &FunctionInfo,
        args: &[Expr],
        expr_position: SourcePosition,
    ) {
        let func_ident = func_info.ident.clone();
        if args.len() != func_info.params.len() {
            self.ctx.add_error(SemanticError::WrongNumberOfArguments {
                name: func_ident.name.clone(),
                expected: func_info.params.len(),
                found: args.len(),
                call_position: expr_position,
                function_position: func_info.position,
            });
            // Don't return here, check types of what we have if possible or just mismatch
        }

        // Type check arguments.
        for (expr, param) in args.iter().zip(&func_info.params) {
            let arg_type = self.visit_expr(expr);
            if !self.ctx.is_subtype(&arg_type, param) {
                self.ctx.add_error(SemanticError::ArgumentTypeMismatch {
                    expected: StrID::from(param.to_string()),
                    found: StrID::from(arg_type.to_string()),
                    position: expr.position,
                    function_name: func_info.ident.name.clone(),
                    argument_position: param.position,
                });
            }
        }
    }
}
