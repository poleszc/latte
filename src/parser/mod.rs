// Exports from the parser module.
pub mod ast;
pub mod lexer;
pub mod tokens;
pub mod utils;

use crate::error::CompilerError;
use crate::latte::ProgramParser;
use lexer::Lexer;

/// Main function performing the parsing. Initializes the lexer and the parser and returns the parsed AST.
pub fn parse(source_code: &str) -> Result<ast::Program, CompilerError> {
    let lexer = Lexer::new(source_code);
    let parser = ProgramParser::new();
    let program = parser.parse(lexer)?;

    Ok(program)
}
