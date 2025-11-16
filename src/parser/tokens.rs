// File containing the token definitions for the custom lexer implementation.
use logos::Logos;
use std::{fmt, num::ParseIntError};

#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    InvalidInteger(ParseIntError),
    InvalidString,
    #[default]
    InvalidToken,
}

impl fmt::Display for LexicalError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexicalError::InvalidInteger(err) => write!(fmt, "Invalid integer literal: {}", err),
            LexicalError::InvalidString => write!(fmt, "Invalid string literal"),
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
#[logos(skip r"[ \t\n\f\r]+", error = LexicalError)] // Skip whitespaces.
#[logos(skip r"//[^\n]*")] // Skip one line comments.
#[logos(skip r"/\*([^*]|\*+[^*/])*\*+/")] // Skip multiline comments.
pub enum Token {
    // Keywords.
    #[token("class")]
    KWClass,
    #[token("extends")]
    KWExtends,
    #[token("int")]
    KWInt,
    #[token("string")]
    KWString,
    #[token("boolean")]
    KWBool,
    #[token("void")]
    KWVoid,
    #[token("if")]
    KWIf,
    #[token("else")]
    KWElse,
    #[token("while")]
    KWWhile,
    #[token("for")]
    KWFor,
    #[token("return")]
    KWReturn,
    #[token("true")]
    KWTrue,
    #[token("false")]
    KWFalse,
    #[token("new")]
    KWNew,
    #[token("null")]
    KWNull,

    // Operators.
    #[token("+")]
    OPAdd,
    #[token("-")]
    OPSub,
    #[token("*")]
    OPMul,
    #[token("/")]
    OPDiv,
    #[token("%")]
    OPMod,
    #[token("<")]
    OPLt,
    #[token("<=")]
    OPLe,
    #[token(">")]
    OPGt,
    #[token(">=")]
    OPGe,
    #[token("==")]
    OPEq,
    #[token("!=")]
    OPNeq,
    #[token("&&")]
    OPAnd,
    #[token("||")]
    OPOr,
    #[token("!")]
    OPNot,
    #[token("=")]
    OPAssign,
    #[token("++")]
    OPIncr,
    #[token("--")]
    OPDecr,

    // Misc.
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    
    // Identifiers and literals.
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),
    
    #[regex(r"[0-9]+", |lex| lex.slice().parse())]
    LiteralInteger(i32),
    
    #[regex(r#""([^"\\]|\\.)*""#, string_literal)]
    LiteralString(String),
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
