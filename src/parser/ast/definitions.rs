use serde::Serialize;

use super::statements::Block;
use super::types::Type;
use super::{Ident, SourcePosition};

/// PROGRAM ///
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Program {
    pub defs: Vec<TopDef>,
    pub position: SourcePosition,
}

/// TOP LEVEL DEFINITIONS ///
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct TopDef {
    pub kind: TopDefKind,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum TopDefKind {
    Func(FuncDef),
    Class(ClassDef),
}

/// FUNCTION ///
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct FuncDef {
    pub name: Ident,
    pub return_type: Type,
    pub args: Vec<Arg>,
    pub body: Block,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Arg {
    pub arg_type: Type,
    pub name: Ident,
    pub position: SourcePosition,
}

/// CLASSES ///
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct ClassDef {
    pub name: Ident,
    pub extends: Option<Ident>,
    pub entries: Vec<ClassEntry>, // Entry -> field or method.
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct ClassEntry {
    pub kind: ClassEntryKind,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum ClassEntryKind {
    Field(Type, Ident),
    Method(FuncDef),
}