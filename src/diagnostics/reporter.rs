use super::error::{CompilerError, LexicalError, ParseError, SemanticError};
use super::{Diagnostic, DiagnosticKind};
use ariadne::{ColorGenerator, Config, Label, Report, ReportKind, Source};

/// Utility for filtering the expected token list.
fn format_expected_tokens(expected: &[String]) -> String {
    if expected.is_empty() {
        return "nothing".to_string();
    }

    let mut tokens: Vec<String> = expected
        .iter()
        .map(|s| s.trim_matches('"').to_string())
        .collect();

    if tokens.len() > 8 {
        tokens.truncate(8);
        tokens.push("...".to_string());
    }

    tokens.join(", ")
}

/// Conversions for compiler errors.
impl Diagnostic {
    pub fn from_lexical_error(error: &LexicalError, span: std::ops::Range<usize>) -> Self {
        match error {
            LexicalError::InvalidInteger(err) => {
                Diagnostic::error(format!("Invalid integer literal: {}", err))
                    .with_label(span, "Cannot parse this as an integer")
                    .with_note("Integer literals must be valid 32-bit signed integers")
                    .with_help("Check if the number is within the range -2147483648 to 2147483647")
            }
            LexicalError::InvalidString => Diagnostic::error("Invalid string literal")
                .with_label(span, "This string literal is malformed")
                .with_note("String literals must be properly terminated with quotes")
                .with_note("Valid escape sequences: \\n, \\t, \\r, \\\\, \\\"")
                .with_help(
                    "Make sure to close the string with a \" and use valid escape sequences",
                ),
            LexicalError::UnterminatedComment => {
                Diagnostic::error("Unterminated multi-line comment")
                    .with_label(span, "This comment is never closed")
                    .with_note("Multi-line comments must end with */")
                    .with_help("Add */ at the end of your comment")
            }
            LexicalError::InvalidToken => Diagnostic::error("Invalid token")
                .with_label(span, "This character sequence is not recognized")
                .with_help("Check for typos or unsupported characters"),
        }
    }

    pub fn from_parse_error(error: &ParseError) -> Self {
        match error {
            ParseError::InvalidToken { location } => Diagnostic::error("Invalid token").with_label(
                *location..*location,
                "This token is invalid in this context",
            ),
            ParseError::UnrecognizedEof { location, expected } => {
                let expected_str = format_expected_tokens(expected);
                Diagnostic::error("Unexpected end of file")
                    .with_label(*location..*location, "File ends unexpectedly here")
                    .with_note(format!("Expected one of: {}", expected_str))
                    .with_help(
                        "Check if you're missing closing braces, parentheses, or a semicolon",
                    )
            }
            ParseError::UnrecognizedToken {
                start,
                end,
                token,
                expected,
            } => {
                let expected_str = format_expected_tokens(expected);
                Diagnostic::error(format!("Unexpected token '{}'", token))
                    .with_label(*start..*end, "This token is not expected here")
                    .with_note(format!("Expected one of: {}", expected_str))
            }
            ParseError::ExtraToken { start, end, token } => {
                Diagnostic::error(format!("Unexpected extra token '{}'", token))
                    .with_label(*start..*end, "This token should not appear here")
                    .with_help("Try removing this token")
            }
            ParseError::LexicalError { start, end, error } => {
                Diagnostic::from_lexical_error(error, *start..*end)
            }
        }
    }

    pub fn from_semantic_error(error: &SemanticError) -> Self {
        match error {
            SemanticError::UndefinedVariable { name, position } => {
                Diagnostic::error(format!("Undefined variable '{}'", name))
                    .with_label(position.0..position.1, "Variable not found")
                    .with_note("This variable was not declared in the current scope")
            }
            SemanticError::UndefinedFunction { name, position } => {
                Diagnostic::error(format!("Undefined function '{}'", name))
                    .with_label(position.0..position.1, "Function not found")
                    .with_note("This function was not declared in the current scope")
            }
            SemanticError::UndefinedClass { name, position } => {
                Diagnostic::error(format!("Undefined class '{}'", name))
                    .with_label(position.0..position.1, "Class not found")
                    .with_note("This class was not declared in the current scope")
            }
            SemanticError::DuplicateSymbol {
                name,
                position,
                previous_position,
            } => {
                Diagnostic::error(format!("Symbol '{}' is already defined", name))
                .with_label(position.0..position.1, "Cannot redefine this symbol")
                .with_secondary_label(
                    previous_position.0..previous_position.1,
                    format!("'{}' was originally defined here", name),
                )
                .with_help("Consider renaming this variable or using a different scope.")
            },
            SemanticError::TypeMismatch {
                expected,
                found,
                position,
            } => {
                Diagnostic::error("Type mismatch")
                .with_label(
                    position.0..position.1,
                    format!("Expected type `{}`, but found type `{}`", expected, found),
                )
                .with_note(format!(
                    "Expected an expression of type `{}` here.",
                    expected
                ))
            },
            SemanticError::InvalidOperands {
                operator,
                operands,
                position,
            } => {
                let op_str = operands.join(", ");
                Diagnostic::error(format!("Invalid operands for operator '{}'", operator))
                    .with_label(position.0..position.1, "Cannot apply this operator")
                    .with_note(format!(
                        "Operator '{}' is not defined for the type(s): {}",
                        operator, op_str
                    ))
            },
            SemanticError::WrongNumberOfArguments {
                name,
                expected,
                found,
                call_position,
                function_position,
            } => {
                // TODO: Add where was the function defined.
                Diagnostic::error(format!("Wrong number of arguments for function '{}'", name))
                    .with_label(
                        call_position.0..call_position.1,
                        format!("Expected {} argument(s), but found {}", expected, found),
                    )
            }
            SemanticError::InvalidReturnValue {
                expected,
                found,
                position,
            } => {
            Diagnostic::error("Invalid return value").with_label(
                position.0..position.1,
                format!(
                    "This function should return `{}`, but it returns `{}`",
                    expected, found
                ),
            )
            },
            SemanticError::InvalidLValue { position } => {
                Diagnostic::error("Invalid assignment")
                    .with_label(position.0..position.1, "This expression cannot be assigned to")
                    .with_help("Only variables, array elements, and object fields can be targets of an assignment.")
            },
            SemanticError::ZeroDivision { position } => {
                Diagnostic::error("Division by zero")
                .with_label(position.0..position.1, "Division by zero literal detected")
                .with_note("Division by zero is undefined and will cause a runtime error")
                .with_help("Use a non-zero value")
            },
            SemanticError::UndefinedMain => {
                Diagnostic::error("This program doesn't provide a `main` function")
                .with_label(0..0, "No `main` function in this program")
                .with_help("Provide a definition for a `main` function")
                
            }
        }
    }
}

/// Rendering with Ariadne.
impl Diagnostic {
    pub fn render(&self, file_path: &str, source: &str) {
        let source_cache = Source::from(source);
        let config = Config::default().with_color(true).with_underlines(true);
        let report_kind = match self.kind {
            DiagnosticKind::Error => ReportKind::Error,
            DiagnosticKind::Warning => ReportKind::Warning,
        };
        let mut report = Report::build(report_kind, file_path, 0)
            .with_config(config)
            .with_message(&self.message);
        let mut colors = ColorGenerator::new();

        for label in &self.labels {
            let color = colors.next();
            let ariadne_label = Label::new((file_path, label.position.clone()))
                .with_message(&label.message)
                .with_color(color);
            report = report.with_label(ariadne_label);
        }

        for note in &self.notes {
            report = report.with_note(note);
        }

        if let Some(help) = &self.help {
            report = report.with_help(help);
        }

        report
            .finish()
            .print((file_path, source_cache))
            .expect("Failed to report an ariadne error")
    }
}

// Main function for reporting errors.
pub fn report_compiler_error(file_path: &str, source: &str, error: &CompilerError) {
    match error {
        CompilerError::Parsing(parse_err) => {
            let diagnostic = Diagnostic::from_parse_error(parse_err);
            diagnostic.render(file_path, source);
        }
        CompilerError::Semantic(semantic_err) => {
            let diagnostic = Diagnostic::from_semantic_error(semantic_err);
            diagnostic.render(file_path, source);
        }
        CompilerError::Lexical(lex_err, start, end) => {
            let diagnostic = Diagnostic::from_lexical_error(lex_err, *start..*end);
            diagnostic.render(file_path, source);
        }
        CompilerError::Io(io_err) => {
            eprintln!("╭─[I/O Error]");
            eprintln!("│ File: {}", file_path);
            eprintln!("│ Error: {}", io_err);
            eprintln!("╰─");
        }
        CompilerError::ExternalCommand(msg) => {
            eprintln!("╭─[External Command Error]");
            eprintln!("│ {}", msg);
            eprintln!("╰─");
        }
        CompilerError::Other(msg) => {
            eprintln!("╭─[Error]");
            eprintln!("│ {}", msg);
            eprintln!("╰─");
        }
    }
}
