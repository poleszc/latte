use serde::Serialize;

use super::expressions::Expr;
use super::types::Type;
use super::{Ident, SourcePosition};

/// STATEMENTS ///
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Stmt {
    pub kind: StmtKind,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum StmtKind {
    Empty,                 // ;
    Block(Block),          // {}
    Decl(Type, Vec<Item>), // int x, y = 5;
    Assign {
        // object.field = 5;
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Return {
        expr: Option<Box<Expr>>,
    },
    Incr(Box<Expr>),
    Decr(Box<Expr>),
    If {
        cond: Box<Expr>,
        if_true: Box<Stmt>,
        if_false: Option<Box<Stmt>>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Stmt>,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Box<Expr>>,
        post: Option<Box<Expr>>,
        body: Box<Stmt>,
    },
    ForEach {
        element_type: Type,
        item_name: Ident,
        collection: Box<Expr>,
        body: Box<Stmt>,
    },
    Expr(Box<Expr>),
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Item {
    pub kind: ItemKind,
    pub position: SourcePosition,
}
#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum ItemKind {
    NoInit(Ident),
    Init(Ident, Box<Expr>),
}
