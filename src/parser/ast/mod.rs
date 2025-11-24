/// Main definition of the whole AST.
pub mod definitions;
pub mod expressions;
pub mod statements;
pub mod types;

pub use definitions::*;
pub use expressions::*;
use serde::Serialize;
pub use statements::*;
pub use types::*;

use crate::utils::StrID;

/// COMMON AST TYPES ///
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Serialize)]
pub struct SourcePosition(pub usize, pub usize);

impl From<(usize, usize)> for SourcePosition {
    fn from(value: (usize, usize)) -> Self {
        SourcePosition(value.0, value.1)
    }
}

/// Identifier
#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize)]
pub struct Ident {
    pub name: StrID,
    pub position: SourcePosition,
}
