use crate::parser::ast::{Arg, Ident, SourcePosition, Type};
use crate::semantics::tast::statements::TBlock;

#[derive(Debug)]
pub struct TProgram {
    pub defs: Vec<TTopDef>,
}

#[derive(Debug)]
pub struct TTopDef {
    pub kind: TTopDefKind,
    pub position: SourcePosition,
}

#[derive(Debug)]
pub enum TTopDefKind {
    Func(TFuncDef),
    Class(TClassDef),
}

#[derive(Debug)]
pub struct TFuncDef {
    pub name: Ident,
    pub return_type: Type,
    pub args: Vec<Arg>,
    pub body: TBlock,
    pub position: SourcePosition,
}

#[derive(Debug)]
pub struct TClassDef {
    pub name: Ident,
    pub extends: Option<Ident>,
    pub entries: Vec<TClassEntry>,
    pub position: SourcePosition,
}

#[derive(Debug)]
pub struct TClassEntry {
    pub kind: TClassEntryKind,
    pub position: SourcePosition,
}

#[derive(Debug)]
pub enum TClassEntryKind {
    Field(Vec<Ident>, Type),
    Method(TFuncDef),
}
