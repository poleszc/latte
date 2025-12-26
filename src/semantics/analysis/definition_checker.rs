use crate::errors::SemanticError;
use crate::parser::ast::*;
use crate::semantics::analysis::context::SemanticContext;
use crate::semantics::analysis::return_checker::ReturnChecker;
use crate::semantics::analysis::stmt_checker::StatementChecker;
use crate::semantics::{ClassInfo, FunctionInfo, tast};
use crate::utils::StrID;
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
    pub fn check_program(&mut self, program: &Program) -> tast::TProgram {
        // Forward declare all functions and classes in the global scope.
        for top_def in &program.defs {
            match &top_def.kind {
                TopDefKind::Func(func_def) => self.add_function(func_def),
                TopDefKind::Class(class_def) => self.add_class(class_def),
            }
        }

        // Now that we have all classes defined, we can validate them.
        for top_def in &program.defs {
            if let TopDefKind::Class(class_def) = &top_def.kind {
                self.validate_class(class_def);
            }
        }

        let tast_defs = program
            .defs
            .iter()
            .map(|top_def| self.visit_top_def(top_def))
            .collect();

        self.validate_main();

        tast::TProgram { defs: tast_defs }
    }
}

// ======================
// Utilities.
// ======================
impl<'a> DefinitionChecker<'a> {
    /// Forward declare a class and validate correctness of class definition.
    /// This function does not validate the inheritance hierarchy. This is done in validate_class.
    fn add_class(&mut self, class_def: &ClassDef) {
        let mut fields: HashMap<StrID, (Type, SourcePosition)> = HashMap::new();
        let mut methods: HashMap<StrID, FunctionInfo> = HashMap::new();
        let mut field_order = Vec::new();
        let mut method_order = Vec::new();

        // Check for duplicate fields and methods. Extends are
        // not validated yet since we don't know the hierarchy.
        for entry in &class_def.entries {
            match &entry.kind {
                ClassEntryKind::Field(identifiers, typ) => {
                    for ident in identifiers {
                        if fields.contains_key(&ident.name) {
                            self.ctx.add_error(SemanticError::DuplicateClassField {
                                class_name: class_def.name.name,
                                field_name: ident.name,
                                position: ident.position,
                                previous_position: fields[&ident.name].1,
                            });
                        } else {
                            fields.insert(ident.name, (typ.clone(), ident.position));
                            field_order.push(ident.name);
                        }
                    }
                }
                ClassEntryKind::Method(func_def) => {
                    let method_ident = func_def.name;
                    let signature = FunctionInfo {
                        ident: method_ident,
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
                            class_name: class_def.name.name,
                            method_name: method_ident.name,
                            position: method_ident.position,
                            previous_position: methods[&method_ident.name].ident.position,
                        });
                    } else {
                        methods.insert(method_ident.name, signature);
                        method_order.push(method_ident.name);
                    }
                }
            }
        }

        // Build class metadata and define the class in the symbol table.
        let class_info = ClassInfo {
            ident: class_def.name,
            parent: class_def.extends,
            fields,
            methods,
            field_order,
            method_order,
            position: class_def.position,
        };

        self.ctx.define_class(class_info);
    }

    /// Forward declare a function.
    fn add_function(&mut self, func_def: &FuncDef) {
        let signature = FunctionInfo {
            ident: func_def.name,
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
        start_class: StrID,
    ) -> Option<Vec<(StrID, StrID, SourcePosition)>> {
        let mut cycle = Vec::new();
        let mut current_name = start_class;

        loop {
            let ident = self.ctx.mock_identifier(current_name);
            let class_info = match self.ctx.symbol_table.lookup_class(&ident) {
                Ok(info) => info,
                Err(_) => return None,
            };

            if let Some(parent_ident) = &class_info.parent {
                let parent_name = parent_ident.name;
                let position = parent_ident.position;

                cycle.push((current_name, parent_name, position));

                if let Some(index) = cycle.iter().position(|(c, _, _)| *c == parent_name) {
                    return Some(cycle[index..].to_vec());
                }
                current_name = parent_name;
            } else {
                return None;
            }
        }
    }

    fn find_field_in_hierarchy(
        &self,
        start_parent_name: StrID,
        field_name: StrID,
    ) -> Option<SourcePosition> {
        let mut current_parent = Some(start_parent_name);

        while let Some(parent_name) = current_parent {
            let dummy_ident = self.ctx.mock_identifier(parent_name);
            if let Ok(class_info) = self.ctx.symbol_table.lookup_class(&dummy_ident) {
                if let Some((_, pos)) = class_info.fields.get(&field_name) {
                    return Some(*pos);
                }
                current_parent = class_info.parent.map(|p| p.name);
            } else {
                break;
            }
        }
        None
    }

    fn validate_class(&mut self, class_def: &ClassDef) {
        if let Some(parent) = &class_def.extends
            && !self.ctx.symbol_table.class_exists(parent.name)
        {
            self.ctx.add_error(SemanticError::UndefinedClass {
                name: parent.name,
                position: parent.position,
            });
        }

        if let Some(parent) = &class_def.extends {
            // Check for cycles in the inheritance hierarchy.
            if let Some(cycle_path) = self.detect_inheritance_cycle(class_def.name.name) {
                // A simple hack to deduplicate the cycle errors. Sort the class names
                // lexicographically and report only when we're the smallest (lexically) class.
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
            for entry in &class_def.entries {
                match &entry.kind {
                    ClassEntryKind::Field(idents, _) => {
                        for ident in idents {
                            if let Some(prev_pos) =
                                self.find_field_in_hierarchy(parent.name, ident.name)
                            {
                                dbg!(&class_def);
                                self.ctx.add_error(SemanticError::DuplicateClassField {
                                    class_name: class_def.name.name,
                                    field_name: ident.name,
                                    position: ident.position,
                                    previous_position: prev_pos,
                                });
                            }
                        }
                    }
                    ClassEntryKind::Method(method_def) => {
                        let method_name = &method_def.name;

                        let parent_method_opt = self
                            .ctx
                            .symbol_table
                            .lookup_method(parent.name, method_name.name);

                        if let Some(parent_method) = parent_method_opt {
                            // Argument count.
                            if method_def.args.len() != parent_method.params.len() {
                                self.ctx.add_error(SemanticError::InvalidOverride {
                                    method_name: method_name.name,
                                    parent_name: parent.name,
                                    reason: StrID::from("Different number of arguments"),
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
                                        method_name: method_name.name,
                                        parent_name: parent.name,
                                        reason: StrID::from(format!(
                                            "Argument #{} type mismatch",
                                            i + 1
                                        )),
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
                                    method_name: method_name.name,
                                    parent_name: parent.name,
                                    reason: StrID::from("Return type mismatch"),
                                    position: method_name.position,
                                });
                            }
                        }
                    }
                }
            }
        }

        for entry in &class_def.entries {
            if let ClassEntryKind::Field(_, typ) = &entry.kind {
                self.ctx.validate_variable_type(typ);
            }
        }
    }
}

// ======================
// Definition visitors.
// ======================
impl<'a> DefinitionChecker<'a> {
    fn visit_top_def(&mut self, top_def: &TopDef) -> tast::TTopDef {
        let kind = match &top_def.kind {
            TopDefKind::Func(func_def) => {
                tast::TTopDefKind::Func(self.visit_function_definition(func_def))
            }
            TopDefKind::Class(class_def) => {
                tast::TTopDefKind::Class(self.visit_class_definition(class_def))
            }
        };
        tast::TTopDef {
            kind,
            position: top_def.position,
        }
    }
    pub fn visit_function_definition(&mut self, func: &FuncDef) -> tast::TFuncDef {
        self.ctx.symbol_table.enter_scope();
        self.ctx.validate_function_return_type(&func.return_type);

        let previous_return_type = self.ctx.current_return_type.take();
        self.ctx.current_return_type = Some(func.return_type.clone());

        // Add arguments to scope.
        for arg in &func.args {
            self.ctx.validate_variable_type(&arg.arg_type);
            self.ctx.define_variable(arg.name, arg.arg_type.clone());
        }

        let typed_body = StatementChecker::new(self.ctx).check_block(&func.body);

        // Check if function is guaranteed to return.
        if !matches!(func.return_type.kind, TypeKind::Void)
            && !ReturnChecker::function_guarantees_return(&func.body)
        {
            self.ctx.add_error(SemanticError::MissingReturn {
                function_name: func.name.name,
                position: func.position,
            });
        }

        self.ctx.current_return_type = previous_return_type;
        self.ctx.symbol_table.leave_scope();

        tast::TFuncDef {
            name: func.name,
            return_type: func.return_type.clone(),
            args: func.args.clone(),
            body: typed_body,
            position: func.position,
        }
    }

    fn visit_class_definition(&mut self, class: &ClassDef) -> tast::TClassDef {
        self.ctx.current_class = Some(class.name.name);

        let mut typed_entries = vec![];
        for entry in &class.entries {
            let typed_entry_kind = match &entry.kind {
                ClassEntryKind::Method(func_def) => {
                    self.ctx.symbol_table.enter_scope();

                    // Define class fields in scope, including superclasses.
                    let all_fields = self.ctx.symbol_table.get_all_class_fields(class.name.name);
                    for (name, (typ, position)) in all_fields {
                        self.ctx
                            .define_variable(Ident { name, position }, typ.clone());
                    }

                    let typed_method =
                        tast::TClassEntryKind::Method(self.visit_function_definition(func_def));
                    self.ctx.symbol_table.leave_scope();
                    typed_method
                }
                ClassEntryKind::Field(idents, typ) => {
                    tast::TClassEntryKind::Field(idents.clone(), typ.clone())
                }
            };

            typed_entries.push(tast::TClassEntry {
                kind: typed_entry_kind,
                position: entry.position,
            });
        }
        self.ctx.current_class = None;

        tast::TClassDef {
            name: class.name,
            extends: class.extends,
            entries: typed_entries,
            position: class.position,
        }
    }

    fn validate_main(&mut self) {
        let main_ident = self.ctx.mock_identifier(StrID::from("main"));
        match self.ctx.symbol_table.lookup_function(&main_ident) {
            Ok(signature) => {
                if !matches!(signature.return_type.kind, TypeKind::Int) {
                    self.ctx.add_error(SemanticError::WrongMainReturnType {
                        expected: StrID::from("int"),
                        found: StrID::from(signature.return_type.to_string()),
                        position: signature.ident.position,
                    });
                }
                if !signature.params.is_empty() {
                    self.ctx
                        .add_error(SemanticError::WrongNumberOfMainArguments {
                            found: signature.params.len(),
                            position: signature.ident.position,
                        });
                }
            }
            Err(_) => {
                self.ctx.add_error(SemanticError::UndefinedMain);
            }
        }
    }
}
