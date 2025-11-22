// TODO: Remove those eventually.
#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::HashMap;
use std::vec;

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
        checker.visit_program(program)?;
        // TODO: Do the error handling nicely.
        if checker.errors.is_empty() {
            Ok(())
        } else {
            Err(checker.errors)
        }
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
        // Mute out the error types.
        if matches!(subtype.kind, TypeKind::Error) || matches!(supertype.kind, TypeKind::Error) {
            return true;
        }

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

        if let (TypeKind::Array(sub_inner), TypeKind::Array(super_inner)) =
            (&subtype.kind, &supertype.kind)
        {
            return self.is_subtype(sub_inner, super_inner);
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
                ClassEntryKind::Field(identifiers, typ) => {
                    for ident in identifiers {
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

    fn type_legal_for_bin_op(&mut self, op: &BinaryOp, typ: &Type) -> bool {
        match op {
            BinaryOp::Add => matches!(typ.kind, TypeKind::Int | TypeKind::String),
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                matches!(typ.kind, TypeKind::Int)
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                matches!(typ.kind, TypeKind::Int)
            }
            BinaryOp::Eq | BinaryOp::Neq => true,
            BinaryOp::And | BinaryOp::Or => {
                matches!(typ.kind, TypeKind::Bool)
            }
        }
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
            TopDefKind::Func(func_def) => Ok(self.visit_function_definition(func_def)?),
            TopDefKind::Class(class_def) => Ok(self.visit_class_definition(class_def)?),
        }
    }

    pub fn visit_function_definition(&mut self, func: &FuncDef) -> Result<(), Vec<SemanticError>> {
        self.symbol_table.enter_scope();
        // TODO: Try to use parametrized lifetime here to avoid cloning?
        self.current_return_type = Some(func.return_type.clone());

        // TODO: Here we should look for all function definitions and handle them.

        // Add arguments to scope.
        for arg in &func.args {
            self.validate_variable_type(&arg.arg_type)?;
            self.symbol_table.define_variable(VariableInfo {
                ident: arg.name.clone(),
                typ: arg.arg_type.clone(),
            })?;
        }

        self.visit_block(&func.body)?;

        // Check if function is guaranteed to return.
        if !matches!(func.return_type.kind, TypeKind::Void) {
            let mut guarantees_return = false;
            for stmt in &func.body.stmts {
                if self.check_stmt_returns(stmt) {
                    guarantees_return = true;
                    break;
                }
            }

            if !guarantees_return {
                self.add_error(SemanticError::MissingReturn {
                    function_name: func.name.name.clone(),
                    position: func.position,
                });
            }
        }

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
    /// Export that to ReturnChecker
    fn check_stmt_returns(&self, stmt: &Stmt) -> bool {
        match &stmt.kind {
            StmtKind::Return { expr } => true,
            StmtKind::Block(block) => {
                for stmt in &block.stmts {
                    if self.check_stmt_returns(stmt) {
                        return true;
                    }
                }
                false
            }
            StmtKind::If {
                cond,
                if_true,
                if_false,
            } => {
                match &cond.kind {
                    ExprKind::LiteralTrue => {
                        // if(true)
                        // Check only the true branch.
                        self.check_stmt_returns(&if_true)
                    }
                    ExprKind::LiteralFalse => {
                        // if(false)
                        if let Some(else_stmt) = if_false {
                            self.check_stmt_returns(&else_stmt)
                        } else {
                            false
                        }
                    }
                    _ => {
                        let true_returns = self.check_stmt_returns(if_true);
                        match if_false {
                            Some(else_stmt) => {
                                let false_returns = self.check_stmt_returns(&else_stmt);
                                true_returns && false_returns
                            }
                            None => false,
                        }
                    }
                }
            }

            // TODO: Think about that
            StmtKind::While { cond, body } => {
                // TODO: This is temporary. We want to evaluate expressions in a smarter way than that.
                return matches!(cond.kind, ExprKind::LiteralTrue);
            }
            // TODO: For now it doesn't work for fors as it's not required. Change that
            StmtKind::For { .. } => false,
            StmtKind::ForEach { .. } => false,
            _ => false,
        }
    }

    // TODO: All of these should probably not be pub.
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
            StmtKind::Expr(expr) => {
                self.visit_expr(expr)?;
                Ok(())
            }
            StmtKind::FuncDef(func_def) => self.visit_function_definition(func_def),
        }
    }

    pub fn visit_block(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        self.symbol_table.enter_scope();
        // TODO: Here we should:
        // - Look for defined functions
        for stmt in &block.stmts {
            self.visit_statement(&stmt)?;
        }
        self.symbol_table.leave_scope();
        Ok(())
    }

    pub fn visit_declaration(
        &mut self,
        typ: &Type,
        items: &Vec<Item>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        self.validate_variable_type(typ)?;
        for item in items {
            match &item.kind {
                ItemKind::NoInit(ident) => {
                    let var_info = VariableInfo {
                        ident: ident.clone(),
                        typ: typ.clone(),
                    };
                    self.symbol_table.define_variable(var_info)?;
                }
                ItemKind::Init(ident, expr) => {
                    let expr_type = self.visit_expr(expr)?;
                    if !self.is_subtype(&expr_type, typ) {
                        self.add_error(SemanticError::DeclarationTypeMismatch {
                            var_name: ident.name.clone(),
                            expected: typ.to_string(),
                            decl_type_position: typ.position,
                            found: expr_type.to_string(),
                            expr_position: expr.position,
                        });
                    }
                    self.symbol_table.define_variable(VariableInfo {
                        ident: ident.clone(),
                        typ: typ.clone(),
                    })?;
                }
            }
        }
        Ok(())
    }

    pub fn visit_assign(
        &mut self,
        target: &Box<Expr>,
        value: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        let is_lvalue = matches!(
            target.kind,
            ExprKind::Var(_) | ExprKind::ArrayAccess { .. } | ExprKind::FieldAccess { .. }
        );

        if !is_lvalue {
            self.add_error(SemanticError::InvalidLValue {
                position: target.position,
            });
            return Ok(());
        }

        let target_type = self.visit_expr(&target)?;
        let value_type = self.visit_expr(&value)?;

        // TODO: Would be nice to seek where was the lhs defined
        if !self.is_subtype(&value_type, &target_type) {
            self.add_error(SemanticError::AssignTypeMismatch {
                lhs_type: target_type.to_string(),
                rhs_type: value_type.to_string(),
                lhs_position: target_type.position,
                rhs_position: value_type.position,
            });
        }

        Ok(())
    }

    pub fn visit_return(
        &mut self,
        expr: &Option<Box<Expr>>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        let return_type = self
            .current_return_type
            .clone()
            .expect("Internal error: visit_return() with no expected return type");

        match expr {
            Some(expr) => {
                let expr_type = self.visit_expr(&expr)?;

                // Can't return void expression.
                if matches!(expr_type.kind, TypeKind::Void) {
                    self.add_error(SemanticError::InvalidVoidReturn {
                        ret_type_position: expr_type.position,
                    });
                }

                if !self.is_subtype(&expr_type, &return_type) {
                    self.add_error(SemanticError::InvalidReturnValue {
                        expected: return_type.to_string(),
                        ret_type_position: return_type.position,
                        found: expr_type.to_string(),
                        position: expr.position,
                    });
                }
            }
            None => {
                if !matches!(return_type.kind, TypeKind::Void) {
                    self.add_error(SemanticError::InvalidReturnValue {
                        expected: return_type.to_string(),
                        ret_type_position: return_type.position,
                        found: "void".to_string(),
                        position,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn visit_increment(
        &mut self,
        expr: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        let expr_type = self.visit_expr(expr)?;
        if !matches!(expr_type.kind, TypeKind::Int) {
            self.add_error(SemanticError::InvalidOperands {
                operator: "++".to_string(),
                operand: expr_type.to_string(),
                position: expr.position,
                type_position: expr_type.position,
            });
        }

        let is_lvalue = matches!(
            expr.kind,
            ExprKind::Var(_) | ExprKind::ArrayAccess { .. } | ExprKind::FieldAccess { .. }
        );
        if !is_lvalue {
            self.add_error(SemanticError::InvalidLValue {
                position: expr.position,
            });
        }

        Ok(())
    }

    pub fn visit_decrement(
        &mut self,
        expr: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        let expr_type = self.visit_expr(expr)?;
        if !matches!(expr_type.kind, TypeKind::Int) {
            self.add_error(SemanticError::InvalidOperands {
                operator: "--".to_string(),
                operand: expr_type.to_string(),
                position: expr.position,
                type_position: expr_type.position,
            });
        }

        let is_lvalue = matches!(
            expr.kind,
            ExprKind::Var(_) | ExprKind::ArrayAccess { .. } | ExprKind::FieldAccess { .. }
        );
        if !is_lvalue {
            self.add_error(SemanticError::InvalidLValue {
                position: expr.position,
            });
        }

        Ok(())
    }

    pub fn visit_if(
        &mut self,
        cond: &Box<Expr>,
        if_true: &Box<Stmt>,
        if_false: &Option<Box<Stmt>>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        let cond_type = self.visit_expr(cond)?;
        if !matches!(cond_type.kind, TypeKind::Bool) {
            self.add_error(SemanticError::TypeMismatch {
                expected: "boolean".to_string(),
                found: cond_type.to_string(),
                position: cond.position,
            });
        }
        self.symbol_table.enter_scope();
        self.visit_statement(&if_true)?;
        if let Some(if_false) = if_false {
            self.visit_statement(&if_false)?;
        }
        self.symbol_table.leave_scope();

        Ok(())
    }

    pub fn visit_while(
        &mut self,
        cond: &Box<Expr>,
        body: &Box<Stmt>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        let cond_type = self.visit_expr(cond)?;
        if !matches!(cond_type.kind, TypeKind::Bool) {
            self.add_error(SemanticError::TypeMismatch {
                expected: "boolean".to_string(),
                found: cond_type.to_string(),
                position: cond.position,
            });
        }
        self.symbol_table.enter_scope();
        self.visit_statement(&body)?;
        self.symbol_table.leave_scope();
        Ok(())
    }

    pub fn visit_for(
        &mut self,
        init: &Option<Box<Stmt>>,
        cond: &Option<Box<Expr>>,
        post: &Option<Box<Expr>>,
        body: &Box<Stmt>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        self.symbol_table.enter_scope();

        if let Some(init_stmt) = init {
            // The init variable will be defined in `visit_statement`.
            self.visit_statement(&init_stmt)?;
        }

        if let Some(cond_expr) = cond {
            let cond_type = self.visit_expr(&cond_expr)?;
            if !matches!(cond_type.kind, TypeKind::Bool) {
                self.add_error(SemanticError::TypeMismatch {
                    expected: "boolean".to_string(),
                    found: cond_type.to_string(),
                    position: cond_expr.position,
                });
            }
        }

        if let Some(post_expr) = post {
            self.visit_expr(&post_expr)?;
        }

        self.visit_statement(&body)?;
        self.symbol_table.leave_scope();
        Ok(())
    }

    pub fn visit_for_each(
        &mut self,
        element_type: &Type,
        item_name: &Ident,
        collection: &Box<Expr>,
        body: &Box<Stmt>,
        position: SourcePosition,
    ) -> Result<(), Vec<SemanticError>> {
        let collection_type = self.visit_expr(&collection)?;
        if let TypeKind::Array(inner) = &collection_type.kind {
            if !self.is_subtype(&inner, &element_type) {
                self.add_error(SemanticError::ForEachTypeMismatch {
                    variable_type: element_type.to_string(),
                    collection_item_type: inner.to_string(),
                    variable_position: element_type.position,
                    collection_position: collection.position,
                });
            }
        } else {
            self.add_error(SemanticError::TypeMismatch {
                expected: format!("{}[]", element_type.to_string()),
                found: collection_type.to_string(),
                position: collection.position,
            });
        }

        self.symbol_table.enter_scope();
        self.symbol_table.define_variable(VariableInfo {
            ident: item_name.clone(),
            typ: element_type.clone(),
        })?;
        self.visit_statement(body)?;
        self.symbol_table.leave_scope();
        Ok(())
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
    pub fn visit_expr(&mut self, outer_expr: &Box<Expr>) -> Result<Type, Vec<SemanticError>> {
        match &outer_expr.kind {
            ExprKind::Var(ident) => Ok(self.symbol_table.lookup_variable(ident)?),
            ExprKind::LiteralInt(_) => Ok(Type {
                kind: TypeKind::Int,
                position: outer_expr.position,
            }),
            ExprKind::LiteralTrue => Ok(Type {
                kind: TypeKind::Bool,
                position: outer_expr.position,
            }),
            ExprKind::LiteralFalse => Ok(Type {
                kind: TypeKind::Bool,
                position: outer_expr.position,
            }),
            ExprKind::LiteralString(_) => Ok(Type {
                kind: TypeKind::String,
                position: outer_expr.position,
            }),
            ExprKind::LiteralNull => Ok(Type {
                kind: TypeKind::Null,
                position: outer_expr.position,
            }),
            ExprKind::App { callee, args } => {
                self.visit_application(callee, args, outer_expr.position)
            }
            ExprKind::ArrayAccess { array, index } => {
                self.visit_array_access(array, index, outer_expr.position)
            }
            ExprKind::FieldAccess { object, field_name } => {
                self.visit_field_access(object, field_name, outer_expr.position)
            }
            ExprKind::Method {
                object,
                method_name,
                args,
            } => self.visit_method_call(object, method_name, args, outer_expr.position),
            ExprKind::Paren(inner) => self.visit_expr(inner),
            ExprKind::UnaryOp { op, expr } => {
                self.visit_unary_operator(op, expr, outer_expr.position)
            }
            ExprKind::BinaryOp { op, lhs, rhs } => {
                self.visit_binary_operator(op, lhs, rhs, outer_expr.position)
            }
            ExprKind::NewObject(typ) => {
                self.validate_variable_type(typ)?;
                Ok(typ.clone())
            }
            ExprKind::NewArray { element_type, size } => {
                self.validate_variable_type(element_type)?;
                let size_type = self.visit_expr(&size)?;
                // TODO: Save the array size somewhere to static check the array access.
                if !matches!(size_type.kind, TypeKind::Int) {
                    self.add_error(SemanticError::TypeMismatch {
                        expected: "int".to_string(),
                        found: size_type.to_string(),
                        position: size.position,
                    });
                }

                Ok(Type {
                    kind: TypeKind::Array(Box::new(element_type.clone())),
                    position: outer_expr.position,
                })
            }
            ExprKind::TypedNull(typ) => {
                self.validate_variable_type(&typ)?;

                // TODO: Test for (null)null
                if matches!(typ.kind, TypeKind::Bool | TypeKind::Int | TypeKind::Void) {
                    let expected = vec![
                        "{ClassType}".to_string(),
                        "{ArrayType}".to_string(),
                        "{string}".to_string(),
                    ];
                    self.add_error(SemanticError::InvalidNullCast {
                        found: typ.to_string(),
                        expected,
                        type_position: typ.position,
                        expr_position: outer_expr.position,
                    });
                }

                Ok(typ.clone())
            }
            ExprKind::ExprTypedNull(expr) => {
                let expected = vec![
                    "{ClassType}".to_string(),
                    "{ArrayType}".to_string(),
                    "{string}".to_string(),
                ];
                match &expr.kind {
                    ExprKind::Var(ident) => {
                        if !self.symbol_table.class_exists(&ident.name) {
                            self.add_error(SemanticError::UndefinedClass {
                                name: ident.name.clone(),
                                position: ident.position,
                            });
                            Ok(Type {
                                kind: TypeKind::Error,
                                position: outer_expr.position,
                            })
                        } else {
                            Ok(Type {
                                kind: TypeKind::Class(ident.clone()),
                                position: ident.position,
                            })
                        }
                    }
                    ExprKind::LiteralString(_) => Ok(Type {
                        kind: TypeKind::String,
                        position: outer_expr.position,
                    }),
                    _ => {
                        self.add_error(SemanticError::InvalidNullCastExpr {
                            expected,
                            cast_position: expr.position,
                            expr_position: outer_expr.position,
                        });
                        Ok(Type {
                            kind: TypeKind::Error,
                            position: outer_expr.position,
                        })
                    }
                }
            }
        }
    }

    fn validate_function_call(
        &mut self,
        func_info: &FunctionInfo,
        args: &Vec<Box<Expr>>,
        expr_position: SourcePosition,
    ) -> Result<bool, Vec<SemanticError>> {
        let func_ident = func_info.ident.clone();
        if args.len() != func_info.params.len() {
            self.add_error(SemanticError::WrongNumberOfArguments {
                name: func_ident.name.clone(),
                expected: func_info.params.len(),
                found: args.len(),
                call_position: expr_position,
                function_position: func_info.position,
            });
            return Ok(false);
        }

        // Type check arguments.
        for (expr, param) in args.iter().zip(&func_info.params) {
            let arg_type = self.visit_expr(expr)?;
            if !self.is_subtype(&arg_type, &param) {
                self.add_error(SemanticError::ArgumentTypeMismatch {
                    expected: param.to_string(),
                    found: arg_type.to_string(),
                    position: expr.position,
                    function_name: func_info.ident.name.clone(),
                    argument_position: param.position,
                });
                return Ok(false);
            }
        }
        return Ok(true);
    }

    fn visit_application(
        &mut self,
        callee: &Box<Expr>,
        args: &Vec<Box<Expr>>, // TODO: Remove box expr.
        position: SourcePosition,
    ) -> Result<Type, Vec<SemanticError>> {
        // TODO: For now we only support direct calls on function names. Change that to use function types.
        if let ExprKind::Var(name) = &callee.kind {
            let func_info = self.symbol_table.lookup_function(&name)?;

            self.validate_function_call(&func_info, &args, position)?;

            // Function call okay.
            return Ok(Type {
                kind: func_info.return_type.kind,
                position,
            });
        }

        Ok(Type {
            kind: TypeKind::Error,
            position,
        })
    }

    fn visit_array_access(
        &mut self,
        array: &Box<Expr>,
        index: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<Type, Vec<SemanticError>> {
        let array_type = self.visit_expr(&array)?;
        let index_type = self.visit_expr(&index)?;

        if !matches!(index_type.kind, TypeKind::Int) {
            self.add_error(SemanticError::TypeMismatch {
                expected: "int".to_string(),
                found: index_type.to_string(),
                position: index.position,
            });
        }

        match &array_type.kind {
            TypeKind::Array(elem_type) => Ok((**elem_type).clone()),
            _ => {
                self.add_error(SemanticError::InvalidOperands {
                    operator: "[]".to_string(),
                    operand: array_type.to_string(),
                    position,
                    type_position: array_type.position,
                });
                Ok(Type {
                    kind: TypeKind::Error,
                    position,
                })
            }
        }
    }

    fn visit_field_access(
        &mut self,
        object: &Box<Expr>,
        field_name: &Ident,
        position: SourcePosition,
    ) -> Result<Type, Vec<SemanticError>> {
        let obj_type = self.visit_expr(&object)?;

        if let TypeKind::Class(class_name) = obj_type.kind {
            let all_fields = self.symbol_table.get_all_class_fields(&class_name.name);

            if let Some((field_type, _)) = all_fields.get(&field_name.name) {
                return Ok(field_type.clone());
            } else {
                self.add_error(SemanticError::UndefinedField {
                    class_name: class_name.name,
                    field_name: field_name.name.clone(),
                    position,
                    class_position: class_name.position,
                });
                return Ok(Type {
                    kind: TypeKind::Error,
                    position,
                });
            }
        } else if let TypeKind::String | TypeKind::Array(_) = obj_type.kind {
            // TODO: Extract the legal type field into a module.
            if field_name.name != "length" {
                self.add_error(SemanticError::UndefinedField {
                    class_name: obj_type.to_string(),
                    field_name: field_name.name.clone(),
                    position,
                    class_position: SourcePosition(0, 0),
                });
                return Ok(Type {
                    kind: TypeKind::Error,
                    position: position,
                });
            } else {
                return Ok(Type {
                    kind: TypeKind::Int,
                    position: position,
                });
            }
        } else {
            self.add_error(SemanticError::InvalidOperands {
                operator: ".".to_string(),
                operand: obj_type.to_string(),
                position: position,
                type_position: obj_type.position,
            });
            return Ok(Type {
                kind: TypeKind::Error,
                position: position,
            });
        }
    }

    // TODO: Probably remove that
    fn visit_method_call(
        &mut self,
        callee: &Box<Expr>,
        method_name: &Ident,
        args: &Vec<Box<Expr>>,
        position: SourcePosition,
    ) -> Result<Type, Vec<SemanticError>> {
        let obj_type = self.visit_expr(callee)?;

        if let TypeKind::Class(class_name) = obj_type.kind {
            let all_methods = self.symbol_table.get_all_class_methods(&class_name.name);

            if let Some(method_info) = all_methods.get(&method_name.name) {
                // self.validate_function_call(&method_info, args, position)?;

                // Function call okay.
                return Ok(Type {
                    kind: method_info.return_type.kind.clone(),
                    position,
                });
            } else {
                self.add_error(SemanticError::UndefinedMethod {
                    class_name: class_name.name,
                    method_name: method_name.name.clone(),
                    position,
                    class_position: class_name.position,
                });
                return Ok(Type {
                    kind: TypeKind::Error,
                    position,
                });
            }
        }

        Ok(Type {
            kind: TypeKind::Error,
            position,
        })
    }

    fn visit_unary_operator(
        &mut self,
        op: &UnaryOp,
        expr: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<Type, Vec<SemanticError>> {
        let expr_type = self.visit_expr(&expr)?;
        match op {
            UnaryOp::Neg => {
                if !matches!(expr_type.kind, TypeKind::Int) {
                    self.add_error(SemanticError::InvalidOperands {
                        operator: op.to_string(),
                        operand: expr_type.to_string(),
                        position: expr.position,
                        type_position: expr_type.position,
                    });
                }
                Ok(Type {
                    kind: TypeKind::Int,
                    position: expr.position,
                })
            }
            UnaryOp::Not => {
                if !matches!(expr_type.kind, TypeKind::Bool) {
                    self.add_error(SemanticError::InvalidOperands {
                        operator: "!".to_string(),
                        operand: expr_type.to_string(),
                        position: expr.position,
                        type_position: expr_type.position,
                    });
                }
                Ok(Type {
                    kind: TypeKind::Bool,
                    position: expr.position,
                })
            }
        }
    }

    fn visit_binary_operator(
        &mut self,
        op: &BinaryOp,
        lhs: &Box<Expr>,
        rhs: &Box<Expr>,
        position: SourcePosition,
    ) -> Result<Type, Vec<SemanticError>> {
        let lhs_type = self.visit_expr(&lhs)?;
        let rhs_type = self.visit_expr(&rhs)?;

        // Check if left operand if valid for this op.
        if !self.type_legal_for_bin_op(op, &lhs_type) {
            self.add_error(SemanticError::InvalidOperands {
                operator: op.to_string(),
                operand: lhs_type.to_string(),
                position: lhs.position,
                type_position: lhs_type.position,
            });
        }
        // Check if right operand if valid for this op.
        if !self.type_legal_for_bin_op(op, &rhs_type) {
            self.add_error(SemanticError::InvalidOperands {
                operator: op.to_string(),
                operand: rhs_type.to_string(),
                position: rhs.position,
                type_position: rhs_type.position,
            });
        }

        match op {
            BinaryOp::Eq | BinaryOp::Neq => {
                let compatible =
                    self.is_subtype(&lhs_type, &rhs_type) || self.is_subtype(&rhs_type, &lhs_type);

                if !compatible {
                    self.add_error(SemanticError::BinaryOpTypeMismatch {
                        lhs_type: lhs_type.to_string(),
                        lhs_position: lhs_type.position,
                        rhs_type: rhs_type.to_string(),
                        rhs_position: rhs_type.position,
                    });
                }
            }
            _ => {
                if !self.types_equal(&lhs_type, &rhs_type) {
                    self.add_error(SemanticError::BinaryOpTypeMismatch {
                        lhs_type: lhs_type.to_string(),
                        lhs_position: lhs_type.position,
                        rhs_type: rhs_type.to_string(),
                        rhs_position: rhs_type.position,
                    });
                }
            }
        }

        match op {
            // Result of this binary is the same as the operands type.
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => Ok(Type {
                kind: lhs_type.kind,
                position,
            }),
            // Result of this binary is the same as the operands type.
            BinaryOp::Div | BinaryOp::Mod => {
                if let ExprKind::LiteralInt(0) = rhs.kind {
                    self.add_error(SemanticError::ZeroDivision {
                        position: rhs.position,
                    });
                }
                Ok(Type {
                    kind: lhs_type.kind,
                    position,
                })
            }
            BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Eq // TODO: Maybe we should check if rhs is a subtype of lhs or other way around in case of Neq and Eq.
            | BinaryOp::Neq
            | BinaryOp::And
            | BinaryOp::Or => Ok(Type {
                kind: TypeKind::Bool,
                position,
            }),
        }
    }
}
