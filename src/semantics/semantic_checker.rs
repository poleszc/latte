use std::thread::panicking;

use crate::diagnostics::SemanticError;
use crate::parser::ast::*;
use crate::semantics::symbol_table::SymbolTable;

pub struct SemanticChecker {
    symbol_table: SymbolTable,
    /// Return type of the function we're currently analyzing.
    current_return_type: Option<Type>,
    /// Class we're currently analyzing.
    current_class: Option<String>,
    errors: Vec<SemanticError>,
}

// ======================
// Basic and utilities
// ======================
impl SemanticChecker {
    pub fn new() -> Self {
        SemanticChecker {
            symbol_table: SymbolTable::new(),
            current_return_type: None,
            current_class: None,
            errors: Vec::new(),
        }
    }

    /// Main function to call which performs the semantic checks.
    pub fn check(program: &Program) -> Result<(), Vec<SemanticError>> {
        let mut checker = SemanticChecker::new();
        checker.visit_program(program)
    }

    fn add_error(&mut self, error: SemanticError) {
        self.errors.push(error);
    }

    fn types_equal(&self, t1: &Type, t2: &Type) -> bool {
        match (&t1.kind, &t2.kind) {
            (TypeKind::Int, TypeKind::Int) => true,
            (TypeKind::String, TypeKind::String) => true,
            (TypeKind::Bool, TypeKind::Bool) => true,
            (TypeKind::Void, TypeKind::Void) => true,
            (TypeKind::Class(i1), TypeKind::Class(i2)) => i1.name == i2.name,
            (TypeKind::Array(inner1), TypeKind::Array(inner2)) => {
                self.types_equal(&inner1, &inner2)
            }
            // TODO: Add functions here?
            _ => false,
        }
    }

    /// True if `subtype` is a subtype of `supertype`. Essentially if they are the same
    /// type, a subtype in inheritance hierarchy or a null subtype.
    /// For now null is interpreted as a subtype of all classes and arrays.
    /// TODO: Change that.
    /// TODO: This should do something more. Take two types and say if they can be assigned to.
    fn is_subtype(&self, subtype: &Type, supertype: &Type) -> bool {
        // Null is a subtype of all classes and arrays.
        if matches!(subtype.kind, TypeKind::Null) {
            return matches!(supertype.kind, TypeKind::Class(_) | TypeKind::Array(_));
        }

        if self.types_equal(subtype, supertype) {
            return true;
        }

        if let (TypeKind::Class(sub_class), TypeKind::Class(super_class)) =
            (&subtype.kind, &supertype.kind)
        {
            return self.is_class_subtype(&sub_class.name, &super_class.name);
        }
        false
    }

    /// True if `subclass` if a subclass of `superclass` in the inheritance hierarchy context.
    fn is_class_subtype(&self, subclass: &str, superclass: &str) -> bool {
        if subclass == superclass {
            return true;
        }

        let mut current = subclass.to_string();
        loop {
            if let Ok(class_info) = self.symbol_table.lookup_class(&Ident {
                name: current.to_string(),
                position: SourcePosition(0, 0),
            }) {
                if let Some(parent) = class_info.parent {
                    if parent.name == superclass {
                        return true;
                    }
                    current = parent.name.clone();
                } else {
                    return false;
                }
            }
        }
    }
}

impl SemanticChecker {
    fn add_class(&mut self, class_def: &ClassDef) -> Result<(), Vec<SemanticError>> {
        todo!()
    }
    fn add_function(&mut self, func_def: &FuncDef) -> Result<(), Vec<SemanticError>> {
        todo!()
    }
    fn validate_class(&mut self, class_def: &ClassDef) -> Result<(), Vec<SemanticError>> {
        todo!()
    }
    fn validate_main(&mut self) -> Result<(), Vec<SemanticError>> {
        let main_ident = Ident {
            name: "main".to_string(),
            position: SourcePosition(0, 0),
        };

        match self.symbol_table.lookup_function(&main_ident) {
            Ok(signature) => {
                if !matches!(signature.return_type.kind, TypeKind::Int) {
                    let error = SemanticError::WrongMainReturnType {
                        expected: "int".to_string(),
                        found: signature.return_type.to_string(),
                        position: signature.ident.position,
                    };
                    self.add_error(error);
                }
                if !signature.params.is_empty() {
                    let error = SemanticError::WrongNumberOfMainArguments {
                        found: signature.params.len(),
                        position: signature.ident.position,
                    };
                    self.add_error(error);
                }
            }
            Err(_) => {
                self.add_error(SemanticError::UndefinedMain);
            }
        }

        Ok(())
    }
}

// ======================
// Top Level
// ======================
impl SemanticChecker {
    // TODO: Probably remove the Result<(), Vec<>>
    pub fn visit_program(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        // Forward declare all functions and classes in the global scope.
        for top_def in &program.defs {
            match &top_def.kind {
                TopDefKind::Func(func_def) => self.add_function(&func_def)?,
                TopDefKind::Class(class_def) => self.add_class(&class_def)?,
            }
        }

        // Now that we have all classes defined, we can validate them.
        for top_def in &program.defs {
            if let TopDefKind::Class(class_def) = &top_def.kind {
                self.validate_class(class_def)?;
            }
        }

        // Now we type check the functions and methods.
        for top_def in &program.defs {
            match &top_def.kind {
                TopDefKind::Func(func_def) => self.visit_function_definition(func_def)?,
                TopDefKind::Class(class_def) => self.visit_class_definition(class_def)?,
            }
        }

        self.validate_main()?;

        Ok(())
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

    pub fn visit_function_definition_expr(
        &mut self,
        block: &Block,
    ) -> Result<(), Vec<SemanticError>> {
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
