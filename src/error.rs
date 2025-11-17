/// Main file for all errors detected at the compilation stage.
use crate::parser::ast::SourcePosition;
use crate::parser::tokens::{LexicalError, Token};
use lalrpop_util::ParseError;
use logos::Span;

/// Errors returned on the lowering stage.
#[derive(Debug)]
pub enum SemanticError {
    UndefinedVariable {
        name: String,
        position: SourcePosition,
    },
    ZeroDivision {
        position: SourcePosition,
    },
}

#[derive(Debug)]
pub enum CompilerError {
    /// Errors regarding I/O operations (reading files, creating directories, etc.)
    Io(std::io::Error),
    /// Syntax errors detected at the parsing stage.
    Parsing(ParseError<usize, Token, LexicalError>),
    /// Semantic errors detected at the lowering stage (undefined variables, zero division, etc.)
    Semantic(SemanticError),
    /// Other, uncategorized errors.
    Other(String),
    ExternalCommand(String),
}

impl From<std::io::Error> for CompilerError {
    fn from(err: std::io::Error) -> Self {
        CompilerError::Io(err)
    }
}

impl From<ParseError<usize, Token, LexicalError>> for CompilerError {
    fn from(err: ParseError<usize, Token, LexicalError>) -> Self {
        CompilerError::Parsing(err)
    }
}

impl From<SemanticError> for CompilerError {
    fn from(err: SemanticError) -> Self {
        CompilerError::Semantic(err)
    }
}

impl From<&str> for CompilerError {
    fn from(err: &str) -> Self {
        CompilerError::Other(err.to_string())
    }
}

use std::fmt;
impl fmt::Display for CompilerError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerError::Io(err) => write!(fmt, "I/O Error: {}", err),
            CompilerError::Parsing(_) => write!(fmt, "Parsing error"),
            CompilerError::Semantic(_) => write!(fmt, "Semantic Error"),
            CompilerError::Other(msg) => write!(fmt, "Error: {}", msg),
            CompilerError::ExternalCommand(msg) => write!(fmt, "External Command Failed: {}", msg),
        }
    }
}
