use super::expressions::Expr;
use super::types::Type;
use super::{Ident, SourcePosition};

/// STATEMENTS ///
#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StmtKind {
    Empty,        // ;
    Block(Block), // {}
    // TODO: Add consts?
    Decl(Type, Vec<Item>), // int x, y = 5;
    Assign {
        // object.field = 5;
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Return {
        expr: Option<Box<Expr>>,
    },
    Incr(Ident),
    Decr(Ident),
    If {
        cond: Box<Expr>,
        if_true: Box<Stmt>,
        if_false: Option<Box<Stmt>>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Stmt>,
    },
    // TODOP: Maybe remove that?
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Box<Expr>>,
        post: Option<Box<Expr>>,
        body: Box<Stmt>,
    },
    Expr(Box<Expr>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Item {
    pub kind: ItemKind,
    pub position: SourcePosition,
}
#[derive(Debug, PartialEq, Clone)]
pub enum ItemKind {
    NoInit(Ident),
    Init(Ident, Box<Expr>),
}