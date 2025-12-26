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
            .ok_or(SemanticError::UndefinedClass {
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
        }
        all_methods
    }

    pub fn get_class_field_layout(&self, class_name: StrID) -> Vec<(StrID, Type, SourcePosition)> {
        if let Some(class_info) = self.classes.get(&class_name) {
            let mut layout = Vec::new();

            if let Some(parent_ident) = &class_info.parent {
                layout.extend(self.get_class_field_layout(parent_ident.name));
            }

            for field_name in &class_info.field_order {
                let (typ, pos) = class_info.fields.get(field_name).unwrap();
                layout.push((*field_name, typ.clone(), *pos));
            }

            return layout;
        }
        panic!(
            "{}",
            &format!("Class {} not found, but requested field layout", class_name)
        )
    }

    pub fn get_class_vtable_layout(&self, class_name: StrID) -> Vec<FunctionInfo> {
        if let Some(class_info) = self.classes.get(&class_name) {
            let mut vtable_layout: Vec<FunctionInfo>;
            let mut method_map: HashMap<StrID, usize> = HashMap::new();

            // Add parent methods.
            if let Some(parent_ident) = &class_info.parent {
                vtable_layout = self.get_class_vtable_layout(parent_ident.name);
                for (i, func_info) in vtable_layout.iter().enumerate() {
                    method_map.insert(func_info.ident.name, i);
                }
            } else {
                vtable_layout = Vec::new();
            }

            // Add our methods.
            for method_name in &class_info.method_order {
                let method_info = class_info.methods.get(method_name).unwrap();

                // Override parent method.
                if let Some(index) = method_map.get(method_name) {
                    vtable_layout[*index] = method_info.clone();
                } else {
                    // Add new method.
                    vtable_layout.push(method_info.clone());
                }
            }

            return vtable_layout;
        }
        panic!(
            "{}",
            &format!(
                "Class {} not found, but requested vtable layout",
                class_name
            )
        )
    }

    pub fn find_method_owner(&self, mut class_name: StrID, method_name: StrID) -> Option<StrID> {
        loop {
            if let Some(class_info) = self.classes.get(&class_name) {
                if class_info.methods.contains_key(&method_name) {
                    return Some(class_name);
                }

                if let Some(parent) = &class_info.parent {
                    class_name = parent.name;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
    }
}
