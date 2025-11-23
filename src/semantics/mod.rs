pub mod builtins;
mod symbol_table;
mod symbols;
mod type_checker;
mod return_checker;
mod context;
mod expr_checker;
mod stmt_checker;
mod definition_checker;

pub use builtins::get_builtin_signature;
pub use symbols::{ClassInfo, FunctionInfo, VariableInfo};

use crate::diagnostics::SemanticError;
use crate::parser::ast::Program;
use context::SemanticContext;
use definition_checker::DefinitionChecker;

/// Main structure performing the semantic validation of the program.
pub struct SemanticChecker {
    /// Semantic verification context.
    ctx: SemanticContext,
}

impl SemanticChecker {
    pub fn new() -> Self {
        Self {
            ctx: SemanticContext::new(),
        }
    }

    /// Main program validation function. 
    pub fn check(program: &Program) -> Result<(), Vec<SemanticError>> {
        let mut checker = SemanticChecker::new();
        let mut def_checker = DefinitionChecker::new(&mut checker.ctx);
        
        def_checker.check_program(program);

        if checker.ctx.errors.is_empty() {
            Ok(())
        } else {
            Err(checker.ctx.errors)
        }
    }
}