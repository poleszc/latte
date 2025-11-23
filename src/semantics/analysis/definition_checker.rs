use crate::diagnostics::SemanticError;
use crate::parser::ast::*;
use crate::semantics::analysis::context::SemanticContext;
use crate::semantics::analysis::return_checker::ReturnChecker;
use crate::semantics::analysis::stmt_checker::StatementChecker;
use crate::semantics::{ClassInfo, FunctionInfo};
use std::collections::HashMap;

pub struct DefinitionChecker<'a> {
    ctx: &'a mut SemanticContext,
}

// ======================
// Basic functionality.
// ======================
impl<'a> DefinitionChecker<'a> {
    pub fn new(ctx: &'a mut SemanticContext) -> Self {
        Self { ctx }
    }

    /// Main function to perform a type check of a program.
    /// Logs errors in the context in case of errors.
    pub fn check_program(&mut self, program: &Program) {
        // Forward declare all functions and classes in the global scope.
        for top_def in &program.defs {
            match &top_def.kind {
                TopDefKind::Func(func_def) => self.add_function(&func_def),
                TopDefKind::Class(class_def) => self.add_class(&class_def),
            }
        }

        // Now that we have all classes defined, we can validate them.
        for top_def in &program.defs {
            if let TopDefKind::Class(class_def) = &top_def.kind {
                self.validate_class(class_def);
            }
        }

        // Now we type check the functions and methods.
        for top_def in &program.defs {
            self.visit_top_def(&top_def);
        }

        self.validate_main();
    }
}

// ======================
// Utilities.
// ======================
impl<'a> DefinitionChecker<'a> {
    /// Forward declare a class and validate correctness of class definition.
    /// This function does not validate the inheritance hierarchy. This is done in validate_class.
    fn add_class(&mut self, class_def: &ClassDef) {
        let mut fields: HashMap<String, (Type, SourcePosition)> = HashMap::new();
        let mut methods: HashMap<String, FunctionInfo> = HashMap::new();

        // Check for duplicate fields and methods. Extends are not validated yet since we don't know the hierarchy.
        for entry in &class_def.entries {
            match &entry.kind {
                ClassEntryKind::Field(identifiers, typ) => {
                    for ident in identifiers {
                        if fields.contains_key(&ident.name) {
                            self.ctx.add_error(SemanticError::DuplicateClassField {
                                class_name: class_def.name.name.clone(),
                                field_name: ident.name.clone(),
                                position: ident.position,
                                previous_position: fields[&ident.name].1,
                            });
                        } else {
                            fields.insert(ident.name.clone(), (typ.clone(), ident.position));
                        }
                    }
                }
                ClassEntryKind::Method(func_def) => {
                    let method_ident = func_def.name.clone();
                    let signature = FunctionInfo {
                        ident: method_ident.clone(),
                        return_type: func_def.return_type.clone(),
                        params: func_def
                            .args
                            .iter()
                            .map(|arg| arg.arg_type.clone())
                            .collect(),
                        position: func_def.position,
                    };
                    if methods.contains_key(&method_ident.name) {
                        self.ctx.add_error(SemanticError::DuplicateMethod {
                            class_name: class_def.name.name.clone(),
                            method_name: method_ident.name.clone(),
                            position: method_ident.position,
                            previous_position: methods[&method_ident.name].ident.position,
                        });
                    } else {
                        methods.insert(method_ident.name.clone(), signature);
                    }
                }
            }
        }

        // Build class metadata and define the class in the symbol table.
        let class_info = ClassInfo {
            ident: class_def.name.clone(),
            parent: class_def.extends.clone(),
            fields,
            methods,
            position: class_def.position,
        };

        self.ctx.define_class(class_info);
    }

    /// Forward declare a function.
    fn add_function(&mut self, func_def: &FuncDef) {
        let signature = FunctionInfo {
            ident: func_def.name.clone(),
            return_type: func_def.return_type.clone(),
            params: func_def
                .args
                .iter()
                .map(|arg| arg.arg_type.clone())
                .collect(),
            position: func_def.position,
        };
        self.ctx.define_function(signature);
    }

    /// Detects a cycle in the inheritance hierarchy starting from `start_class`.
    /// Returns a list of classes which form a cycle.
    fn detect_inheritance_cycle(
        &self,
        start_class: &str,
    ) -> Option<Vec<(String, String, SourcePosition)>> {
        let mut cycle = Vec::new();
        let mut current_name = start_class.to_string();

        loop {
            let ident = Ident {
                name: current_name.clone(),
                position: SourcePosition(0, 0),
            };

            // TODO: Utility function for that.
            let class_info = match self.ctx.symbol_table.lookup_class(&ident) {
                Ok(info) => info,
                Err(_) => return None,
            };

            if let Some(parent_ident) = &class_info.parent {
                let parent_name = parent_ident.name.clone();
                let position = parent_ident.position;

                cycle.push((current_name.clone(), parent_name.clone(), position));

                if let Some(index) = cycle.iter().position(|(c, _, _)| c == &parent_name) {
                    return Some(cycle[index..].to_vec());
                }
                current_name = parent_name;
            } else {
                return None;
            }
        }
    }

    fn validate_class(&mut self, class_def: &ClassDef) {
        if let Some(parent) = &class_def.extends {
            if !self.ctx.symbol_table.class_exists(&parent.name) {
                self.ctx.add_error(SemanticError::UndefinedClass {
                    name: parent.name.clone(),
                    position: parent.position,
                });
            }
        }

        if let Some(parent) = &class_def.extends {
            // Check for cycles in the inheritance hierarchy.
            if let Some(cycle_path) = self.detect_inheritance_cycle(&class_def.name.name) {
                // A simple hack to deduplicate the cycle errors. Sort the class names
                // lexicographically and report only when we're the smallest (lexically) class.
                // TODO: Introduce and move that to error handler?
                let leader_name = cycle_path
                    .iter()
                    .map(|(name, _, _)| name)
                    .min()
                    .expect("Cycle path cannot be empty");

                if &class_def.name.name == leader_name {
                    self.ctx
                        .add_error(SemanticError::InheritanceCycle { cycle: cycle_path });
                }
            }

            // Check if all overrides are valid - have the same signature.
            // TODO: Check if the field overrides are okay - have the same type.
            for entry in &class_def.entries {
                if let ClassEntryKind::Method(method_def) = &entry.kind {
                    let method_name = &method_def.name;

                    let parent_method_opt = self
                        .ctx
                        .symbol_table
                        .lookup_method(&parent.name, &method_name.name);

                    if let Some(parent_method) = parent_method_opt {
                        // TODO: Use types equal? or some other function which returns an enum on what the mismatch was
                        // Argument count.
                        if method_def.args.len() != parent_method.params.len() {
                            self.ctx.add_error(SemanticError::InvalidOverride {
                                method_name: method_name.name.clone(),
                                parent_name: parent.name.clone(),
                                reason: "Different number of arguments".to_string(),
                                position: method_name.position,
                            });
                            continue;
                        }

                        // Argument types.
                        for (i, (arg, parent_type)) in method_def
                            .args
                            .iter()
                            .zip(&parent_method.params)
                            .enumerate()
                        {
                            if !self.ctx.types_equal(&arg.arg_type, parent_type) {
                                self.ctx.add_error(SemanticError::InvalidOverride {
                                    method_name: method_name.name.clone(),
                                    parent_name: parent.name.clone(),
                                    reason: format!("Argument #{} type mismatch", i + 1),
                                    position: method_name.position,
                                });
                                continue;
                            }
                        }

                        // Return type.
                        if !self
                            .ctx
                            .types_equal(&method_def.return_type, &parent_method.return_type)
                        {
                            self.ctx.add_error(SemanticError::InvalidOverride {
                                method_name: method_name.name.clone(),
                                parent_name: parent.name.clone(),
                                reason: "Return type mismatch".to_string(),
                                position: method_name.position,
                            });
                        }
                    }
                }
            }
        }

        for entry in &class_def.entries {
            if let ClassEntryKind::Field(_, typ) = &entry.kind {
                self.ctx.validate_variable_type(&typ);
            }
        }
    }
}

// ======================
// Definition visitors.
// ======================
impl<'a> DefinitionChecker<'a> {
    fn visit_top_def(&mut self, top_def: &TopDef) {
        match &top_def.kind {
            TopDefKind::Func(func_def) => self.visit_function_definition(func_def),
            TopDefKind::Class(class_def) => self.visit_class_definition(class_def),
        }
    }
    pub fn visit_function_definition(&mut self, func: &FuncDef) {
        self.ctx.symbol_table.enter_scope();

        self.ctx.validate_function_return_type(&func.return_type);

        // TODO: Try to use parametrized lifetime here to avoid cloning?
        let previous_return_type = self.ctx.current_return_type.take();
        self.ctx.current_return_type = Some(func.return_type.clone());

        // TODO: Here we should look for all nested function definitions and handle them.

        // Add arguments to scope.
        for arg in &func.args {
            self.ctx.validate_variable_type(&arg.arg_type);
            self.ctx
                .define_variable(arg.name.clone(), arg.arg_type.clone());
        }

        StatementChecker::new(self.ctx).check_block(&func.body);

        // Check if function is guaranteed to return.
        if !matches!(func.return_type.kind, TypeKind::Void) {
            if !ReturnChecker::function_guarantees_return(&func.body) {
                self.ctx.add_error(SemanticError::MissingReturn {
                    function_name: func.name.name.clone(),
                    position: func.position,
                });
            }
        }

        // TODO: This is bugged in nested functions.
        self.ctx.current_return_type = previous_return_type;
        self.ctx.symbol_table.leave_scope();
    }

    fn visit_class_definition(&mut self, class: &ClassDef) {
        self.ctx.current_class = Some(class.name.name.clone());
        for entry in &class.entries {
            if let ClassEntryKind::Method(method) = &entry.kind {
                self.ctx.symbol_table.enter_scope();

                // Define class fields in scope, including superclasses.
                let all_fields = self.ctx.symbol_table.get_all_class_fields(&class.name.name);
                for (name, (typ, position)) in all_fields {
                    // Helper z context.rs
                    let ident = Ident {
                        name: name.clone(),
                        position,
                    };
                    self.ctx.define_variable(ident, typ.clone());
                }

                self.visit_function_definition(method);
                self.ctx.symbol_table.leave_scope();
            }
        }
        self.ctx.current_class = None;
    }

    fn validate_main(&mut self) {
        let main_ident = Ident {
            name: "main".to_string(),
            position: SourcePosition(0, 0),
        };

        match self.ctx.symbol_table.lookup_function(&main_ident) {
            Ok(signature) => {
                if !matches!(signature.return_type.kind, TypeKind::Int) {
                    let error = SemanticError::WrongMainReturnType {
                        expected: "int".to_string(),
                        found: signature.return_type.to_string(),
                        position: signature.ident.position,
                    };
                    self.ctx.add_error(error);
                }
                if !signature.params.is_empty() {
                    let error = SemanticError::WrongNumberOfMainArguments {
                        found: signature.params.len(),
                        position: signature.ident.position,
                    };
                    self.ctx.add_error(error);
                }
            }
            Err(_) => {
                self.ctx.add_error(SemanticError::UndefinedMain);
            }
        }
    }
}
