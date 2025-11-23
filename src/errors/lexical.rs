use super::diagnostics::{Diagnostic, ToDiagnostic};
use std::num::ParseIntError;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LexicalError {
    InvalidInteger(String),
    InvalidString,
    UnterminatedComment,
    #[default]
    InvalidToken,
}

impl From<ParseIntError> for LexicalError {
    fn from(err: ParseIntError) -> Self {
        LexicalError::InvalidInteger(err.to_string())
    }
}

pub struct LocatedLexicalError {
    pub error: LexicalError,
    pub span: Range<usize>,
}

impl ToDiagnostic for LocatedLexicalError {
    fn to_diagnostic(&self) -> Diagnostic {
        let span = self.span.clone();
        match &self.error {
            LexicalError::InvalidInteger(err) => {
                Diagnostic::error(format!("Invalid integer literal: {}", err))
                    .label(span, "Cannot parse as integer")
                    .help("Check range (-2^31 to 2^31-1)")
            }

            LexicalError::InvalidString => Diagnostic::error("Invalid string literal")
                .label(span, "Malformed string")
                .help("Add closing quote"),

            LexicalError::UnterminatedComment => Diagnostic::error("Unterminated comment")
                .label(span, "Never closed")
                .help("Add */"),

            LexicalError::InvalidToken => {
                Diagnostic::error("Invalid token").label(span, "Unrecognized character sequence")
            }
        }
    }
}
