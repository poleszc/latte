/// Utilities for the parsing stage. Mainly functions used by the lalrpop parser.
use crate::SourcePosition;
use crate::parser::ast;
#[inline]
pub fn make_binary(op: ast::BinaryOp, lhs: Box<ast::Expr>, rhs: Box<ast::Expr>) -> Box<ast::Expr> {
    Box::new(ast::Expr {
        position: SourcePosition(lhs.position.0, rhs.position.1),
        kind: ast::ExprKind::BinaryOp { op, lhs, rhs },
    })
}

#[inline]
pub fn make_unary(
    start: usize,
    op: ast::UnaryOp,
    expr: Box<ast::Expr>,
    end: usize,
) -> Box<ast::Expr> {
    Box::new(ast::Expr {
        position: SourcePosition(start, end),
        kind: ast::ExprKind::UnaryOp { op, expr },
    })
}
