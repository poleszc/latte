pub mod diagnostics;
pub mod lexical;
pub mod parsing;
pub mod semantic;

use diagnostics::ToDiagnostic;
pub use lexical::LexicalError;
pub use parsing::ParseError;
pub use semantic::SemanticError;

use crate::parser::tokens::Token;
use lalrpop_util::ParseError as LalrpopError;
use std::ops::Range;

#[derive(Debug)]
pub enum CompilerError {
    Io(std::io::Error),
    Lexical(LexicalError, usize, usize),
    Parsing(ParseError),
    Semantic(SemanticError),
    Other(String),
    Silent,
}

impl From<std::io::Error> for CompilerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<ParseError> for CompilerError {
    fn from(e: ParseError) -> Self {
        Self::Parsing(e)
    }
}
impl From<SemanticError> for CompilerError {
    fn from(e: SemanticError) -> Self {
        Self::Semantic(e)
    }
}

impl From<LalrpopError<usize, Token, (LexicalError, Range<usize>)>> for CompilerError {
    fn from(err: LalrpopError<usize, Token, (LexicalError, Range<usize>)>) -> Self {
        let parse_error = ParseError::from(err);
        CompilerError::Parsing(parse_error)
    }
}

pub fn report_compiler_error(file_path: &str, source: &str, error: &CompilerError) {
    match error {
        CompilerError::Parsing(e) => e.to_diagnostic().render(file_path, source),
        CompilerError::Semantic(e) => e.to_diagnostic().render(file_path, source),
        CompilerError::Lexical(err, start, end) => {
            lexical::LocatedLexicalError {
                error: err.clone(),
                span: *start..*end,
            }
            .to_diagnostic()
            .render(file_path, source);
        }
        CompilerError::Io(e) => eprintln!("I/O Error: {}", e),
        CompilerError::Other(msg) => eprintln!("Error: {}", msg),
        CompilerError::Silent => {}
    }
}
