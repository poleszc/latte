/// Main file with the error logging logic. Transforms all the compilation errors to ariadne messages.
use crate::error::{CompilerError, SemanticError};
use crate::parser::tokens::{LexicalError, Token};
use ariadne::{ColorGenerator, Config, Label, Report, ReportKind, Source};
use lalrpop_util::ParseError;

/// Reports a parsing error with ariadne.
fn report_parsing_error(
    file_path: &str,
    source: &str,
    err: ParseError<usize, Token, LexicalError>,
) {
    let source_cache = Source::from(source);
    let config = Config::default().with_color(true).with_underlines(true);
    let report = Report::build(ReportKind::Error, file_path, 0).with_config(config);
    let mut colors = ColorGenerator::new();

    let (report, _) = match err {
        ParseError::InvalidToken { location } => (
            report.with_message("Invalid token").with_label(
                Label::new((file_path, location..location))
                    .with_message("This token is invalid in this context")
                    .with_color(colors.next()),
            ),
            location..location,
        ),
        ParseError::UnrecognizedEof { location, expected } => (
            report.with_message("Unrecognized end of file").with_label(
                Label::new((file_path, location..location))
                    .with_message(format!("Expected one of: {}", expected.join(", ")))
                    .with_color(colors.next()),
            ),
            location..location,
        ),
        ParseError::UnrecognizedToken { token, expected } => {
            let (start, tok, end) = token;
            (
                report
                    .with_message(format!("Unrecognized token '{}'", tok))
                    .with_label(
                        Label::new((file_path, start..end))
                            .with_message(format!("Expected one of: {}", expected.join(", ")))
                            .with_color(colors.next()),
                    ),
                start..end,
            )
        }
        ParseError::ExtraToken { token } => {
            let (start, tok, end) = token;
            (
                report
                    .with_message(format!("Unexpected token '{}'", tok))
                    .with_label(
                        Label::new((file_path, start..end))
                            .with_message("This token should not appear here")
                            .with_color(colors.next()),
                    ),
                start..end,
            )
        }
        ParseError::User { error } => (
            report
                .with_message("Lexer error")
                .with_note(format!("Additional information: {}", error)),
            0..0,
        ),
    };

    report
        .finish()
        .print((file_path, source_cache))
        .expect("Failed to report an error");
}

/// Reports a semantic error with ariadne.
fn report_semantic_error(file_path: &str, source: &str, err: SemanticError) {
    let source_cache = Source::from(source);
    let config = Config::default().with_color(true).with_underlines(true);
    let report = Report::build(ReportKind::Error, file_path, 0).with_config(config);
    let mut colors = ColorGenerator::new();

    let (report, _) = match err {
        SemanticError::UndefinedVariable { name, position } => (
            report
                .with_message(format!("Undefined variable '{}'", name))
                .with_label(
                    Label::new((file_path, position.0..position.1))
                        .with_message(
                            "This variable was not declared in the current scope.".to_string(),
                        )
                        .with_color(colors.next()),
                ),
            position.0..position.1,
        ),
        SemanticError::ZeroDivision { position } => (
            report.with_message("Zero division").with_label(
                Label::new((file_path, position.0..position.1))
                    .with_message("Zero literal on the right side of division operator.")
                    .with_color(colors.next()),
            ),
            position.0..position.1,
        ),
    };

    report
        .finish()
        .print((file_path, source_cache))
        .expect("Failed to report an error");
}

pub fn report_error(file_path: &str, source: &str, err: CompilerError) {
    match err {
        CompilerError::Parsing(parse_err) => report_parsing_error(file_path, source, parse_err),
        CompilerError::Semantic(sematic_err) => {
            report_semantic_error(file_path, source, sematic_err)
        }
        CompilerError::Io(io_err) => {
            println!("[I/O Error] {}: {}", file_path, io_err)
        }
        CompilerError::ExternalCommand(msg) => {
            println!("[External Command Error]: {}", msg)
        }
        CompilerError::Other(msg) => {
            println!("[Error]: {}", msg)
        }
    }
}
