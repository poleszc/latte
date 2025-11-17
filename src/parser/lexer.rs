// src/parser/lexer.rs
use crate::parser::tokens::{LexicalError, Token};
use logos::Logos;

pub struct LatteLexer<'input> {
    lexer: logos::Lexer<'input, Token>,
}

impl<'input> LatteLexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            lexer: Token::lexer(input),
        }
    }
}

impl<'input> Iterator for LatteLexer<'input> {
    type Item = Result<(usize, Token, usize), LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lexer.next() {
            Some(Ok(token)) => {
                let span = self.lexer.span();
                Some(Ok((span.start, token, span.end)))
            }
            Some(Err(error)) => {
                let span = self.lexer.span();
                eprintln!(
                    "LEXICAL ERROR: {:?} at span {:?} for slice '{}'",
                    error,
                    span,
                    self.lexer.slice()
                );
                Some(Err(error))
            }
            None => None,
        }
    }
}
