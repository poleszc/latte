use serde::Serialize;

use super::{Ident, SourcePosition};

/// TYPES ///
#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum TypeKind {
    Int,
    String,
    Bool,
    Void,
    Null,
    Class(Ident),
    Array(Box<Type>), // TODO: Box?
    // TODO: Think about that. Maybe it will be used internally by the type checker.
    Fun(Box<Type>, Vec<Type>),
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Type {
    pub kind: TypeKind,
    pub position: SourcePosition,
}


impl Type {
    pub fn to_string(&self) -> String {
        match &self.kind {
            TypeKind::Int => "int".to_string(),
            TypeKind::String => "string".to_string(),
            TypeKind::Bool => "bool".to_string(),
            TypeKind::Void => "void".to_string(),
            // TODO: Think about that. Null is not a type
            TypeKind::Null => "null".to_string(),
            TypeKind::Class(ident) => ident.name.clone(),
            TypeKind::Array(inner) => format!("{}[]", inner.to_string()),
            TypeKind::Fun(ret, params) => {
                let param_str = params
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("({}) -> {}", param_str, ret.to_string())
            }
        }
    }
}
