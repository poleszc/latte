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
    Field(Vec<Ident>, Type),
    Method(FuncDef),
}

impl Program {
    pub fn dump_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let json_string = serde_json::to_string_pretty(&self)?;
        let mut file = File::create(path)?;
        write!(file, "{}", json_string)?;

        Ok(())
    }
}
