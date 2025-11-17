// Exports from the parser module.
pub mod ast;
pub mod lexer;
pub mod tokens;
pub mod utils;

use crate::diagnostics::CompilerError;
use crate::latte::ProgramParser;
use crate::parser::lexer::LatteLexer;
use crate::parser::tokens::Token;

/// Main function performing the parsing:
/// - Initializes the lexer and lexes the code.
/// - Filters out the Comment tokens, since they are parsed as tokens.
/// - Parses the code using lalrpop and returns the parsed AST.
pub fn parse(source_code: &str) -> Result<ast::Program, CompilerError> {
    let lexer = LatteLexer::new(source_code);

    let filtered_iterator = lexer.filter_map(|item| match item {
        Ok((_start, Token::Comment, _end)) => None,
        Ok((start, token, end)) => Some(Ok((start, token, end))),
        Err(lexical_error) => Some(Err(lexical_error)),
    });

    let parser = ProgramParser::new();
    let program = parser.parse(filtered_iterator)?;

    Ok(program)
}
