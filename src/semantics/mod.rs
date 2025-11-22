pub mod builtins;
mod semantic_checker;
mod symbol_table;
mod symbols;
mod type_checker;
mod return_checker;

pub use builtins::get_builtin_signature;
pub use semantic_checker::SemanticChecker;
pub use symbols::{ClassInfo, FunctionInfo, VariableInfo};
