use crate::{
    SourcePosition,
    parser::ast::{Ident, Type},
    utils::StrID,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub ident: Ident,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub ident: Ident,
    pub return_type: Type,
    pub params: Vec<Type>,
    pub position: SourcePosition,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub ident: Ident,
    pub parent: Option<Ident>,
    pub fields: HashMap<StrID, (Type, SourcePosition)>,
    pub methods: HashMap<StrID, FunctionInfo>,
    pub field_order: Vec<StrID>,
    pub method_order: Vec<StrID>,
    pub position: SourcePosition,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Variable(VariableInfo),
    Function(FunctionInfo),
}

impl SymbolKind {
    /// Returns a source position where the symbol was defined.
    pub fn get_source_position(&self) -> SourcePosition {
        match self {
            SymbolKind::Variable(info) => info.ident.position,
            SymbolKind::Function(info) => info.ident.position,
        }
    }
}
