use super::diagnostics::{Diagnostic, ToDiagnostic};
use super::lexical::{LexicalError, LocatedLexicalError};
use crate::parser::tokens::Token;
use lalrpop_util::ParseError as LalrpopError;

#[derive(Debug, Clone)]
pub enum ParseError {
    InvalidToken {
        location: usize,
    },
    UnrecognizedEof {
        location: usize,
        expected: Vec<String>,
    },
    UnrecognizedToken {
        start: usize,
        end: usize,
        token: Token,
        expected: Vec<String>,
    },
    ExtraToken {
        start: usize,
        end: usize,
        token: Token,
    },
    Lexical {
        start: usize,
        end: usize,
        error: LexicalError,
    },
}

impl From<LalrpopError<usize, Token, (LexicalError, std::ops::Range<usize>)>> for ParseError {
    fn from(err: LalrpopError<usize, Token, (LexicalError, std::ops::Range<usize>)>) -> Self {
        match err {
            LalrpopError::InvalidToken { location } => ParseError::InvalidToken { location },
            LalrpopError::UnrecognizedEof { location, expected } => {
                ParseError::UnrecognizedEof { location, expected }
            }
            LalrpopError::UnrecognizedToken {
                token: (start, token, end),
                expected,
            } => ParseError::UnrecognizedToken {
                start,
                end,
                token,
                expected,
            },
            LalrpopError::ExtraToken {
                token: (start, token, end),
            } => ParseError::ExtraToken { start, end, token },
            LalrpopError::User { error: (err, span) } => ParseError::Lexical {
                start: span.start,
                end: span.end,
                error: err,
            },
        }
    }
}

fn format_expected(expected: &[String]) -> String {
    if expected.is_empty() {
        "nothing".to_string()
    } else {
        expected.join(", ")
    }
}

impl ToDiagnostic for ParseError {
    fn to_diagnostic(&self) -> Diagnostic {
        match self {
            ParseError::InvalidToken { location } => {
                Diagnostic::error("Invalid token").label(*location..*location, "Unexpected here")
            }

            ParseError::UnrecognizedEof { location, expected } => {
                Diagnostic::error("Unexpected end of file")
                    .label(*location..*location, "File ends here")
                    .note(format!("Expected one of: {}", format_expected(expected)))
            }

            ParseError::UnrecognizedToken {
                start,
                end,
                token,
                expected,
            } => Diagnostic::error(format!("Unexpected token '{}'", token))
                .label(*start..*end, "Not expected here")
                .note(format!("Expected one of: {}", format_expected(expected))),

            ParseError::ExtraToken { start, end, token } => {
                Diagnostic::error(format!("Extra token '{}'", token))
                    .label(*start..*end, "Remove this")
            }

            ParseError::Lexical { start, end, error } => LocatedLexicalError {
                error: error.clone(),
                span: *start..*end,
            }
            .to_diagnostic(),
        }
    }
}
