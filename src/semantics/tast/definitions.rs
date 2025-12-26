use serde::Serialize;

use crate::parser::ast::{Arg, Ident, SourcePosition, Type};
use crate::semantics::tast::statements::TBlock;

#[derive(Debug, Serialize)]
pub struct TProgram {
    pub defs: Vec<TTopDef>,
}

#[derive(Debug, Serialize)]
pub struct TTopDef {
    pub kind: TTopDefKind,
    pub position: SourcePosition,
}

#[derive(Debug, Serialize)]
pub enum TTopDefKind {
    Func(TFuncDef),
    Class(TClassDef),
}

#[derive(Debug, Serialize)]
pub struct TFuncDef {
    pub name: Ident,
    pub return_type: Type,
    pub args: Vec<Arg>,
    pub body: TBlock,
    pub position: SourcePosition,
}

#[derive(Debug, Serialize)]
pub struct TClassDef {
    pub name: Ident,
    pub extends: Option<Ident>,
    pub entries: Vec<TClassEntry>,
    pub position: SourcePosition,
}

#[derive(Debug, Serialize)]
pub struct TClassEntry {
    pub kind: TClassEntryKind,
    pub position: SourcePosition,
}

#[derive(Debug, Serialize)]
pub enum TClassEntryKind {
    Field(Vec<Ident>, Type),
    Method(TFuncDef),
}

impl TProgram {
    pub fn dump_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let json_string = serde_json::to_string_pretty(&self)?;
        let mut file = File::create(path)?;
        write!(file, "{}", json_string)?;

        Ok(())
    }
}
