use super::ty::Type;
use super::{Ident, SourcePosition};

/// EXPRESSIONS ///
#[derive(Debug, PartialEq, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExprKind {
    Var(Ident),
    LiteralInt(i32),
    LiteralTrue,
    LiteralFalse,
    LiteralString(String),
    App {
        callee: Box<Expr>,
        args: Vec<Expr>,
    }, // obj.foo(1, 2)
    Field {
        expr: Box<Expr>,
        field_name: Ident,
    }, // class.field
    Method {
        // class.method(1, 2)
        expr: Box<Expr>,
        method_name: Ident,
        args: Vec<Expr>,
    },
    Paren(Box<Expr>),
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    BinaryOp {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    New(Type),
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOp {
    Neg, // -
    Not, // !
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp {
    // Arithmetic operators
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Mod, // %

    // Comparisons
    Eq,  // ==
    Lt,  // <
    Le,  // <=
    Gt,  // >
    Ge,  // >=
    Neq, // !=

    // Logical
    And, // &&
    Or,  // ||
}
