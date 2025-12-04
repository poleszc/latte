use crate::{
    SourcePosition,
    parser::ast::{BinaryOp, Ident, Type, UnaryOp},
};

#[derive(Debug, Clone)]
pub struct TExpr {
    pub kind: TExprKind,
    pub position: SourcePosition,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub enum TExprKind {
    Var(Ident),
    LiteralInt(i32),
    LiteralTrue,
    LiteralFalse,
    This,
    LiteralString(String),
    LiteralNull,
    App {
        callee: Box<TExpr>,
        args: Vec<TExpr>,
    },
    ArrayAccess {
        array: Box<TExpr>,
        index: Box<TExpr>,
    },
    FieldAccess {
        object: Box<TExpr>,
        field_name: Ident,
    },
    Method {
        object: Box<TExpr>,
        method_name: Ident,
        args: Vec<TExpr>,
    },
    Paren(Box<TExpr>),
    UnaryOp {
        op: UnaryOp,
        expr: Box<TExpr>,
    },
    BinaryOp {
        op: BinaryOp,
        lhs: Box<TExpr>,
        rhs: Box<TExpr>,
    },
    NewObject(Type),
    NewArray {
        element_type: Type,
        size: Box<TExpr>,
    },
    TypedNull(Type),
    // TODO: Maybe remove that?
    ExprTypedNull(Box<TExpr>),
}
