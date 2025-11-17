use crate::{diagnostics::SemanticError, parser::ast::*};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<HashMap<Ident, Type>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // Insert global scope.
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn leave_scope(&mut self) {
        self.scopes.pop().expect("Cannot leave the global scope");
    }

    pub fn define(&mut self, name: &Ident, typ: Type) -> Result<(), SemanticError> {
        let current_scope = self.scopes.last_mut().expect("Define on an empty scope");
        if current_scope.contains_key(name) {
            // Duplicate declaration.
        }

        // TODO: Try to remove clone here?
        current_scope.insert(name.clone(), typ);
        Ok(())
    }

    pub fn lookup(&self, name: &Ident) -> Result<Type, SemanticError> {
        for scope in self.scopes.iter().rev() {
            if let Some(typ) = scope.get(name) {
                // TODO: Remove clone.
                return Ok(typ.clone());
            }
        }
        // TODO: Undefined variable
        Err(SemanticError::UndefinedVariable {
            name: name.0.clone(),
            position: name.1,
        })
    }
}
