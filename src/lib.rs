// Export all modules used by the compiler from the module for them to be used in tests.
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub latte);

pub mod config;
pub mod diagnostics;
pub mod driver;
pub mod error;
pub mod parser;
pub mod llvm;

pub use crate::config::LatteArgs;
pub use crate::diagnostics::report_error;
pub use crate::driver::run_compiler;
pub use crate::error::CompilerError;
pub use crate::latte::ProgramParser;
pub use crate::parser::ast::SourcePosition;
