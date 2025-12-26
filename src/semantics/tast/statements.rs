use serde::Serialize;

use crate::{
    SourcePosition,
    parser::ast::{Ident, Type},
    semantics::tast::expressions::TExpr,
};

#[derive(Debug, Serialize)]
pub struct TBlock {
    pub stmts: Vec<TStmt>,
    pub position: SourcePosition,
}

#[derive(Debug, Serialize)]
pub struct TStmt {
    pub kind: TStmtKind,
    pub position: SourcePosition,
}

#[derive(Debug, Serialize)]
pub enum TStmtKind {
    Empty,
    Block(TBlock),
    Decl(Type, Vec<TItem>),
    Assign {
        target: TExpr,
        value: TExpr,
    },
    Return {
        expr: Option<TExpr>,
    },
    Incr(TExpr),
    Decr(TExpr),
    If {
        cond: TExpr,
        if_true: Box<TStmt>,
        if_false: Option<Box<TStmt>>,
    },
    While {
        cond: TExpr,
        body: Box<TStmt>,
    },
    ForEach {
        element_type: Type,
        item_name: Ident,
        collection: TExpr,
        body: Box<TStmt>,
    },
    Expr(TExpr),
}

#[derive(Debug, Serialize)]
pub struct TItem {
    pub kind: TItemKind,
    pub position: SourcePosition,
}

#[derive(Debug, Serialize)]
pub enum TItemKind {
    NoInit(Ident),
    Init(Ident, TExpr),
}
