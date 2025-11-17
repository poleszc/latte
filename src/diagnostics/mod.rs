mod diagnostics;
mod error;
mod reporter;

pub use diagnostics::{Diagnostic, DiagnosticKind, DiagnosticLabel};
pub use error::{CompilerError, LexicalError, ParseError, SemanticError};
pub use reporter::report_compiler_error;
