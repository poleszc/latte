// Exports from the parser module.
pub mod ast;
mod lexer;
pub mod tokens;

use crate::error::CompilerError;
use crate::instant::ProgramParser;
use lexer::MyLexer;

/// Main function performing the parsing. Initializes the lexer and the parser and returns the parsed AST.
pub fn parse(source_code: &str) -> Result<ast::Program, CompilerError> {
    let lexer = MyLexer::new(source_code);
    let parser = ProgramParser::new();
    let program = parser.parse(lexer)?;

    Ok(program)
}
