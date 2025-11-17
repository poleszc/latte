use crate::diagnostics::LexicalError;
use crate::parser::tokens::Token;
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
    type Item = Result<(usize, Token, usize), (LexicalError, std::ops::Range<usize>)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lexer.next() {
            Some(Ok(token)) => {
                let span = self.lexer.span();
                Some(Ok((span.start, token, span.end)))
            }
            Some(Err(error)) => {
                let span = self.lexer.span();
                Some(Err((error, span)))
            }
            None => None,
        }
    }
}
