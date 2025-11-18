use serde::Serialize;

use super::types::Type;
use super::{Ident, SourcePosition};

/// EXPRESSIONS ///
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum ExprKind {
    Var(Ident),
    LiteralInt(i32),
    LiteralTrue,
    LiteralFalse,
    LiteralString(String),
    LiteralNull,
    App {
        callee: Box<Expr>,
        args: Vec<Box<Expr>>,
    }, // obj.foo(1, 2)
    ArrayAccess {
        array: Box<Expr>,
        index: Box<Expr>,
    }, // arr [i]
    FieldAccess {
        object: Box<Expr>,
        field_name: Ident,
    }, // class.field
    Method {
        object: Box<Expr>,
        method_name: Ident,
        args: Vec<Expr>,
    }, // class.method(1, 2)
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
    NewObject(Type), // new Animal
    NewArray {
        element_type: Type,
        size: Box<Expr>,
    }, // new int[10]
    TypedNull(Type),
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum UnaryOp {
    Neg, // -
    Not, // !
}

#[derive(Debug, PartialEq, Clone, Serialize)]
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
