use super::{Ident, SourcePosition};

/// TYPES ///
#[derive(Debug, PartialEq, Clone)]
pub struct Type {
    pub kind: TypeKind,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TypeKind {
    Int,
    String,
    Bool,
    Void,
    Class(Ident),
    // TODO: Think about that. Maybe it will be used internally by the type checker.
    Fun(Box<Type>, Vec<Type>),
}
