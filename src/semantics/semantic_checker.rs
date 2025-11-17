// TODO: Remove those eventually.
#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::HashMap;

use crate::diagnostics::SemanticError;
use crate::parser::ast::*;
use crate::semantics::symbol_table::SymbolTable;
use crate::semantics::{ClassInfo, FunctionInfo, VariableInfo};

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
    /// Forward declare a class and validate correctness of class definition.
    /// This function does not validate the inheritance hierarchy. This is done in validate_class.
    fn add_class(&mut self, class_def: &ClassDef) -> Result<(), Vec<SemanticError>> {
        let mut fields: HashMap<String, (Type, SourcePosition)> = HashMap::new();
        let mut methods: HashMap<String, FunctionInfo> = HashMap::new();

        // Check for duplicate fields and methods. Extends are not validated yet since we don't know the hierarchy.
        // TODO: Can fields be overridden from a superclass?
        for entry in &class_def.entries {
            match &entry.kind {
                ClassEntryKind::Field(ident, typ) => {
                    if fields.contains_key(&ident.name) {
                        self.add_error(SemanticError::DuplicateClassField {
                            class_name: class_def.name.name.clone(),
                            field_name: ident.name.clone(),
                            position: ident.position,
                            previous_position: fields[&ident.name].1,
                        });
                    } else {
                        fields.insert(ident.name.clone(), (typ.clone(), ident.position));
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
                        self.add_error(SemanticError::DuplicateMethod {
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
        self.symbol_table.define_class(class_info)?;

        Ok(())
    }

    /// Forward declare a function.
    fn add_function(&mut self, func_def: &FuncDef) -> Result<(), Vec<SemanticError>> {
        let signature = FunctionInfo {
            ident: func_def.name.clone(),
            return_type: func_def.return_type.clone(),
            // TODO: We may need arg names as well. Not only types.
            params: func_def
                .args
                .iter()
                .map(|arg| arg.arg_type.clone())
                .collect(),
            position: func_def.position,
        };
        self.symbol_table.define_function(signature)?;
        Ok(())
    }

    fn validate_class(&mut self, class_def: &ClassDef) -> Result<(), Vec<SemanticError>> {
        // TODO: Add inheritance hierarchy cycles detection.

        // Undefined superclass.
        if let Some(parent) = &class_def.extends {
            if !self.symbol_table.class_exists(&parent.name) {
                self.add_error(SemanticError::UndefinedClass {
                    name: parent.name.clone(),
                    position: parent.position,
                });
            }
        }

        for entry in &class_def.entries {
            if let ClassEntryKind::Field(_, typ) = &entry.kind {
                self.validate_variable_type(&typ)?;
            }
        }

        Ok(())
    }

    /// TODO: Try to unify the validate_type functions.
    /// Validates the types assuming the type is used to declare a variable of a class field.
    /// This means void and null type will be treated as an error.
    fn validate_variable_type(&mut self, typ: &Type) -> Result<(), Vec<SemanticError>> {
        let expected_types = vec![
            "int".to_string(),
            "bool".to_string(),
            "string".to_string(),
            "{ClassType}".to_string(),
            "{ArrayType}".to_string(),
        ];
        match &typ.kind {
            TypeKind::Void => {
                self.add_error(SemanticError::InvalidVariableType {
                    found: typ.to_string(),
                    expected: expected_types,
                    position: typ.position,
                });
            }
            TypeKind::Null => {
                self.add_error(SemanticError::InvalidVariableType {
                    found: typ.to_string(),
                    expected: expected_types,
                    position: typ.position,
                });
            }
            TypeKind::Class(ident) => {
                if !self.symbol_table.class_exists(&ident.name) {
                    self.add_error(SemanticError::UndefinedClass {
                        name: ident.name.clone(),
                        position: ident.position,
                    });
                }
            }
            TypeKind::Array(inner) => {
                if let TypeKind::Void = inner.kind {
                    self.add_error(SemanticError::InvalidVoidArrayType {
                        expected: vec![
                            "int".to_string(),
                            "bool".to_string(),
                            "string".to_string(),
                            "{ClassType}".to_string(),
                            "{ArrayType}".to_string(),
                        ],
                        position: inner.position,
                    });
                } else {
                    self.validate_variable_type(&inner)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Validates the types assuming the type is used to declare a functions return type.
    /// This means void and null type will be treated as an error.
    fn validate_functions_return_type(&mut self, typ: &Type) -> Result<(), Vec<SemanticError>> {
        let expected_types = vec![
            "void".to_string(),
            "int".to_string(),
            "bool".to_string(),
            "string".to_string(),
            "{ClassType}".to_string(),
            "{ArrayType}".to_string(),
        ];
        match &typ.kind {
            TypeKind::Null => {
                self.add_error(SemanticError::InvalidVariableType {
                    found: typ.to_string(),
                    expected: expected_types,
                    position: typ.position,
                });
            }
            TypeKind::Class(ident) => {
                if !self.symbol_table.class_exists(&ident.name) {
                    self.add_error(SemanticError::UndefinedClass {
                        name: ident.name.clone(),
                        position: ident.position,
                    });
                }
            }
            TypeKind::Array(inner) => {
                if let TypeKind::Void = inner.kind {
                    self.add_error(SemanticError::InvalidVoidArrayType {
                        expected: vec![
                            "void".to_string(),
                            "int".to_string(),
                            "bool".to_string(),
                            "string".to_string(),
                            "{ClassType}".to_string(),
                            "{ArrayType}".to_string(),
                        ],
                        position: inner.position,
                    });
                } else {
                    self.validate_variable_type(&inner)?;
                }
            }
            _ => {}
        }
        Ok(())
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
            self.visit_top_def(&top_def)?;
        }

        self.validate_main()?;

        Ok(())
    }

    pub fn visit_top_def(&mut self, top_def: &TopDef) -> Result<(), Vec<SemanticError>> {
        match &top_def.kind {
            TopDefKind::Func(func_def) => self.visit_function_definition(func_def)?,
            TopDefKind::Class(class_def) => self.visit_class_definition(class_def)?,
        }
        todo!();
    }

    pub fn visit_function_definition(&mut self, func: &FuncDef) -> Result<(), Vec<SemanticError>> {
        self.symbol_table.enter_scope();
        // TODO: Try to use parametrized lifetime here to avoid cloning?
        self.current_return_type = Some(func.return_type.clone());

        // Add arguments to scope.
        for arg in &func.args {
            self.validate_variable_type(&arg.arg_type)?;
            self.symbol_table.define_variable(VariableInfo {
                ident: arg.name.clone(),
                typ: arg.arg_type.clone(),
            })?;
        }

        self.visit_block(&func.body)?;
        // TODO: This is bugged in nested functions.
        self.current_return_type = None;
        self.symbol_table.leave_scope();

        Ok(())
    }

    pub fn visit_class_definition(&mut self, class: &ClassDef) -> Result<(), Vec<SemanticError>> {
        self.current_class = Some(class.name.name.clone());
        for entry in &class.entries {
            if let ClassEntryKind::Method(method) = &entry.kind {
                self.symbol_table.enter_scope();

                // Define class fields in scope.
                let class_info = self.symbol_table.lookup_class(&class.name)?;
                for (name, (typ, position)) in &class_info.fields {
                    self.symbol_table.define_variable(VariableInfo {
                        ident: Ident {
                            name: name.clone(),
                            position: *position,
                        },
                        typ: typ.clone(),
                    })?;
                }

                self.visit_function_definition(method)?;
                self.symbol_table.leave_scope();
            }
        }
        self.current_class = None;
        Ok(())
    }
}

// ======================
// Statements
// ======================
impl SemanticChecker {
    pub fn visit_statement(&mut self, stmt: &Stmt) -> Result<(), Vec<SemanticError>> {
        let position = stmt.position;
        match &stmt.kind {
            StmtKind::Empty => Ok(()),
            StmtKind::Block(block) => self.visit_block(block),
            StmtKind::Decl(typ, items) => self.visit_declaration(typ, items, position),
            StmtKind::Assign { target, value } => self.visit_assign(target, value, position),
            StmtKind::Return { expr } => self.visit_return(expr, position),
            StmtKind::Incr(expr) => self.visit_increment(expr, position),
            StmtKind::Decr(expr) => self.visit_decrement(expr, position),
            StmtKind::If {
                cond,
                if_true,
                if_false,
            } => self.visit_if(cond, if_true, if_false, position),
            StmtKind::While { cond, body } => self.visit_while(cond, body, position),
            StmtKind::For {
                init,
                cond,
                post,
                body,
            } => self.visit_for(init, cond, post, body, position),
            StmtKind::ForEach {
                element_type,
                item_name,
                collection,
                body,
            } => self.visit_for_each(element_type, item_name, collection, body, position),
            StmtKind::Expr(expr) => self.visit_expr_statement(expr, position),
            StmtKind::FuncDef(func_def) => self.visit_function_definition(func_def),
        }
    }

    pub fn visit_block(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_declaration(
        &mut self,
        typ: &Type,
        items: &Vec<Item>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_assign(
        &mut self,
        target: &Box<Expr>,
        value: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_return(
        &mut self,
        block: &Option<Box<Expr>>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_increment(
        &mut self,
        expr: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_decrement(
        &mut self,
        expr: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_if(
        &mut self,
        cond: &Box<Expr>,
        if_true: &Box<Stmt>,
        if_false: &Option<Box<Stmt>>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_while(
        &mut self,
        cond: &Box<Expr>,
        body: &Box<Stmt>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_for(
        &mut self,
        init: &Option<Box<Stmt>>,
        cond: &Option<Box<Expr>>,
        post: &Option<Box<Expr>>,
        body: &Box<Stmt>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_for_each(
        &mut self,
        element_type: &Type,
        item_name: &Ident,
        collection: &Box<Expr>,
        body: &Box<Stmt>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        todo!();
    }

    pub fn visit_expr_statement(
        &mut self,
        expr: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
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
    pub fn visit_expr(&mut self, expr: &Box<Expr>) -> Result<Type, Vec<SemanticError>> {
        todo!();
    }
}
