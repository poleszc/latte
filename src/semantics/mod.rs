pub mod analysis;
pub mod builtins;
pub mod symbol_table;
pub mod symbols;

pub use analysis::SemanticChecker;
pub use symbols::{ClassInfo, FunctionInfo, VariableInfo};
pub use symbol_table::SymbolTable;
pub use builtins::get_builtin_signature;