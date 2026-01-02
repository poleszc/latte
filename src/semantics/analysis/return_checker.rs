use crate::parser::ast::{Block, Expr, ExprKind, Stmt, StmtKind};

/// Module responsible for checking if functions guarantee to return a value.
/// This module performs control flow analysis to determine if all code paths
/// in a function lead to a return statement.
pub struct ReturnChecker;

impl ReturnChecker {
    /// Check if a function body guarantees to return.
    pub fn function_guarantees_return(body: &Block) -> bool {
        Self::block_guarantees_return(body)
    }

    /// Check if a statement guarantees to return.
    ///
    /// A statement guarantees return if:
    /// - It's a return statement
    /// - It's a block where at least one statement guarantees return
    /// - It's an if-else where both branches guarantee return
    /// - It's a while(true) loop (infinite loop)
    pub fn stmt_guarantees_return(stmt: &Stmt) -> bool {
        match &stmt.kind {
            StmtKind::Return { .. } => true,
            StmtKind::Block(block) => Self::block_guarantees_return(block),
            StmtKind::If {
                cond,
                if_true,
                if_false,
            } => Self::if_guarantees_return(cond, if_true, if_false),
            StmtKind::While { cond, body } => Self::while_guarantees_return(cond, body),
            StmtKind::ForEach { .. } => false,
            // Other statements don't guarantee return.
            _ => false,
        }
    }

    /// Check if a block guarantees to return.
    fn block_guarantees_return(block: &Block) -> bool {
        // A block guarantees return if any of its statements guarantees return.
        for stmt in &block.stmts {
            if Self::stmt_guarantees_return(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if an if statement guarantees to return.
    fn if_guarantees_return(cond: &Expr, if_true: &Stmt, if_false: &Option<Box<Stmt>>) -> bool {
        match &cond.kind {
            // if (true) - only check true branch
            ExprKind::LiteralTrue => Self::stmt_guarantees_return(if_true),
            // if (false) - only check false branch
            ExprKind::LiteralFalse => {
                if let Some(else_stmt) = if_false {
                    Self::stmt_guarantees_return(else_stmt)
                } else {
                    false
                }
            }
            // if (expr) - both branches must guarantee return
            _ => {
                let true_returns = Self::stmt_guarantees_return(if_true);
                match if_false {
                    Some(else_stmt) => {
                        let false_returns = Self::stmt_guarantees_return(else_stmt);
                        true_returns && false_returns
                    }
                    None => false, // No else branch = doesn't guarantee return
                }
            }
        }
    }

    /// Check if a while loop guarantees to return.
    fn while_guarantees_return(cond: &Expr, _body: &Stmt) -> bool {
        match &cond.kind {
            ExprKind::LiteralTrue => true,
            _ => false, // Other while loops may not execute
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;

    #[test]
    fn test_complex_return_checking() {
        // if (cond) return 1; else { if (cond2) return 2; else return 3; } -> true
        let mock_expr = Box::new(Expr {
            kind: ExprKind::LiteralTrue,
            position: SourcePosition(0, 0),
        });
        let ret_stmt = Stmt {
            kind: StmtKind::Return {
                expr: Some(mock_expr.clone()),
            },
            position: SourcePosition(0, 0),
        };

        let nested_if = Stmt {
            kind: StmtKind::If {
                cond: mock_expr.clone(),
                if_true: Box::new(ret_stmt.clone()),
                if_false: Some(Box::new(ret_stmt.clone())),
            },
            position: SourcePosition(0, 0),
        };

        let root_if = Stmt {
            kind: StmtKind::If {
                cond: mock_expr.clone(),
                if_true: Box::new(ret_stmt.clone()),
                if_false: Some(Box::new(nested_if)),
            },
            position: SourcePosition(0, 0),
        };

        assert!(ReturnChecker::stmt_guarantees_return(&root_if));
    }

    #[test]
    fn test_while_true_return() {
        let mock_expr_true = Box::new(Expr {
            kind: ExprKind::LiteralTrue,
            position: SourcePosition(0, 0),
        });
        let loop_stmt = Stmt {
            kind: StmtKind::While {
                cond: mock_expr_true,
                body: Box::new(Stmt {
                    kind: StmtKind::Empty,
                    position: SourcePosition(0, 0),
                }),
            },
            position: SourcePosition(0, 0),
        };
        assert!(ReturnChecker::stmt_guarantees_return(&loop_stmt));
    }
}
