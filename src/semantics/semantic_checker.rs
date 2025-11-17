use crate::diagnostics::SemanticError;
use crate::parser::ast::*;
use crate::semantics::symbol_table::SymbolTable;

pub struct SemanticChecker {
    symbol_table: SymbolTable,
    current_return_type: Option<Type>,
    errors: Vec<SemanticError>,
}

// ======================
// Basic
// ======================
impl SemanticChecker {
    pub fn new() -> Self {
        SemanticChecker {
            symbol_table: SymbolTable::new(),
            current_return_type: None,
            errors: Vec::new(),
        }
    }
}

// ======================
// Top Level
// ======================
impl SemanticChecker {
    pub fn visit_program(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_top_level(&mut self, top_def: &TopDef) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_function_definition(&mut self, func: &FuncDef) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_class_definition(&mut self, class: &ClassDef) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_class_entry(&mut self, entry: ClassEntry) -> Result<(), Vec<SemanticError>> {
        todo!();
    }
}

// ======================
// Statements
// ======================
impl SemanticChecker {
    pub fn visit_statement(&mut self, stmt: &Stmt) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_empty(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_block(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_declaration(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_assign(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_return(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_increment(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_decrement(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_if(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_while(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_for(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_each(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_expr_statement(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_function_definition_expr(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }
}

// ======================
// Expressions
// ======================
impl SemanticChecker {
    // Checks an expression and returns it's type.
    pub fn visit_expr(&mut self, block: &Expr) -> Result<Type, Vec<SemanticError>> {
        todo!();
    }
}
