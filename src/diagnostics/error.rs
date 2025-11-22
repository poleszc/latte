/// Main file for all errors detected at the compilation stage.
use crate::parser::ast::SourcePosition;
use crate::parser::tokens::Token;
use ariadne::Source;
use lalrpop_util::ParseError as LalrpopParseError;
use logos::source;
use std::fmt;
use std::num::ParseIntError;

// =================================
// Lexical Errors
// =================================
#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    InvalidInteger(String),
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
        LexicalError::InvalidInteger(err.to_string())
    }
}

// =================================
// Parse Errors
// =================================
#[derive(Debug, Clone)]
pub enum ParseError {
    InvalidToken {
        location: usize,
    },
    UnrecognizedEof {
        location: usize,
        expected: Vec<String>,
    },
    UnrecognizedToken {
        start: usize,
        end: usize,
        token: Token,
        expected: Vec<String>,
    },
    ExtraToken {
        start: usize,
        end: usize,
        token: Token,
    },
    LexicalError {
        start: usize,
        end: usize,
        error: LexicalError,
    },
}

impl From<LalrpopParseError<usize, Token, (LexicalError, std::ops::Range<usize>)>> for ParseError {
    fn from(err: LalrpopParseError<usize, Token, (LexicalError, std::ops::Range<usize>)>) -> Self {
        match err {
            LalrpopParseError::InvalidToken { location } => ParseError::InvalidToken { location },
            LalrpopParseError::UnrecognizedEof { location, expected } => {
                ParseError::UnrecognizedEof { location, expected }
            }
            LalrpopParseError::UnrecognizedToken { token, expected } => {
                let (start, tok, end) = token;
                ParseError::UnrecognizedToken {
                    start,
                    end,
                    token: tok,
                    expected,
                }
            }
            LalrpopParseError::ExtraToken { token } => {
                let (start, tok, end) = token;
                ParseError::ExtraToken {
                    start,
                    end,
                    token: tok,
                }
            }
            LalrpopParseError::User { error } => {
                let (err, span) = error;

                ParseError::LexicalError {
                    start: span.start,
                    end: span.end,
                    error: err,
                }
            }
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidToken { .. } => write!(f, "Invalid token"),
            ParseError::UnrecognizedEof { .. } => write!(f, "Unexpected end of file"),
            ParseError::UnrecognizedToken { token, .. } => {
                write!(f, "Unexpected token '{}'", token)
            }
            ParseError::ExtraToken { token, .. } => write!(f, "Unexpected extra token '{}'", token),
            ParseError::LexicalError { error, .. } => write!(f, "Lexical error: {}", error),
        }
    }
}

// =================================
// Semantic Errors
// =================================
#[derive(Debug, Clone)]
pub enum SemanticError {
    UndefinedVariable {
        name: String,
        position: SourcePosition,
    },
    UndefinedFunction {
        name: String,
        position: SourcePosition,
    },
    UndefinedClass {
        name: String,
        position: SourcePosition,
    },
    UndefinedField {
        class_name: String,
        field_name: String,
        position: SourcePosition,
        class_position: SourcePosition,
    },
    UndefinedMethod {
        class_name: String,
        method_name: String,
        position: SourcePosition,
        class_position: SourcePosition,
    },
    DuplicateVariable {
        name: String,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    DuplicateFunction {
        name: String,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    DuplicateClass {
        name: String,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    DuplicateClassField {
        class_name: String,
        field_name: String,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    DuplicateMethod {
        class_name: String,
        method_name: String,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    TypeMismatch {
        expected: String,
        found: String,
        position: SourcePosition,
    },
    ForEachTypeMismatch {
        variable_type: String,
        collection_item_type: String,
        variable_position: SourcePosition,
        collection_position: SourcePosition,
    },
    ArgumentTypeMismatch {
        expected: String,
        found: String,
        position: SourcePosition,
        function_name: String,
        argument_position: SourcePosition,
    },
    AssignTypeMismatch {
        lhs_type: String,
        rhs_type: String,
        lhs_position: SourcePosition,
        rhs_position: SourcePosition,
    },
    BinaryOpTypeMismatch {
        lhs_type: String,
        lhs_position: SourcePosition,
        rhs_type: String,
        rhs_position: SourcePosition,
    },
    DeclarationTypeMismatch {
        var_name: String,
        expected: String,
        decl_type_position: SourcePosition,
        found: String,
        expr_position: SourcePosition,
    },
    InvalidOperands {
        operator: String,
        operand: String,
        position: SourcePosition,
        type_position: SourcePosition,
    },
    WrongNumberOfArguments {
        name: String,
        expected: usize,
        found: usize,
        call_position: SourcePosition,
        function_position: SourcePosition,
    },
    InvalidReturnValue {
        expected: String,
        ret_type_position: SourcePosition,
        found: String,
        position: SourcePosition,
    },
    InvalidVoidReturn {
        ret_type_position: SourcePosition,
    },
    InvalidLValue {
        position: SourcePosition,
    },
    ZeroDivision {
        position: SourcePosition,
    },
    UndefinedMain,
    WrongMainReturnType {
        expected: String,
        found: String,
        position: SourcePosition,
    },
    WrongNumberOfMainArguments {
        found: usize,
        position: SourcePosition,
    },
    InvalidVariableType {
        found: String,
        expected: Vec<String>,
        position: SourcePosition,
    },
    InvalidFunctionReturnType {
        found: String,
        expected: Vec<String>,
        position: SourcePosition,
    },
    InvalidVoidArrayType {
        expected: Vec<String>,
        position: SourcePosition,
    },
    InvalidNullCast {
        found: String,
        expected: Vec<String>,
        type_position: SourcePosition,
        expr_position: SourcePosition,
    },
    InvalidNullCastExpr {
        expected: Vec<String>,
        cast_position: SourcePosition,
        expr_position: SourcePosition,
    },
    MissingReturn {
        function_name: String,
        position: SourcePosition,
    },
    ThisUsedOutsideClass {
        position: SourcePosition,
    },
    InheritanceCycle {
        // (subclass, superclass, extends_position)
        cycle: Vec<(String, String, SourcePosition)>,
    },
    InvalidOverride {
        method_name: String,
        parent_name: String,
        reason: String,
        position: SourcePosition,
    },
}
// TODO: Maybe not needed. Remove?

impl fmt::Display for SemanticError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticError::UndefinedVariable { name, .. } => {
                write!(fmt, "Undefined variable '{}'", name)
            }
            SemanticError::UndefinedFunction { name, .. } => {
                write!(fmt, "Undefined function '{}'", name)
            }
            SemanticError::UndefinedClass { name, .. } => {
                write!(fmt, "Undefined class '{}'", name)
            }
            SemanticError::UndefinedField {
                class_name,
                field_name,
                ..
            } => {
                write!(
                    fmt,
                    "Undefined field '{}' in class '{}'",
                    field_name, class_name
                )
            }
            SemanticError::UndefinedMethod {
                class_name,
                method_name,
                ..
            } => {
                write!(
                    fmt,
                    "Undefined method '{}' in class '{}'",
                    method_name, class_name
                )
            }
            SemanticError::DuplicateVariable { name, .. } => {
                write!(fmt, "Duplicate variable '{}'", name)
            }
            SemanticError::DuplicateFunction { name, .. } => {
                write!(fmt, "Duplicate function '{}'", name)
            }
            SemanticError::DuplicateClass { name, .. } => {
                write!(fmt, "Duplicate class '{}'", name)
            }
            SemanticError::DuplicateClassField {
                class_name,
                field_name,
                ..
            } => {
                write!(
                    fmt,
                    "Duplicate field '{}' in class '{}'",
                    field_name, class_name
                )
            }
            SemanticError::DuplicateMethod {
                class_name,
                method_name,
                ..
            } => {
                write!(
                    fmt,
                    "Duplicate method '{}' in class '{}'",
                    method_name, class_name
                )
            }
            SemanticError::TypeMismatch {
                expected, found, ..
            } => write!(
                fmt,
                "Mismatched types: expected {}, found {}",
                expected, found
            ),
            SemanticError::DeclarationTypeMismatch {
                expected, found, ..
            } => write!(
                fmt,
                "Mismatched types: expected {}, found {}",
                expected, found
            ),
            SemanticError::InvalidOperands { operator, .. } => {
                write!(fmt, "Invalid operands for operator '{}'", operator)
            }
            SemanticError::WrongNumberOfArguments { name, .. } => {
                write!(fmt, "Wrong number of arguments for function '{}'", name)
            }
            SemanticError::InvalidReturnValue { .. } => {
                write!(fmt, "Invalid return value for function")
            }
            SemanticError::InvalidLValue { .. } => {
                write!(fmt, "Invalid left-hand side of assignment")
            }
            SemanticError::ZeroDivision { .. } => write!(fmt, "Division by a zero literal"),
            SemanticError::UndefinedMain => {
                write!(fmt, "Program doesn't provide a `main` function")
            }
            SemanticError::WrongMainReturnType {
                expected, found, ..
            } => {
                write!(
                    fmt,
                    "Wrong `main` return type: expected '{}', got '{}'",
                    expected, found
                )
            }
            SemanticError::WrongNumberOfMainArguments { found, .. } => {
                write!(
                    fmt,
                    "Wrong `main` argument count: expected '{}', got '{}'",
                    0, found
                )
            }
            SemanticError::InvalidVariableType { found, .. } => {
                write!(fmt, "Invalid variable type: {}", found)
            }
            SemanticError::InvalidFunctionReturnType { found, .. } => {
                write!(fmt, "Invalid function return type: {}", found)
            }
            SemanticError::InvalidVoidArrayType { .. } => {
                write!(fmt, "Invalid void array type")
            }
            SemanticError::InvalidNullCast { found, .. } => {
                write!(fmt, "Invalid null cast to: '{}'", found)
            }
            SemanticError::InvalidNullCastExpr { .. } => {
                write!(fmt, "Null cast to invalid expression")
            }
            SemanticError::MissingReturn { function_name, .. } => {
                write!(
                    fmt,
                    "Function {} doesn't return on all paths",
                    function_name
                )
            }
            SemanticError::ThisUsedOutsideClass { .. } => {
                write!(fmt, "This used outside of class")
            }
            SemanticError::BinaryOpTypeMismatch {
                lhs_type, rhs_type, ..
            } => {
                write!(
                    fmt,
                    "Type mismatch in assignment: cannot assign value of type '{}' to variable of type '{}'",
                    rhs_type, lhs_type
                )
            }
            SemanticError::AssignTypeMismatch {
                lhs_type, rhs_type, ..
            } => {
                write!(
                    fmt,
                    "Type mismatch in binary operation: left side is '{}', right side is '{}'",
                    lhs_type, rhs_type
                )
            }
            SemanticError::InvalidVoidReturn { .. } => {
                write!(
                    fmt,
                    "Invalid return: cannot return an expression of type 'void'"
                )
            }
            SemanticError::ArgumentTypeMismatch {
                expected,
                found,
                function_name,
                ..
            } => {
                write!(
                    fmt,
                    "Type mismatch for argument in call to '{}': expected '{}', found '{}'",
                    function_name, expected, found
                )
            }
            SemanticError::ForEachTypeMismatch {
                variable_type,
                collection_item_type,
                ..
            } => {
                write!(
                    fmt,
                    "Type mismatch in for-each loop: collection contains elements of type '{}', but variable is declared as '{}'",
                    collection_item_type, variable_type
                )
            }
            SemanticError::InheritanceCycle { cycle } => {
                let path = cycle
                    .iter()
                    .map(|(cls, _, _)| cls.as_str())
                    .collect::<Vec<&str>>()
                    .join(" -> ");

                write!(
                    fmt,
                    "Inheritance cycle detected: {} -> (back to start)",
                    path
                )
            }
            SemanticError::InvalidOverride {
                method_name,
                parent_name,
                reason,
                ..
            } => {
                write!(
                    fmt,
                    "Invalid override of method '{}' from class '{}': {}",
                    method_name, parent_name, reason
                )
            }
        }
    }
}

// =================================
// General compiler errors.
// =================================
#[derive(Debug)]
pub enum CompilerError {
    /// Errors regarding I/O operations (reading files, creating directories, etc.)
    Io(std::io::Error),
    // Lexical errors.
    Lexical(LexicalError, usize, usize),
    /// Syntax errors detected at the parsing stage.
    Parsing(ParseError),
    /// Semantic errors detected at the lowering stage (undefined variables, zero division, etc.)
    Semantic(SemanticError),
    /// External command errors (LLVM tools, etc.)
    ExternalCommand(String),
    /// Other, uncategorized errors.
    Other(String),
}

impl fmt::Display for CompilerError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerError::Io(err) => write!(fmt, "I/O Error: {}", err),
            CompilerError::Lexical(err, _, _) => write!(fmt, "Lexical error: {}", err),
            CompilerError::Parsing(err) => write!(fmt, "Parsing error: {}", err),
            CompilerError::Semantic(err) => write!(fmt, "Semantic Error: {}", err),
            CompilerError::ExternalCommand(msg) => write!(fmt, "External Command Failed: {}", msg),
            CompilerError::Other(msg) => write!(fmt, "Error: {}", msg),
        }
    }
}

// =================================
// Conversions.
// =================================
impl From<std::io::Error> for CompilerError {
    fn from(err: std::io::Error) -> Self {
        CompilerError::Io(err)
    }
}

impl From<ParseError> for CompilerError {
    fn from(err: ParseError) -> Self {
        CompilerError::Parsing(err)
    }
}

impl From<LalrpopParseError<usize, Token, (LexicalError, std::ops::Range<usize>)>>
    for CompilerError
{
    fn from(err: LalrpopParseError<usize, Token, (LexicalError, std::ops::Range<usize>)>) -> Self {
        CompilerError::Parsing(err.into())
    }
}

impl From<SemanticError> for CompilerError {
    fn from(err: SemanticError) -> Self {
        CompilerError::Semantic(err)
    }
}

impl From<&str> for CompilerError {
    fn from(err: &str) -> Self {
        CompilerError::Other(err.to_string())
    }
}

// TODO: Try to remove that.
impl From<Vec<SemanticError>> for CompilerError {
    fn from(errors: Vec<SemanticError>) -> Self {
        if let Some(first_error) = errors.into_iter().next() {
            CompilerError::Semantic(first_error)
        } else {
            CompilerError::Other("Unknown semantic error".to_string())
        }
    }
}

// TODO: Try to remove that.
impl From<SemanticError> for Vec<SemanticError> {
    fn from(value: SemanticError) -> Self {
        vec![value]
    }
}

// TODO: Try to remove that.
impl From<CompilerError> for Vec<CompilerError> {
    fn from(value: CompilerError) -> Self {
        vec![value]
    }
}
