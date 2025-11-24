use crate::errors::SemanticError;
use crate::parser::ast::*;
use crate::semantics::VariableInfo;
use crate::semantics::analysis::context::SemanticContext;
use crate::semantics::analysis::expr_checker::ExpressionChecker;
use crate::semantics::analysis::return_checker::ReturnChecker;
use crate::utils::StrID;

pub struct StatementChecker<'a> {
    ctx: &'a mut SemanticContext,
}

// ======================
// Basic functionality.
// ======================
impl<'a> StatementChecker<'a> {
    pub fn new(ctx: &'a mut SemanticContext) -> Self {
        Self { ctx }
    }

    /// Main function to perform a type check of a statement.
    /// Logs errors in the context in case of errors.
    pub fn check_stmt(&mut self, stmt: &Stmt) {
        self.visit_stmt(stmt)
    }

    /// Main function to perform a type check of a block statement.
    /// Logs errors in the context in case of errors.
    pub fn check_block(&mut self, block: &Block) {
        self.visit_block(block);
    }
}

// ======================
// Utilities.
// ======================
impl<'a> StatementChecker<'a> {
    /// Checks if an expression is a LValue. Logs error otherwise.
    fn check_lvalue(&mut self, expr: &Expr) {
        let is_lvalue = matches!(
            expr.kind,
            ExprKind::Var(_) | ExprKind::ArrayAccess { .. } | ExprKind::FieldAccess { .. }
        );

        if !is_lvalue {
            self.ctx.add_error(SemanticError::InvalidLValue {
                position: expr.position,
            });
        }
    }

    // Defines a variable in the current scope. Logs an error on duplicate definition.
    fn define_variable(&mut self, ident: &Ident, typ: &Type) {
        let var_info = VariableInfo {
            ident: *ident,
            typ: typ.clone(),
        };
        if let Err(e) = self.ctx.symbol_table.define_variable(var_info) {
            self.ctx.add_error(e);
        }
    }
}

// ======================
// Statement Visitors.
// ======================
impl<'a> StatementChecker<'a> {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        let position = stmt.position;
        match &stmt.kind {
            StmtKind::Empty => (),
            StmtKind::Block(block) => self.visit_block(block),
            StmtKind::Decl(typ, items) => self.visit_declaration(typ, items, position),
            StmtKind::Assign { target, value } => self.visit_assign(target, value, position),
            StmtKind::Return { expr } => self.visit_return(expr, position),
            StmtKind::Incr(expr) => self.visit_increment(expr, position),
            StmtKind::Decr(expr) => self.visit_decrement(expr, position),
            StmtKind::If {
                cond,
                if_true,
                if_false,
            } => self.visit_if(cond, if_true, if_false, position),
            StmtKind::While { cond, body } => self.visit_while(cond, body, position),
            StmtKind::For {
                init,
                cond,
                post,
                body,
            } => self.visit_for(init, cond, post, body, position),
            StmtKind::ForEach {
                element_type,
                item_name,
                collection,
                body,
            } => self.visit_for_each(element_type, item_name, collection, body, position),
            StmtKind::Expr(expr) => {
                ExpressionChecker::new(self.ctx).check(expr);
            }
            StmtKind::FuncDef(func_def) => self.visit_nested_function_definition(func_def),
        }
    }

    fn visit_block(&mut self, block: &Block) {
        self.ctx.symbol_table.enter_scope();
        // TODO: Here we should:
        // - Look for defined functions
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        self.ctx.symbol_table.leave_scope();
    }

    fn visit_declaration(&mut self, typ: &Type, items: &Vec<Item>, _position: SourcePosition) {
        self.ctx.validate_variable_type(typ);
        for item in items {
            match &item.kind {
                ItemKind::NoInit(ident) => {
                    self.define_variable(ident, typ);
                }
                ItemKind::Init(ident, expr) => {
                    let expr_type = ExpressionChecker::new(self.ctx).check(expr);
                    if !self.ctx.is_subtype(&expr_type, typ) {
                        self.ctx.add_error(SemanticError::DeclarationTypeMismatch {
                            var_name: ident.name,
                            expected: StrID::from(typ.to_string()),
                            decl_type_position: typ.position,
                            found: StrID::from(expr_type.to_string()),
                            expr_position: expr.position,
                        });
                    }
                    self.define_variable(ident, typ);
                }
            }
        }
    }

    fn visit_assign(&mut self, target: &Expr, value: &Expr, _position: SourcePosition) {
        // Continue checking rhs even if lhs is invalid lvalue
        self.check_lvalue(target);

        let mut expr_checker = ExpressionChecker::new(self.ctx);
        let target_type = expr_checker.check(target);
        let value_type = expr_checker.check(value);

        if !self.ctx.is_subtype(&value_type, &target_type) {
            self.ctx.add_error(SemanticError::AssignTypeMismatch {
                lhs_type: StrID::from(target_type.to_string()),
                rhs_type: StrID::from(value_type.to_string()),
                lhs_position: target_type.position,
                rhs_position: value_type.position,
            });
        }
    }

    fn visit_return(&mut self, expr: &Option<Box<Expr>>, position: SourcePosition) {
        let return_type = self
            .ctx
            .current_return_type
            .clone()
            .expect("Internal error: visit_return() with no expected return type");

        match expr {
            Some(expr) => {
                let expr_type = ExpressionChecker::new(self.ctx).check(expr);

                // Can't return void expression.
                if matches!(expr_type.kind, TypeKind::Void) {
                    self.ctx.add_error(SemanticError::InvalidVoidReturn {
                        ret_type_position: expr_type.position,
                    });
                }

                if !self.ctx.is_subtype(&expr_type, &return_type) {
                    self.ctx.add_error(SemanticError::InvalidReturnValue {
                        expected: StrID::from(return_type.to_string()),
                        ret_type_position: return_type.position,
                        found: StrID::from(expr_type.to_string()),
                        position: expr.position,
                    });
                }
            }
            None => {
                if !matches!(return_type.kind, TypeKind::Void) {
                    self.ctx.add_error(SemanticError::InvalidReturnValue {
                        expected: StrID::from(return_type.to_string()),
                        ret_type_position: return_type.position,
                        found: StrID::from("void"),
                        position,
                    });
                }
            }
        }
    }

    fn visit_increment(&mut self, expr: &Expr, _position: SourcePosition) {
        let expr_type = ExpressionChecker::new(self.ctx).check(expr);

        if !matches!(expr_type.kind, TypeKind::Int | TypeKind::Error) {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from("++"),
                operand: StrID::from(expr_type.to_string()),
                position: expr.position,
                type_position: expr_type.position,
            });
        }

        self.check_lvalue(expr);
    }

    // TODO: Remove positions
    fn visit_decrement(&mut self, expr: &Expr, _position: SourcePosition) {
        let expr_type = ExpressionChecker::new(self.ctx).check(expr);

        if !matches!(expr_type.kind, TypeKind::Int | TypeKind::Error) {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from("--"),
                // TODO: Implement type_to_strid?
                operand: StrID::from(expr_type.to_string()),
                position: expr.position,
                type_position: expr_type.position,
            });
        }

        self.check_lvalue(expr);
    }

    fn visit_if(
        &mut self,
        cond: &Expr,
        if_true: &Stmt,
        if_false: &Option<Box<Stmt>>,
        _position: SourcePosition,
    ) {
        let cond_type = ExpressionChecker::new(self.ctx).check(cond);

        if !matches!(cond_type.kind, TypeKind::Bool | TypeKind::Error) {
            self.ctx.add_error(SemanticError::TypeMismatch {
                expected: StrID::from("boolean"),
                found: StrID::from(cond_type.to_string()),
                position: cond.position,
            });
        }

        self.ctx.symbol_table.enter_scope();
        self.visit_stmt(if_true);
        self.ctx.symbol_table.leave_scope();

        if let Some(if_false) = if_false {
            self.ctx.symbol_table.enter_scope();
            self.visit_stmt(if_false);
            self.ctx.symbol_table.leave_scope();
        }
    }

    fn visit_while(&mut self, cond: &Expr, body: &Stmt, _position: SourcePosition) {
        let cond_type = ExpressionChecker::new(self.ctx).check(cond);

        if !matches!(cond_type.kind, TypeKind::Bool | TypeKind::Error) {
            self.ctx.add_error(SemanticError::TypeMismatch {
                expected: StrID::from("boolean"),
                found: StrID::from(cond_type.to_string()),
                position: cond.position,
            });
        }
        self.ctx.symbol_table.enter_scope();
        self.visit_stmt(body);
        self.ctx.symbol_table.leave_scope();
    }

    fn visit_for(
        &mut self,
        init: &Option<Box<Stmt>>,
        cond: &Option<Box<Expr>>,
        post: &Option<Box<Expr>>,
        body: &Stmt,
        _position: SourcePosition,
    ) {
        self.ctx.symbol_table.enter_scope();

        if let Some(init_stmt) = init {
            // The init variable will be defined in `visit_statement`.
            self.visit_stmt(init_stmt);
        }

        if let Some(cond_expr) = cond {
            let cond_type = ExpressionChecker::new(self.ctx).check(cond_expr);
            if !matches!(cond_type.kind, TypeKind::Bool | TypeKind::Error) {
                self.ctx.add_error(SemanticError::TypeMismatch {
                    expected: StrID::from("boolean"),
                    found: StrID::from(cond_type.to_string()),
                    position: cond_expr.position,
                });
            }
        }

        if let Some(post_expr) = post {
            ExpressionChecker::new(self.ctx).check(post_expr);
        }

        self.visit_stmt(body);
        self.ctx.symbol_table.leave_scope();
    }

    fn visit_for_each(
        &mut self,
        element_type: &Type,
        item_name: &Ident,
        collection: &Expr,
        body: &Stmt,
        _position: SourcePosition,
    ) {
        let collection_type = ExpressionChecker::new(self.ctx).check(collection);

        if let TypeKind::Array(inner) = &collection_type.kind {
            if !self.ctx.is_subtype(inner, element_type) {
                self.ctx.add_error(SemanticError::ForEachTypeMismatch {
                    variable_type: StrID::from(element_type.to_string()),
                    collection_item_type: StrID::from(inner.to_string()),
                    variable_position: element_type.position,
                    collection_position: collection.position,
                });
            }
        } else {
            self.ctx.add_error(SemanticError::TypeMismatch {
                expected: StrID::from(format!("{}[]", element_type)),
                found: StrID::from(collection_type.to_string()),
                position: collection.position,
            });
        }

        self.ctx.symbol_table.enter_scope();
        self.define_variable(item_name, element_type);
        self.visit_stmt(body);
        self.ctx.symbol_table.leave_scope();
    }

    // TODO: Remove that?
    fn visit_nested_function_definition(&mut self, func: &FuncDef) {
        self.ctx.symbol_table.enter_scope();

        self.ctx.validate_function_return_type(&func.return_type);

        // TODO: Try to use parametrized lifetime here to avoid cloning?
        let previous_return_type = self.ctx.current_return_type.take();
        self.ctx.current_return_type = Some(func.return_type.clone());

        // TODO: Here we should look for all nested function definitions and handle them.

        // Add arguments to scope.
        for arg in &func.args {
            self.ctx.validate_variable_type(&arg.arg_type);
            self.ctx.define_variable(arg.name, arg.arg_type.clone());
        }

        self.visit_block(&func.body);

        // Check if function is guaranteed to return.
        if !matches!(func.return_type.kind, TypeKind::Void)
            && !ReturnChecker::function_guarantees_return(&func.body)
        {
            self.ctx.add_error(SemanticError::MissingReturn {
                function_name: func.name.name,
                position: func.position,
            });
        }

        // TODO: This is bugged in nested functions.
        self.ctx.current_return_type = previous_return_type;
        self.ctx.symbol_table.leave_scope();
    }
}
