use crate::errors::SemanticError;
use crate::parser::ast::*;
use crate::semantics::analysis::context::SemanticContext;
use crate::semantics::analysis::expr_checker::ExpressionChecker;
use crate::semantics::analysis::return_checker::ReturnChecker;
use crate::semantics::{VariableInfo, tast};
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
    /// Returns the transformed typed AST.
    pub fn check_stmt(&mut self, stmt: &Stmt) -> tast::TStmt {
        self.visit_stmt(stmt)
    }

    /// Main function to perform a type check of a block statement.
    /// Logs errors in the context in case of errors.
    /// Returns the transformed typed AST.
    pub fn check_block(&mut self, block: &Block) -> tast::TBlock {
        self.visit_block(block)
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
    fn visit_stmt(&mut self, stmt: &Stmt) -> tast::TStmt {
        let position = stmt.position;
        let kind = match &stmt.kind {
            StmtKind::Empty => tast::TStmtKind::Empty,
            StmtKind::Block(block) => tast::TStmtKind::Block(self.visit_block(block)),
            StmtKind::Decl(typ, items) => self.visit_declaration(typ, items),
            StmtKind::Assign { target, value } => self.visit_assign(target, value),
            StmtKind::Return { expr } => self.visit_return(expr, position),
            StmtKind::Incr(expr) => self.visit_increment(expr),
            StmtKind::Decr(expr) => self.visit_decrement(expr),
            StmtKind::If {
                cond,
                if_true,
                if_false,
            } => self.visit_if(cond, if_true, if_false),
            StmtKind::While { cond, body } => self.visit_while(cond, body),
            StmtKind::For {
                init,
                cond,
                post,
                body,
            } => self.visit_for(init, cond, post, body),
            StmtKind::ForEach {
                element_type,
                item_name,
                collection,
                body,
            } => self.visit_for_each(element_type, item_name, collection, body),
            StmtKind::Expr(expr) => {
                tast::TStmtKind::Expr(ExpressionChecker::new(self.ctx).check(expr))
            }
            StmtKind::FuncDef(func_def) => self.visit_nested_function_definition(func_def),
        };
        tast::TStmt { kind, position }
    }

    fn visit_block(&mut self, block: &Block) -> tast::TBlock {
        self.ctx.symbol_table.enter_scope();
        // TODO: Here we should:
        // - Look for defined functions
        let stmts = block.stmts.iter().map(|s| self.visit_stmt(s)).collect();
        self.ctx.symbol_table.leave_scope();
        tast::TBlock {
            stmts,
            position: block.position,
        }
    }

    fn visit_declaration(&mut self, typ: &Type, items: &[Item]) -> tast::TStmtKind {
        self.ctx.validate_variable_type(typ);
        let typed_items = items
            .iter()
            .map(|item| self.visit_item(item, typ))
            .collect();
        tast::TStmtKind::Decl(typ.clone(), typed_items)
    }

    fn visit_item(&mut self, item: &Item, typ: &Type) -> tast::TItem {
        let kind = match &item.kind {
            ItemKind::NoInit(ident) => {
                self.define_variable(ident, typ);
                tast::TItemKind::NoInit(*ident)
            }
            ItemKind::Init(ident, expr) => {
                let typed_expr = ExpressionChecker::new(self.ctx).check(expr);
                if !self.ctx.is_subtype(&typed_expr.typ, typ) {
                    self.ctx.add_error(SemanticError::DeclarationTypeMismatch {
                        var_name: ident.name,
                        expected: StrID::from(typ.to_string()),
                        decl_type_position: typ.position,
                        found: StrID::from(typed_expr.typ.to_string()),
                        expr_position: expr.position,
                    });
                }
                self.define_variable(ident, typ);
                tast::TItemKind::Init(*ident, typed_expr)
            }
        };
        tast::TItem {
            kind,
            position: item.position,
        }
    }

    fn visit_assign(&mut self, target: &Expr, value: &Expr) -> tast::TStmtKind {
        // Continue checking rhs even if lhs is invalid lvalue
        self.check_lvalue(target);

        let mut expr_checker = ExpressionChecker::new(self.ctx);
        let typed_target = expr_checker.check(target);
        let typed_value = expr_checker.check(value);

        if !self.ctx.is_subtype(&typed_value.typ, &typed_target.typ) {
            self.ctx.add_error(SemanticError::AssignTypeMismatch {
                lhs_type: StrID::from(typed_target.typ.to_string()),
                rhs_type: StrID::from(typed_value.typ.to_string()),
                lhs_position: typed_target.typ.position,
                rhs_position: typed_value.typ.position,
            });
        }
        tast::TStmtKind::Assign {
            target: typed_target,
            value: typed_value,
        }
    }

    fn visit_return(
        &mut self,
        expr: &Option<Box<Expr>>,
        position: SourcePosition,
    ) -> tast::TStmtKind {
        let return_type = self
            .ctx
            .current_return_type
            .clone()
            .expect("Internal error: visit_return() with no expected return type");

        let typed_expr = match expr {
            Some(expr) => {
                let typed_expr = ExpressionChecker::new(self.ctx).check(expr);
                // Can't return void expression.
                if matches!(typed_expr.typ.kind, TypeKind::Void) {
                    self.ctx.add_error(SemanticError::InvalidVoidReturn {
                        ret_type_position: typed_expr.typ.position,
                    });
                }
                if !self.ctx.is_subtype(&typed_expr.typ, &return_type) {
                    self.ctx.add_error(SemanticError::InvalidReturnValue {
                        expected: StrID::from(return_type.to_string()),
                        ret_type_position: return_type.position,
                        found: StrID::from(typed_expr.typ.to_string()),
                        position: expr.position,
                    });
                }
                Some(typed_expr)
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
                None
            }
        };
        tast::TStmtKind::Return { expr: typed_expr }
    }

    fn visit_increment(&mut self, expr: &Expr) -> tast::TStmtKind {
        let typed_expr = ExpressionChecker::new(self.ctx).check(expr);

        if !matches!(typed_expr.typ.kind, TypeKind::Int | TypeKind::Error) {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from("++"),
                operand: StrID::from(typed_expr.typ.to_string()),
                position: expr.position,
                type_position: typed_expr.typ.position,
            });
        }
        self.check_lvalue(expr);
        tast::TStmtKind::Incr(typed_expr)
    }

    fn visit_decrement(&mut self, expr: &Expr) -> tast::TStmtKind {
        let typed_expr = ExpressionChecker::new(self.ctx).check(expr);

        if !matches!(typed_expr.typ.kind, TypeKind::Int | TypeKind::Error) {
            self.ctx.add_error(SemanticError::InvalidOperands {
                operator: StrID::from("--"),
                // TODO: Imlement type_to_str_id?
                operand: StrID::from(typed_expr.typ.to_string()),
                position: expr.position,
                type_position: typed_expr.typ.position,
            });
        }
        self.check_lvalue(expr);
        tast::TStmtKind::Decr(typed_expr)
    }

    fn visit_if(
        &mut self,
        cond: &Expr,
        if_true: &Stmt,
        if_false: &Option<Box<Stmt>>,
    ) -> tast::TStmtKind {
        let typed_cond = ExpressionChecker::new(self.ctx).check(cond);
        if !matches!(typed_cond.typ.kind, TypeKind::Bool | TypeKind::Error) {
            self.ctx.add_error(SemanticError::TypeMismatch {
                expected: StrID::from("boolean"),
                found: StrID::from(typed_cond.typ.to_string()),
                position: cond.position,
            });
        }

        self.ctx.symbol_table.enter_scope();
        let typed_if_true = Box::new(self.visit_stmt(if_true));
        self.ctx.symbol_table.leave_scope();

        self.ctx.symbol_table.enter_scope();
        let typed_if_false = if_false
            .as_ref()
            .map(|stmt| Box::new(self.visit_stmt(stmt)));
        self.ctx.symbol_table.leave_scope();

        tast::TStmtKind::If {
            cond: typed_cond,
            if_true: typed_if_true,
            if_false: typed_if_false,
        }
    }

    fn visit_while(&mut self, cond: &Expr, body: &Stmt) -> tast::TStmtKind {
        let typed_cond = ExpressionChecker::new(self.ctx).check(cond);
        if !matches!(typed_cond.typ.kind, TypeKind::Bool | TypeKind::Error) {
            self.ctx.add_error(SemanticError::TypeMismatch {
                expected: StrID::from("boolean"),
                found: StrID::from(typed_cond.typ.to_string()),
                position: cond.position,
            });
        }
        self.ctx.symbol_table.enter_scope();
        let typed_body = Box::new(self.visit_stmt(body));
        self.ctx.symbol_table.leave_scope();

        tast::TStmtKind::While {
            cond: typed_cond,
            body: typed_body,
        }
    }

    fn visit_for(
        &mut self,
        init: &Option<Box<Stmt>>,
        cond: &Option<Box<Expr>>,
        post: &Option<Box<Expr>>,
        body: &Stmt,
    ) -> tast::TStmtKind {
        self.ctx.symbol_table.enter_scope();

        if let Some(init_stmt) = init {
            // The init variable will be defined in `visit_statement`.
            self.visit_stmt(init_stmt);
        }

        if let Some(cond_expr) = cond {
            let cond_type = ExpressionChecker::new(self.ctx).check(cond_expr);
            if !matches!(cond_type.typ.kind, TypeKind::Bool | TypeKind::Error) {
                self.ctx.add_error(SemanticError::TypeMismatch {
                    expected: StrID::from("boolean"),
                    found: StrID::from(cond_type.typ.to_string()),
                    position: cond_expr.position,
                });
            }
        }

        if let Some(post_expr) = post {
            ExpressionChecker::new(self.ctx).check(post_expr);
        }

        self.visit_stmt(body);
        self.ctx.symbol_table.leave_scope();

        // TODO: For now just skip the for loop. In the future probably remove.
        tast::TStmtKind::Empty
    }

    fn visit_for_each(
        &mut self,
        element_type: &Type,
        item_name: &Ident,
        collection: &Expr,
        body: &Stmt,
    ) -> tast::TStmtKind {
        let typed_collection = ExpressionChecker::new(self.ctx).check(collection);
        if let TypeKind::Array(inner) = &typed_collection.typ.kind {
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
                found: StrID::from(typed_collection.typ.to_string()),
                position: collection.position,
            });
        }

        self.ctx.symbol_table.enter_scope();
        self.define_variable(item_name, element_type);
        let typed_body = self.visit_stmt(body);
        self.ctx.symbol_table.leave_scope();

        tast::TStmtKind::ForEach {
            element_type: element_type.clone(),
            item_name: *item_name,
            collection: typed_collection,
            body: Box::new(typed_body),
        }
    }

    // TODO: Remove that?
    fn visit_nested_function_definition(&mut self, func: &FuncDef) -> tast::TStmtKind {
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
        // TODO: Think about that. Probably remove.
        tast::TStmtKind::Empty
    }
}
