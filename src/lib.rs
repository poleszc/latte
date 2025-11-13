// Export all modules used by the compiler from the module for them to be used in tests.
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub instant);

pub mod config;
pub mod diagnostics;
pub mod driver;
pub mod error;
pub mod jvm;
pub mod backend;
pub mod parser;

pub use crate::config::InstantArgs;
pub use crate::diagnostics::report_error;
pub use crate::driver::run_compiler;
pub use crate::error::CompilerError;
pub use crate::instant::ProgramParser;
pub use crate::parser::ast::SourcePosition;
