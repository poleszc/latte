// File containing the token definitions for the custom lexer implementation.
use logos::Logos;
use std::{fmt, num::ParseIntError};

#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    InvalidInteger(ParseIntError),
    #[default]
    InvalidToken,
}

impl fmt::Display for LexicalError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexicalError::InvalidInteger(err) => write!(fmt, "Invalid integer literal: {}", err),
            LexicalError::InvalidToken => write!(fmt, "Invalid token"),
        }
    }
}

impl From<ParseIntError> for LexicalError {
    fn from(err: ParseIntError) -> Self {
        LexicalError::InvalidInteger(err)
    }
}

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f\r]+", error = LexicalError)]
pub enum Token {
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),
    #[regex(r"[0-9]+", |lex| lex.slice().parse())]
    Integer(i32),

    #[token("=")]
    Assign,
    #[token(";")]
    Semicolon,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,

    #[token("+")]
    OperatorAdd,
    #[token("-")]
    OperatorSub,
    #[token("*")]
    OperatorMul,
    #[token("/")]
    OperatorDiv,
}

impl fmt::Display for Token {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Identifier(s) => write!(fmt, "identifier '{}'", s),
            Token::Integer(i) => write!(fmt, "integer '{}'", i),
            Token::Assign => write!(fmt, "="),
            Token::Semicolon => write!(fmt, ";"),
            Token::LParen => write!(fmt, "("),
            Token::RParen => write!(fmt, ")"),
            Token::OperatorAdd => write!(fmt, "+"),
            Token::OperatorSub => write!(fmt, "-"),
            Token::OperatorMul => write!(fmt, "*"),
            Token::OperatorDiv => write!(fmt, "/"),
        }
    }
}
