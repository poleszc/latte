// File containing the token definitions for the custom lexer implementation.
use logos::Logos;
use std::{fmt::{self, write}, num::ParseIntError};

#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    InvalidInteger(ParseIntError),
    InvalidString,
    UnterminatedComment,
    #[default]
    InvalidToken,
}

impl fmt::Display for LexicalError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexicalError::InvalidInteger(err) => write!(fmt, "Invalid integer literal: {}", err),
            LexicalError::InvalidString => write!(fmt, "Unterminated or invalid string literal"),
            LexicalError::InvalidToken => write!(fmt, "Invalid token"),
            LexicalError::UnterminatedComment => write!(fmt, "Unterminated multi-line comment"),
        }
    }
}

impl From<ParseIntError> for LexicalError {
    fn from(err: ParseIntError) -> Self {
        LexicalError::InvalidInteger(err)
    }
}

fn string_literal(lex: &mut logos::Lexer<Token>) -> Result<String, LexicalError> {
    let slice = lex.slice();
    let inner = &slice[1..slice.len() - 1];

    let mut s = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => s.push('\n'),
                Some('t') => s.push('\t'),
                Some('r') => s.push('\r'),
                Some('\\') => s.push('\\'),
                Some('"') => s.push('"'),
                Some(_) => {
                    return Err(LexicalError::InvalidString);
                }
                None => return Err(LexicalError::InvalidString),
            }
        } else {
            s.push(c);
        }
    }
    Ok(s)
}

fn skip_multiline_comment(lex: &mut logos::Lexer<Token>) -> Result<(), LexicalError> {
    let remainder = lex.remainder();
    
    match remainder.find("*/") {
        Some(end_pos) => {
            lex.bump(end_pos + 2);
            Ok(())
        }
        None => {
            Err(LexicalError::UnterminatedComment)
        }
    }
}

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f\r]+", error = LexicalError)] // Skip whitespaces.
#[logos(skip r"//[^\n]*")] // Skip one line comments.
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
    #[token("/*", skip_multiline_comment)]
    Comment,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Keywords
            Token::KWClass => write!(f, "'class'"),
            Token::KWExtends => write!(f, "'extends'"),
            Token::KWInt => write!(f, "'int'"),
            Token::KWString => write!(f, "'string'"),
            Token::KWBool => write!(f, "'boolean'"),
            Token::KWVoid => write!(f, "'void'"),
            Token::KWIf => write!(f, "'if'"),
            Token::KWElse => write!(f, "'else'"),
            Token::KWWhile => write!(f, "'while'"),
            Token::KWFor => write!(f, "'for'"),
            Token::KWReturn => write!(f, "'return'"),
            Token::KWTrue => write!(f, "'true'"),
            Token::KWFalse => write!(f, "'false'"),
            Token::KWNew => write!(f, "'new'"),
            Token::KWNull => write!(f, "'null'"),

            // Operators
            Token::OPAdd => write!(f, "'+'"),
            Token::OPSub => write!(f, "'-'"),
            Token::OPMul => write!(f, "'*'"),
            Token::OPDiv => write!(f, "'/'"),
            Token::OPMod => write!(f, "'%'"),
            Token::OPLt => write!(f, "'<'"),
            Token::OPLe => write!(f, "'<='"),
            Token::OPGt => write!(f, "'>'"),
            Token::OPGe => write!(f, "'>='"),
            Token::OPEq => write!(f, "'=='"),
            Token::OPNeq => write!(f, "'!='"),
            Token::OPAnd => write!(f, "'&&'"),
            Token::OPOr => write!(f, "'||'"),
            Token::OPNot => write!(f, "'!'"),
            Token::OPAssign => write!(f, "'='"),
            Token::OPIncr => write!(f, "'++'"),
            Token::OPDecr => write!(f, "'--'"),

            // Misc
            Token::LParen => write!(f, "'('"),
            Token::RParen => write!(f, "')'"),
            Token::LBrace => write!(f, "'{{'"),
            Token::RBrace => write!(f, "'}}'"),
            Token::LBracket => write!(f, "'['"),
            Token::RBracket => write!(f, "']'"),
            Token::Semicolon => write!(f, "';'"),
            Token::Colon => write!(f, "':'"),
            Token::Comma => write!(f, "','"),
            Token::Dot => write!(f, "'.'"),

            // Identifiers and Literals
            Token::Identifier(s) => write!(f, "identifier '{}'", s),
            Token::LiteralInteger(i) => write!(f, "integer literal '{}'", i),
            Token::LiteralString(s) => write!(f, "string literal \"{}\"", s.escape_default()),
            Token::Comment => write!(f, "Comment")
        }
    }
}
