// TODO: Remove those eventually.
#![allow(dead_code)]
#![allow(unused_variables)]
use crate::{
    diagnostics::SemanticError,
    parser::ast::*,
    semantics::symbols::{ClassInfo, FunctionInfo, SymbolKind, VariableInfo},
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SymbolTable {
    /// Scope stack - mapping from name to symbol.
    scopes: Vec<HashMap<String, SymbolKind>>,
    /// Classes defined in the program.
    classes: HashMap<String, ClassInfo>,
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
        let builtins = vec![
            (
                "printInt",
                Type {
                    kind: TypeKind::Void,
                    position: SourcePosition(0, 0),
                },
                vec![Type {
                    kind: TypeKind::Int,
                    position: SourcePosition(0, 0),
                }],
            ),
            (
                "printString",
                Type {
                    kind: TypeKind::Void,
                    position: SourcePosition(0, 0),
                },
                vec![Type {
                    kind: TypeKind::String,
                    position: SourcePosition(0, 0),
                }],
            ),
            (
                "error",
                Type {
                    kind: TypeKind::Void,
                    position: SourcePosition(0, 0),
                },
                vec![],
            ),
            (
                "readInt",
                Type {
                    kind: TypeKind::Int,
                    position: SourcePosition(0, 0),
                },
                vec![],
            ),
            (
                "readString",
                Type {
                    kind: TypeKind::String,
                    position: SourcePosition(0, 0),
                },
                vec![],
            ),
        ];

        for (name, ret_type, params) in builtins {
            let ident = Ident {
                name: name.to_string(),
                position: SourcePosition(0, 0),
            };

            let signature = FunctionInfo {
                ident: ident,
                return_type: ret_type,
                params,
                // TODO: Check if SourcePosition is 0, 0 in dia and don't print labels then.
                position: SourcePosition(0, 0),
            };
            self.scopes[0].insert(name.to_string(), SymbolKind::Function(signature));
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
                previous_position: previous_position,
            });
        }

        scope.insert(
            ident.name.clone(),
            SymbolKind::Variable(VariableInfo {
                ident: ident.clone(),
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

        let ident = function_info.ident.clone();

        if let Some(symbol) = scope.get(&ident.name) {
            let previous_position = symbol.get_source_position();
            return Err(SemanticError::DuplicateFunction {
                name: ident.name,
                position: ident.position,
                previous_position: previous_position,
            });
        }

        scope.insert(ident.name.clone(), SymbolKind::Function(function_info));

        Ok(())
    }

    pub fn define_class(&mut self, class_info: ClassInfo) -> Result<(), SemanticError> {
        let ident = class_info.ident.clone();
        if let Some(symbol) = self.classes.get(&ident.name) {
            let previous_position = symbol.position;
            return Err(SemanticError::DuplicateClass {
                name: ident.name,
                position: ident.position,
                previous_position: previous_position,
            });
        }

        self.classes.insert(ident.name.clone(), class_info);
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
            name: ident.name.clone(),
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
            name: ident.name.clone(),
            position: ident.position,
        })
    }

    pub fn lookup_class(&self, ident: &Ident) -> Result<ClassInfo, SemanticError> {
        self.classes
            .get(&ident.name)
            .cloned()
            .ok_or_else(|| SemanticError::UndefinedClass {
                name: ident.name.clone(),
                position: ident.position,
            })
    }
}

// ============================
// Utilities
// ============================
impl SymbolTable {
    pub fn class_exists(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    /// Recursively collects all fields from the parent classes.
    pub fn get_all_class_fields(
        &self,
        class_name: &str,
    ) -> HashMap<String, (Type, SourcePosition)> {
        let mut all_fields = HashMap::new();
        let mut current_class = class_name;

        loop {
            if let Some(class_info) = self.classes.get(current_class) {
                // TODO: Parent fields will be overridden if redefined. If this ok?
                for (field_name, field_info) in &class_info.fields {
                    if !all_fields.contains_key(field_name) {
                        all_fields.insert(field_name.clone(), field_info.clone());
                    }
                }

                if let Some(parent) = &class_info.parent {
                    current_class = &parent.name;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        all_fields
    }

    /// Recursively collects all method from the parent classes.
    pub fn get_all_class_methods(&self, class_name: &str) -> HashMap<String, FunctionInfo> {
        let mut all_methods = HashMap::new();
        let mut current_class = class_name;

        loop {
            if let Some(class_info) = self.classes.get(current_class) {
                for (method_name, method_sig) in &class_info.methods {
                    if !all_methods.contains_key(method_name) {
                        all_methods.insert(method_name.clone(), method_sig.clone());
                    }
                }

                if let Some(parent) = &class_info.parent {
                    current_class = &parent.name;
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
