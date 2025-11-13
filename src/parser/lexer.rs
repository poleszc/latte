// Custom lexer implementation using logos.
use crate::parser::tokens::{LexicalError, Token};
use logos::{Logos, SpannedIter};

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

pub struct MyLexer<'input> {
    token_stream: SpannedIter<'input, Token>,
}

impl<'input> MyLexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            token_stream: Token::lexer(input).spanned(),
        }
    }
}

impl<'input> Iterator for MyLexer<'input> {
    type Item = Spanned<Token, usize, LexicalError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.token_stream
            .next()
            .map(|(token, span)| Ok((span.start, token?, span.end)))
    }
}
