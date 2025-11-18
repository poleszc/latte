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
        // TODO: Try to remove Vec<Box<_>>
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
impl UnaryOp {
    pub fn to_string(&self) -> String {
        match self {
            UnaryOp::Neg => "-".to_string(),
            UnaryOp::Not => "!".to_string(),
        }
    }
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

impl BinaryOp {
    pub fn to_string(&self) -> String {
        match self {
            BinaryOp::Add => "+".to_string(),
            BinaryOp::Sub => "-".to_string(),
            BinaryOp::Mul => "*".to_string(),
            BinaryOp::Div => "/".to_string(),
            BinaryOp::Mod => "%".to_string(),
            BinaryOp::Eq => "==".to_string(),
            BinaryOp::Neq => "!=".to_string(),
            BinaryOp::Lt => "<".to_string(),
            BinaryOp::Le => "<=".to_string(),
            BinaryOp::Gt => ">".to_string(),
            BinaryOp::Ge => ">=".to_string(),
            BinaryOp::And => "&&".to_string(),
            BinaryOp::Or => "||".to_string(),
        }
    }
}
