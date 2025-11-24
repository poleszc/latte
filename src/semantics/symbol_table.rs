// TODO: Remove those eventually.
#![allow(dead_code)]
#![allow(unused_variables)]
use crate::{
    errors::SemanticError,
    parser::ast::*,
    semantics::{
        builtins,
        symbols::{ClassInfo, FunctionInfo, SymbolKind, VariableInfo},
    },
    utils::StrID,
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct SymbolTable {
    /// Scope stack - mapping from name to symbol.
    scopes: Vec<HashMap<StrID, SymbolKind>>,
    /// Classes defined in the program.
    classes: HashMap<StrID, ClassInfo>,
}

// ============================
// Basic functionality
// ============================
impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self {
            scopes: vec![HashMap::new()], // Insert global scope.
            classes: HashMap::new(),
        };
        table.add_builtin_functions();
        table
    }

    /// Adds builtin functions to the scope.
    fn add_builtin_functions(&mut self) {
        for func_info in builtins::get_builtin_functions() {
            self.scopes[0].insert(func_info.ident.name, SymbolKind::Function(func_info));
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn leave_scope(&mut self) {
        self.scopes.pop().expect("Cannot leave the global scope");
    }

    // TODO: Template the define functions?
    pub fn define_variable(&mut self, var_info: VariableInfo) -> Result<(), SemanticError> {
        let scope = self
            .scopes
            .last_mut()
            .expect("Define variable on empty scope");

        let ident = var_info.ident;

        if let Some(symbol) = scope.get(&ident.name) {
            let previous_position = symbol.get_source_position();
            return Err(SemanticError::DuplicateVariable {
                name: ident.name,
                position: ident.position,
                previous_position,
            });
        }

        scope.insert(
            ident.name,
            SymbolKind::Variable(VariableInfo {
                ident,
                typ: var_info.typ,
            }),
        );

        Ok(())
    }

    pub fn define_function(&mut self, function_info: FunctionInfo) -> Result<(), SemanticError> {
        let scope = self
            .scopes
            .last_mut()
            .expect("Define function on empty scope");

        let ident = function_info.ident;

        if let Some(symbol) = scope.get(&ident.name) {
            let previous_position = symbol.get_source_position();
            return Err(SemanticError::DuplicateFunction {
                name: ident.name,
                position: ident.position,
                previous_position,
            });
        }

        scope.insert(ident.name, SymbolKind::Function(function_info));

        Ok(())
    }

    pub fn define_class(&mut self, class_info: ClassInfo) -> Result<(), SemanticError> {
        let ident = class_info.ident;
        if let Some(symbol) = self.classes.get(&ident.name) {
            let previous_position = symbol.position;
            return Err(SemanticError::DuplicateClass {
                name: ident.name,
                position: ident.position,
                previous_position,
            });
        }

        self.classes.insert(ident.name, class_info);
        Ok(())
    }

    // TODO: Template the lookup functions?
    pub fn lookup_variable(&self, ident: &Ident) -> Result<Type, SemanticError> {
        for scope in self.scopes.iter().rev() {
            if let Some(SymbolKind::Variable(var_info)) = scope.get(&ident.name) {
                return Ok(var_info.typ.clone());
            }
        }

        Err(SemanticError::UndefinedVariable {
            name: ident.name,
            position: ident.position,
        })
    }

    pub fn lookup_function(&self, ident: &Ident) -> Result<FunctionInfo, SemanticError> {
        for scope in self.scopes.iter().rev() {
            if let Some(SymbolKind::Function(func_info)) = scope.get(&ident.name) {
                return Ok(func_info.clone());
            }
        }

        Err(SemanticError::UndefinedFunction {
            name: ident.name,
            position: ident.position,
        })
    }

    pub fn lookup_class(&self, ident: &Ident) -> Result<ClassInfo, SemanticError> {
        self.classes
            .get(&ident.name)
            .cloned()
            .ok_or_else(|| SemanticError::UndefinedClass {
                name: ident.name,
                position: ident.position,
            })
    }

    pub fn lookup_method(&self, class_name: StrID, method_name: StrID) -> Option<FunctionInfo> {
        let mut current_class_name = class_name;

        loop {
            if let Some(class_info) = self.classes.get(&current_class_name) {
                if let Some(method) = class_info.methods.get(&method_name) {
                    return Some(method.clone());
                }

                if let Some(parent) = &class_info.parent {
                    current_class_name = parent.name;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
    }
}

// ============================
// Utilities
// ============================
impl SymbolTable {
    pub fn class_exists(&self, name: StrID) -> bool {
        self.classes.contains_key(&name)
    }

    /// Recursively collects all fields from the parent classes.
    pub fn get_all_class_fields(
        &self,
        class_name: StrID,
    ) -> HashMap<StrID, (Type, SourcePosition)> {
        let mut all_fields = HashMap::new();
        let mut current_class = class_name;
        while let Some(class_info) = self.classes.get(&current_class) {
            // TODO: Parent fields will be overridden if redefined. If this ok?
            for (field_name, field_info) in &class_info.fields {
                if !all_fields.contains_key(field_name) {
                    all_fields.insert(*field_name, field_info.clone());
                }
            }

            if let Some(parent) = &class_info.parent {
                current_class = parent.name;
            } else {
                break;
            }
        }
        all_fields
    }

    /// Recursively collects all method from the parent classes.
    pub fn get_all_class_methods(&self, class_name: StrID) -> HashMap<StrID, FunctionInfo> {
        let mut all_methods = HashMap::new();
        let mut current_class = class_name;

        while let Some(class_info) = self.classes.get(&current_class) {
            if let Some(class_info) = self.classes.get(&current_class) {
                for (method_name, method_sig) in &class_info.methods {
                    if !all_methods.contains_key(method_name) {
                        all_methods.insert(*method_name, method_sig.clone());
                    }
                }

                if let Some(parent) = &class_info.parent {
                    current_class = parent.name;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        all_methods
    }
}
