use std::fmt;

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
    Array(Box<Type>),
    Fun(Box<Type>, Vec<Type>),
    Error,
}

impl fmt::Display for TypeKind {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Int => write!(fmt, "int"),
            TypeKind::String => write!(fmt, "string"),
            TypeKind::Bool => write!(fmt, "bool"),
            TypeKind::Void => write!(fmt, "void"),
            TypeKind::Null => write!(fmt, "null"),
            TypeKind::Class(ident) => write!(fmt, "{}", ident.name),
            TypeKind::Array(inner) => write!(fmt, "{}[]", inner),
            TypeKind::Fun(ret, params) => {
                let param_str = params
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(fmt, "({}) -> {}", param_str, ret)
            }
            TypeKind::Error => write!(fmt, "error"),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Type {
    pub kind: TypeKind,
    pub position: SourcePosition,
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}
