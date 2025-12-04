// Export all modules used by the compiler from the module for them to be used in tests.
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub latte);

pub mod config;
pub mod driver;
pub mod errors;
pub mod llvm;
pub mod mir;
pub mod parser;
pub mod semantics;
pub mod utils;

pub use crate::config::LatteArgs;
pub use crate::driver::run_compiler;
pub use crate::errors::{CompilerError, report_compiler_error};
pub use crate::latte::ProgramParser;
pub use crate::parser::ast::SourcePosition;
