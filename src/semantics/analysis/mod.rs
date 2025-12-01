pub mod context;
pub mod definition_checker;
pub mod expr_checker;
pub mod return_checker;
pub mod stmt_checker;
pub mod type_checker;

use crate::errors::SemanticError;
use crate::parser::ast::Program;
use crate::semantics::SemanticAnalysisResult;

use context::SemanticContext;
use definition_checker::DefinitionChecker;

/// Main structure performing the semantic validation of the program.
#[derive(Debug, Default)]
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
    pub fn check(program: &Program) -> Result<SemanticAnalysisResult, Vec<SemanticError>> {
        let mut checker = SemanticChecker::new();
        let mut def_checker = DefinitionChecker::new(&mut checker.ctx);

        let tast = def_checker.check_program(program);

        if checker.ctx.errors.is_empty() {
            Ok(SemanticAnalysisResult {
                tast,
                symbol_table: checker.ctx.symbol_table,
            })
        } else {
            Err(checker.ctx.errors)
        }
    }
}
