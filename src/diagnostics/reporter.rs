use crate::SourcePosition;
use crate::semantics::builtins;

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
                .with_note("Valid escape sequences: '\\n', '\\t', '\\r', '\\\\', '\\\"'")
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
            },
            SemanticError::UndefinedFunction { name, position } => {
                Diagnostic::error(format!("Undefined function '{}'", name))
                    .with_label(position.0..position.1, "Function not found")
                    .with_note("This function was not declared in the current scope")
            },
            SemanticError::UndefinedClass { name, position } => {
                Diagnostic::error(format!("Undefined class '{}'", name))
                    .with_label(position.0..position.1, "Class not found")
                    .with_note("This class was not declared in the current scope")
            },
            SemanticError::UndefinedField { class_name, field_name, position, class_position } => {
                Diagnostic::error(format!("Undefined field '{}'", field_name))
                    .with_label(position.0..position.1, format!("Field '{}' was not declared", field_name))
                    .with_label(class_position.0..class_position.1, "In this class")
                    .with_note(format!("Class field '{}' was not declared in class '{}'", field_name, class_name))
                .with_help("Consider declaring this field.")
            }
            SemanticError::UndefinedMethod { class_name, method_name, position, class_position } => {
                Diagnostic::error(format!("Undefined method '{}'", method_name))
                    .with_label(position.0..position.1, format!("Method '{}' was not declared", method_name))
                    .with_label(class_position.0..class_position.1, "In this class")
                    .with_note(format!("Method '{}' was not declared in class '{}'", method_name, class_name))
                .with_help("Consider implementing this method.")
            }
            SemanticError::DuplicateVariable { name, position, previous_position } => {
                Diagnostic::error(format!("Variable '{}' is already defined", name))
                .with_label(position.0..position.1, "Cannot redefine this variable")
                .with_secondary_label(
                    previous_position.0..previous_position.1,
                    format!("'{}' was originally defined here", name),
                )
                .with_help("Consider renaming this variable or using a different scope.")
            },
            SemanticError::DuplicateFunction { name, position, previous_position } => {
                Diagnostic::error(format!("Function '{}' is already defined", name))
                .with_label(position.0..position.1, "Cannot redefine this function")
                .with_secondary_label(
                    previous_position.0..previous_position.1,
                    format!("'{}' was originally defined here", name),
                )
                .with_help("Consider renaming this function or using a different scope.")
            },
            SemanticError::DuplicateClass { name, position, previous_position } => {
                Diagnostic::error(format!("Class '{}' is already defined", name))
                .with_label(position.0..position.1, "Cannot redefine this class")
                .with_secondary_label(
                    previous_position.0..previous_position.1,
                    format!("'{}' was originally defined here", name),
                )
                .with_help("Consider renaming this class.")
            },
            SemanticError::DuplicateClassField { class_name, field_name, position, previous_position } => {
                Diagnostic::error(format!("Field '{}' is already defined for class '{}'", field_name, class_name))
                .with_label(position.0..position.1, "Cannot redefine this field")
                .with_secondary_label(
                    previous_position.0..previous_position.1,
                    format!("'{}' was originally defined here", field_name),
                )
                .with_help("Consider renaming this field.")
            },
            SemanticError::DuplicateMethod { class_name, method_name, position, previous_position } => {
                Diagnostic::error(format!("Method '{}' is already defined for class '{}'", method_name, class_name))
                .with_label(position.0..position.1, "Cannot redefine this method")
                .with_secondary_label(
                    previous_position.0..previous_position.1,
                    format!("'{}' was originally defined here", method_name),
                )
                .with_help("Consider renaming this method.")
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
            SemanticError::ForEachTypeMismatch { variable_type, collection_item_type, variable_position, collection_position } => {
                Diagnostic::error("Type mismatch in for-each loop")
                .with_label(
                    collection_position.0..collection_position.1,
                    format!("This collection contains elements of type `{}`", collection_item_type),
                )
                .with_label(
                    variable_position.0..variable_position.1,
                    format!("But the loop variable is declared as type `{}`", variable_type),
                )
                .with_note(format!(
                    "Type `{}` cannot be assigned to variable of type `{}`",
                    collection_item_type, variable_type
                ))
            },
            SemanticError::ArgumentTypeMismatch { expected, found, position, function_name, argument_position } => {
                let mut diag = Diagnostic::error("Argument type mismatch")
                    .with_label(
                        position.0..position.1,
                        format!("Expected type `{}`, but found type `{}`", expected, found),
                    );

                if let Some(signature) = builtins::get_builtin_signature(function_name) {
                    diag = diag
                        .with_note(format!("`{}` is a built-in function with signature: '{}'", function_name, signature))
                } else {
                    diag = diag
                        .with_label(
                            argument_position.0..argument_position.1,
                            format!("Function `{}` declared here expecting `{}`", function_name, expected),
                        )
                        .with_note(format!(
                            "Expected an expression of type `{}` here.",
                            expected
                        ));
                }
                diag
            }
            SemanticError::AssignTypeMismatch { lhs_type, rhs_type, lhs_position, rhs_position } => {
                Diagnostic::error("Type mismatch")
                .with_label(
                    lhs_position.0..lhs_position.1,
                    format!("Left side of assignment is of type `{}`", lhs_type),
                )
                .with_label(
                    rhs_position.0..rhs_position.1,
                    format!("Right side of assignment is of type `{}`", rhs_type),
                )
                .with_note("Expected both sides of the assignment to have the same type")
            }
            SemanticError::BinaryOpTypeMismatch {
                lhs_type,
                lhs_position,
                rhs_type,
                rhs_position,
            } => {
                Diagnostic::error("Type mismatch")
                .with_label(
                    lhs_position.0..lhs_position.1,
                    format!("Left side of expression is of type `{}`", lhs_type),
                )
                .with_label(
                    rhs_position.0..rhs_position.1,
                    format!("Right side of expression is of type `{}`", rhs_type),
                )
                .with_note("Expected both sides of the expression to have the same type")
            },
            SemanticError::DeclarationTypeMismatch { var_name,expected, decl_type_position, found, expr_position } => {
                Diagnostic::error("Type mismatch")
                .with_label(
                    expr_position.0..expr_position.1,
                    format!("The initializing expression is of type '{}'", found),
                )
                .with_label(
                    decl_type_position.0..decl_type_position.1,
                    format!("Variable '{}' was declared with type '{}'", var_name, expected),
                )
                .with_help("Consider changing the type of this variable or change the initializing expression.")
            }
            SemanticError::InvalidOperands {
                operator,
                operand,
                position,
                type_position
            } => {
                Diagnostic::error(format!("Invalid operands for operator '{}'", operator))
                    .with_label(position.0..position.1, format!("Cannot apply '{}' to a value of type '{}'", operator, operand))
                    .with_label(type_position.0..type_position.1, format!("Operand is of type '{}'", operand))
                    .with_note(format!(
                        "Operator '{}' is not defined for the type(s): {}",
                        operator, operand
                    ))
            },
            SemanticError::WrongNumberOfArguments {
                name,
                expected,
                found,
                call_position,
                function_position,
            } => {
                Diagnostic::error(format!("Wrong number of arguments for function '{}'", name))
                    .with_label(
                        call_position.0..call_position.1,
                        format!("But `{}` was called here and provided with {} argument(s)", name, found),
                    )
                    .with_label(function_position.0..function_position.1, format!("`{}` was defined here and expects {} argument(s)", name, expected)
                    )
                .with_help("Make sure the function call is provided with the same number of arguments as in the definition")
            },
            SemanticError::InvalidReturnValue {
                expected,
                found,
                position,
                ret_type_position,
            } => {
                Diagnostic::error("Invalid return value")
                .with_label(
                    position.0..position.1,
                    format!("But here a value of type `{}` was provided", found),
                )
                .with_label(
                    ret_type_position.0..ret_type_position.1,
                    format!("This function returns `{}`", expected),
                )
                .with_help("Consider changing the return type or the return statement")
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
            },
            SemanticError::WrongMainReturnType { expected, found, position } => {
                Diagnostic::error(format!("Wrong return type for `main`"))
                .with_label(position.0..position.1, format!("`main` was defined here and returns {}, but is expected to return {}", found, expected))
                .with_help(format!("Change the main return type to {}", expected))
            },
            SemanticError::WrongNumberOfMainArguments { found, position } => {
                Diagnostic::error(format!("Wrong number of arguments for `main`"))
                .with_label(position.0..position.1, format!("`main` was defined here and expects {} arguments, but is expected to take in 0 arguments", found))
                .with_note("Main is expected to have 0 arguments in a valid program")
                .with_help("Change the `main` definition to expect no arguments")
            },
            SemanticError::InvalidVariableType { found, expected, position } => {
                Diagnostic::error("Invalid variable type")
                .with_label(position.0..position.1, format!("`{}` is not a valid type for a variable, function argument or a class field", found))
                .with_help(format!("Try using one of: {}", expected.join(", ")))
            },
            SemanticError::InvalidFunctionReturnType { found, expected, position } => {
                Diagnostic::error("Invalid function return type")
                .with_label(position.0..position.1, format!("`{}` is not a valid return type", found))
                .with_help(format!("Try using one of: {}", expected.join(", ")))
            },
            SemanticError::InvalidVoidArrayType { expected, position } => {
                Diagnostic::error("Invalid array type")
                .with_label(position.0..position.1, "`void` is not a valid base type for an array type")
                .with_help(format!("Try using one of: {}", expected.join(", ")))
            },
            SemanticError::InvalidNullCast { found, expected, type_position, expr_position } => {
                Diagnostic::error("Invalid 'null' cast")
                .with_label(expr_position.0..expr_position.1, "In this expression")
                .with_label(type_position.0..type_position.1, format!("`{}` is not a valid type for a null cast", found))
                .with_help(format!("Try using one of: {}", expected.join(", ")))
            }
            SemanticError::InvalidNullCastExpr { expected, cast_position, expr_position } => {
                Diagnostic::error("Invalid 'null' cast")
                .with_label(expr_position.0..expr_position.1, "In this expression")
                .with_label(cast_position.0..cast_position.1, "This is not a valid expression for a null cast")
                .with_help(format!("Try using one of: {}", expected.join(", ")))
            }
            SemanticError::MissingReturn { function_name, position } => {
                Diagnostic::error(format!("Function {} doesn't return on all paths", function_name))
                .with_label(position.0..position.1, "In this function, not all paths end with a return statement")
                .with_help("Make sure all paths end with a return statement")
            }
            SemanticError::InvalidVoidReturn { ret_type_position } => {
                Diagnostic::error("Return with an expression of type 'void'")
                .with_label(ret_type_position.0..ret_type_position.1, "This expression if of type 'void'")
                .with_note("Return statements can't return values of type 'void'")
                .with_help("Change the expression in the return statement")
            }
            SemanticError::ThisUsedOutsideClass { position } => {
                Diagnostic::error("Invalid use of 'this'")
                .with_label(
                    position.0..position.1,
                    "'this' keyword can only be used inside a class method",
                )
                .with_help("Remove 'this' or move the code into a class method.")
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

        // Location of the error is the position of the first label.
        let offset = self
            .labels
            .first()
            .map(|label| label.position.start)
            .unwrap_or(0);
        let mut report = Report::build(report_kind, file_path, offset)
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
