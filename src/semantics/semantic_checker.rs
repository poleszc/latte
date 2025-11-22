// TODO: Remove those eventually.
#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::HashMap;
use std::vec;

use crate::diagnostics::SemanticError;
use crate::parser::ast::*;
use crate::semantics::return_checker::ReturnChecker;
use crate::semantics::symbol_table::SymbolTable;
use crate::semantics::type_checker::TypeChecker;
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
        checker.visit_program(program);

        if checker.errors.is_empty() {
            Ok(())
        } else {
            Err(checker.errors)
        }
    }

    fn add_error(&mut self, error: SemanticError) {
        self.errors.push(error);
    }
}

// ======================
// Types
// ======================
// TODO: Try to move that and make Semantic analyzer store a reference?
impl SemanticChecker {
    fn types_equal(&self, t1: &Type, t2: &Type) -> bool {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.types_equal(t1, t2)
    }

    fn is_subtype(&self, subtype: &Type, supertype: &Type) -> bool {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.is_subtype(subtype, supertype)
    }

    fn validate_variable_type(&mut self, typ: &Type) {
        let type_checker = TypeChecker::new(&self.symbol_table);
        if let Err(e) = type_checker.validate_variable_type(typ) {
            self.add_error(e);
        }
    }

    fn validate_function_return_type(&mut self, typ: &Type) {
        let type_checker = TypeChecker::new(&self.symbol_table);
        if let Err(e) = type_checker.validate_function_return_type(typ) {
            self.add_error(e);
        }
    }

    fn type_legal_for_bin_op(&self, op: &BinaryOp, typ: &Type) -> bool {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.type_legal_for_bin_op(op, typ)
    }

    fn type_legal_for_unary_op(&self, op: &UnaryOp, typ: &Type) -> bool {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.type_legal_for_unary_op(op, typ)
    }

    fn binary_op_result_type(&self, op: &BinaryOp, typ: &Type, position: SourcePosition) -> Type {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.binary_op_result_type(op, typ, position)
    }

    fn unary_op_result_type(&self, op: &UnaryOp, typ: &Type, position: SourcePosition) -> Type {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.unary_op_result_type(op, position)
    }

    fn error_type(&self, pos: SourcePosition) -> Type {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.error_type(pos)
    }
}

// ======================
// Misc
// ======================
impl SemanticChecker {
    fn validate_function_call(
        &mut self,
        func_info: &FunctionInfo,
        args: &Vec<Box<Expr>>,
        expr_position: SourcePosition,
    ) {
        let func_ident = func_info.ident.clone();
        if args.len() != func_info.params.len() {
            self.add_error(SemanticError::WrongNumberOfArguments {
                name: func_ident.name.clone(),
                expected: func_info.params.len(),
                found: args.len(),
                call_position: expr_position,
                function_position: func_info.position,
            });
            // Don't return here, check types of what we have if possible or just mismatch
        }

        // Type check arguments.
        for (expr, param) in args.iter().zip(&func_info.params) {
            let arg_type = self.visit_expr(expr);
            if !self.is_subtype(&arg_type, &param) {
                self.add_error(SemanticError::ArgumentTypeMismatch {
                    expected: param.to_string(),
                    found: arg_type.to_string(),
                    position: expr.position,
                    function_name: func_info.ident.name.clone(),
                    argument_position: param.position,
                });
            }
        }
    }

    /// Validate that operands are valid for the binary operation.
    fn validate_binary_operands(
        &mut self,
        op: &BinaryOp,
        lhs_type: &Type,
        rhs_type: &Type,
        lhs_position: SourcePosition,
        rhs_position: SourcePosition,
    ) {
        // Check if left operand is valid for this operation.
        if !self.type_legal_for_bin_op(op, lhs_type) {
            self.add_error(SemanticError::InvalidOperands {
                operator: op.to_string(),
                operand: lhs_type.to_string(),
                position: lhs_position,
                type_position: lhs_type.position,
            });
        }

        // Check if right operand is valid for this operation.
        if !self.type_legal_for_bin_op(op, rhs_type) {
            self.add_error(SemanticError::InvalidOperands {
                operator: op.to_string(),
                operand: rhs_type.to_string(),
                position: rhs_position,
                type_position: rhs_type.position,
            });
        }

        // Check type compatibility between operands.
        self.validate_operand_compatibility(op, lhs_type, rhs_type);
    }

    /// Check that the two operands are compatible for the binary operation.
    fn validate_operand_compatibility(&mut self, op: &BinaryOp, lhs_type: &Type, rhs_type: &Type) {
        match op {
            // Equality operators allow subtype relationships.
            BinaryOp::Eq | BinaryOp::Neq => {
                let compatible =
                    self.is_subtype(lhs_type, rhs_type) || self.is_subtype(rhs_type, lhs_type);

                if !compatible {
                    self.add_error(SemanticError::BinaryOpTypeMismatch {
                        lhs_type: lhs_type.to_string(),
                        lhs_position: lhs_type.position,
                        rhs_type: rhs_type.to_string(),
                        rhs_position: rhs_type.position,
                    });
                }
            }
            // All other operators require exact type match.
            _ => {
                if !self.types_equal(lhs_type, rhs_type) {
                    self.add_error(SemanticError::BinaryOpTypeMismatch {
                        lhs_type: lhs_type.to_string(),
                        lhs_position: lhs_type.position,
                        rhs_type: rhs_type.to_string(),
                        rhs_position: rhs_type.position,
                    });
                }
            }
        }
    }
}

// ======================
// Classes
// ======================
impl SemanticChecker {
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

        if let Err(e) = self.symbol_table.define_class(class_info) {
            self.add_error(e);
        }
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
        if let Err(e) = self.symbol_table.define_function(signature) {
            self.add_error(e);
        }
    }

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

            let class_info = match self.symbol_table.lookup_class(&ident) {
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
            if !self.symbol_table.class_exists(&parent.name) {
                self.add_error(SemanticError::UndefinedClass {
                    name: parent.name.clone(),
                    position: parent.position,
                });
            }
        }

        if let Some(parent) = &class_def.extends {
            // Check for cycles in the inheritance hierarchy.
            if let Some(cycle_path) = self.detect_inheritance_cycle(&class_def.name.name) {
                // A simple hack to deduplicate the cycle errors. We look for the smallest lexical
                // class name in the cycle and report it only for this.
                let leader_name = cycle_path
                    .iter()
                    .map(|(name, _, _)| name)
                    .min()
                    .expect("Cycle path cannot be empty");

                if &class_def.name.name == leader_name {
                    self.add_error(SemanticError::InheritanceCycle { cycle: cycle_path });
                }
            }

            // Check if all overrides are valid - have the same signature.
            // TODO: Check if the field overrides are okay - have the same type.
            for entry in &class_def.entries {
                if let ClassEntryKind::Method(method_def) = &entry.kind {
                    let method_name = &method_def.name;

                    if let Some(parent_method) = self
                        .symbol_table
                        .lookup_method(&parent.name, &method_name.name)
                    {
                        // TODO: Use types equal? or some other function which returns an enum on what the mismatch was
                        // Argument count.
                        if method_def.args.len() != parent_method.params.len() {
                            self.add_error(SemanticError::InvalidOverride {
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
                            if !self.types_equal(&arg.arg_type, parent_type) {
                                self.add_error(SemanticError::InvalidOverride {
                                    method_name: method_name.name.clone(),
                                    parent_name: parent.name.clone(),
                                    reason: format!("Argument #{} type mismatch", i + 1),
                                    position: method_name.position,
                                });
                                continue;
                            }
                        }

                        // Return type.
                        if !self.types_equal(&method_def.return_type, &parent_method.return_type) {
                            self.add_error(SemanticError::InvalidOverride {
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
                self.validate_variable_type(&typ);
            }
        }
    }

    fn validate_main(&mut self) {
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
    }
}

// ======================
// Top Level Visitors
// ======================
impl SemanticChecker {
    fn visit_program(&mut self, program: &Program) {
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

    fn visit_top_def(&mut self, top_def: &TopDef) {
        match &top_def.kind {
            TopDefKind::Func(func_def) => self.visit_function_definition(func_def),
            TopDefKind::Class(class_def) => self.visit_class_definition(class_def),
        }
    }

    fn visit_function_definition(&mut self, func: &FuncDef) {
        self.symbol_table.enter_scope();

        self.validate_function_return_type(&func.return_type);
        // TODO: Try to use parametrized lifetime here to avoid cloning?
        self.current_return_type = Some(func.return_type.clone());

        // TODO: Here we should look for all nested function definitions and handle them.

        // Add arguments to scope.
        for arg in &func.args {
            self.validate_variable_type(&arg.arg_type);
            if let Err(e) = self.symbol_table.define_variable(VariableInfo {
                ident: arg.name.clone(),
                typ: arg.arg_type.clone(),
            }) {
                self.add_error(e);
            }
        }

        self.visit_block(&func.body);

        // Check if function is guaranteed to return.
        if !matches!(func.return_type.kind, TypeKind::Void) {
            if !ReturnChecker::function_guarantees_return(&func.body) {
                self.add_error(SemanticError::MissingReturn {
                    function_name: func.name.name.clone(),
                    position: func.position,
                });
            }
        }

        // TODO: This is bugged in nested functions.
        self.current_return_type = None;
        self.symbol_table.leave_scope();
    }

    fn visit_class_definition(&mut self, class: &ClassDef) {
        self.current_class = Some(class.name.name.clone());
        for entry in &class.entries {
            if let ClassEntryKind::Method(method) = &entry.kind {
                self.symbol_table.enter_scope();

                // Define class fields in scope, including superclasses.
                let all_fields = self.symbol_table.get_all_class_fields(&class.name.name);
                for (name, (typ, position)) in all_fields {
                    if let Err(e) = self.symbol_table.define_variable(VariableInfo {
                        ident: Ident {
                            name: name.clone(),
                            position: position,
                        },
                        typ: typ.clone(),
                    }) {
                        // Sometimes fields shadow, or semantic logic differs.
                        // Assuming simple shadowing or error reporting:
                        // self.add_error(e);
                        // For now, we just try to define them for the scope.
                    }
                }

                self.visit_function_definition(method);
                self.symbol_table.leave_scope();
            }
        }
        self.current_class = None;
    }
}

// ======================
// Statement Visitors
// ======================
impl SemanticChecker {
    fn visit_statement(&mut self, stmt: &Stmt) {
        let position = stmt.position;
        match &stmt.kind {
            StmtKind::Empty => (),
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
                self.visit_expr(expr);
            }
            StmtKind::FuncDef(func_def) => self.visit_function_definition(func_def),
        }
    }

    fn visit_block(&mut self, block: &Block) {
        self.symbol_table.enter_scope();
        // TODO: Here we should:
        // - Look for defined functions
        for stmt in &block.stmts {
            self.visit_statement(&stmt);
        }
        self.symbol_table.leave_scope();
    }

    fn visit_declaration(&mut self, typ: &Type, items: &Vec<Item>, position: SourcePosition) {
        self.validate_variable_type(typ);
        for item in items {
            match &item.kind {
                ItemKind::NoInit(ident) => {
                    let var_info = VariableInfo {
                        ident: ident.clone(),
                        typ: typ.clone(),
                    };
                    if let Err(e) = self.symbol_table.define_variable(var_info) {
                        self.add_error(e);
                    }
                }
                ItemKind::Init(ident, expr) => {
                    let expr_type = self.visit_expr(expr);
                    if !self.is_subtype(&expr_type, typ) {
                        self.add_error(SemanticError::DeclarationTypeMismatch {
                            var_name: ident.name.clone(),
                            expected: typ.to_string(),
                            decl_type_position: typ.position,
                            found: expr_type.to_string(),
                            expr_position: expr.position,
                        });
                    }
                    if let Err(e) = self.symbol_table.define_variable(VariableInfo {
                        ident: ident.clone(),
                        typ: typ.clone(),
                    }) {
                        self.add_error(e);
                    }
                }
            }
        }
    }

    fn visit_assign(&mut self, target: &Box<Expr>, value: &Box<Expr>, position: SourcePosition) {
        let is_lvalue = matches!(
            target.kind,
            ExprKind::Var(_) | ExprKind::ArrayAccess { .. } | ExprKind::FieldAccess { .. }
        );

        if !is_lvalue {
            self.add_error(SemanticError::InvalidLValue {
                position: target.position,
            });
            // Continue checking rhs even if lhs is invalid lvalue
        }

        let target_type = self.visit_expr(&target);
        let value_type = self.visit_expr(&value);

        if !self.is_subtype(&value_type, &target_type) {
            self.add_error(SemanticError::AssignTypeMismatch {
                lhs_type: target_type.to_string(),
                rhs_type: value_type.to_string(),
                lhs_position: target_type.position,
                rhs_position: value_type.position,
            });
        }
    }

    fn visit_return(&mut self, expr: &Option<Box<Expr>>, position: SourcePosition) {
        let return_type = self
            .current_return_type
            .clone()
            .expect("Internal error: visit_return() with no expected return type");

        match expr {
            Some(expr) => {
                let expr_type = self.visit_expr(&expr);

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
    }

    fn visit_increment(&mut self, expr: &Box<Expr>, position: SourcePosition) {
        let expr_type = self.visit_expr(expr);
        if !matches!(expr_type.kind, TypeKind::Int | TypeKind::Error) {
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
    }

    fn visit_decrement(&mut self, expr: &Box<Expr>, position: SourcePosition) {
        let expr_type = self.visit_expr(expr);
        if !matches!(expr_type.kind, TypeKind::Int | TypeKind::Error) {
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
    }

    fn visit_if(
        &mut self,
        cond: &Box<Expr>,
        if_true: &Box<Stmt>,
        if_false: &Option<Box<Stmt>>,
        position: SourcePosition,
    ) {
        let cond_type = self.visit_expr(cond);
        if !matches!(cond_type.kind, TypeKind::Bool | TypeKind::Error) {
            self.add_error(SemanticError::TypeMismatch {
                expected: "boolean".to_string(),
                found: cond_type.to_string(),
                position: cond.position,
            });
        }
        self.symbol_table.enter_scope();
        self.visit_statement(&if_true);
        if let Some(if_false) = if_false {
            self.visit_statement(&if_false);
        }
        self.symbol_table.leave_scope();
    }

    fn visit_while(&mut self, cond: &Box<Expr>, body: &Box<Stmt>, position: SourcePosition) {
        let cond_type = self.visit_expr(cond);
        if !matches!(cond_type.kind, TypeKind::Bool | TypeKind::Error) {
            self.add_error(SemanticError::TypeMismatch {
                expected: "boolean".to_string(),
                found: cond_type.to_string(),
                position: cond.position,
            });
        }
        self.symbol_table.enter_scope();
        self.visit_statement(&body);
        self.symbol_table.leave_scope();
    }

    fn visit_for(
        &mut self,
        init: &Option<Box<Stmt>>,
        cond: &Option<Box<Expr>>,
        post: &Option<Box<Expr>>,
        body: &Box<Stmt>,
        position: SourcePosition,
    ) {
        self.symbol_table.enter_scope();

        if let Some(init_stmt) = init {
            // The init variable will be defined in `visit_statement`.
            self.visit_statement(&init_stmt);
        }

        if let Some(cond_expr) = cond {
            let cond_type = self.visit_expr(&cond_expr);
            if !matches!(cond_type.kind, TypeKind::Bool | TypeKind::Error) {
                self.add_error(SemanticError::TypeMismatch {
                    expected: "boolean".to_string(),
                    found: cond_type.to_string(),
                    position: cond_expr.position,
                });
            }
        }

        if let Some(post_expr) = post {
            self.visit_expr(&post_expr);
        }

        self.visit_statement(&body);
        self.symbol_table.leave_scope();
    }

    fn visit_for_each(
        &mut self,
        element_type: &Type,
        item_name: &Ident,
        collection: &Box<Expr>,
        body: &Box<Stmt>,
        position: SourcePosition,
    ) {
        let collection_type = self.visit_expr(&collection);
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
        if let Err(e) = self.symbol_table.define_variable(VariableInfo {
            ident: item_name.clone(),
            typ: element_type.clone(),
        }) {
            self.add_error(e);
        }
        self.visit_statement(body);
        self.symbol_table.leave_scope();
    }

    // TODO: Remove that?
    fn visit_function_definition_expr(&mut self, block: &Block) -> Result<(), Vec<SemanticError>> {
        todo!();
    }
}

// ======================
// Expression Visitors
// ======================
impl SemanticChecker {
    /// Type checks an expression and returns it's type.
    fn visit_expr(&mut self, outer_expr: &Box<Expr>) -> Type {
        match &outer_expr.kind {
            ExprKind::Var(ident) => match self.symbol_table.lookup_variable(ident) {
                Ok(t) => t,
                Err(e) => {
                    self.add_error(e);
                    self.error_type(ident.position)
                }
            },
            ExprKind::LiteralInt(_) => Type {
                kind: TypeKind::Int,
                position: outer_expr.position,
            },
            ExprKind::LiteralTrue => Type {
                kind: TypeKind::Bool,
                position: outer_expr.position,
            },
            ExprKind::LiteralFalse => Type {
                kind: TypeKind::Bool,
                position: outer_expr.position,
            },
            ExprKind::LiteralString(_) => Type {
                kind: TypeKind::String,
                position: outer_expr.position,
            },
            ExprKind::LiteralNull => Type {
                kind: TypeKind::Null,
                position: outer_expr.position,
            },
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
                self.validate_variable_type(typ);
                typ.clone()
            }
            ExprKind::NewArray { element_type, size } => {
                self.validate_variable_type(element_type);
                let size_type = self.visit_expr(&size);
                if !matches!(size_type.kind, TypeKind::Int | TypeKind::Error) {
                    self.add_error(SemanticError::TypeMismatch {
                        expected: "int".to_string(),
                        found: size_type.to_string(),
                        position: size.position,
                    });
                }

                Type {
                    kind: TypeKind::Array(Box::new(element_type.clone())),
                    position: outer_expr.position,
                }
            }
            ExprKind::TypedNull(typ) => {
                self.validate_variable_type(&typ);

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

                typ.clone()
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
                            self.error_type(outer_expr.position)
                        } else {
                            Type {
                                kind: TypeKind::Class(ident.clone()),
                                position: ident.position,
                            }
                        }
                    }
                    ExprKind::LiteralString(_) => Type {
                        kind: TypeKind::String,
                        position: outer_expr.position,
                    },
                    _ => {
                        self.add_error(SemanticError::InvalidNullCastExpr {
                            expected,
                            cast_position: expr.position,
                            expr_position: outer_expr.position,
                        });
                        self.error_type(outer_expr.position)
                    }
                }
            }
            ExprKind::This => {
                if let Some(class_name) = &self.current_class {
                    let class_ident = Ident {
                        name: class_name.clone(),
                        position: outer_expr.position,
                    };

                    Type {
                        kind: TypeKind::Class(class_ident),
                        position: outer_expr.position,
                    }
                } else {
                    self.add_error(SemanticError::ThisUsedOutsideClass {
                        position: outer_expr.position,
                    });
                    self.error_type(outer_expr.position)
                }
            }
        }
    }

    fn visit_application(
        &mut self,
        callee: &Box<Expr>,
        args: &Vec<Box<Expr>>, // TODO: Remove box expr.
        position: SourcePosition,
    ) -> Type {
        // TODO: For now we only support direct calls on function names. Change that to use function types.
        // TODO: implicit this.
        if let ExprKind::Var(name) = &callee.kind {
            match self.symbol_table.lookup_function(&name) {
                Ok(func_info) => {
                    self.validate_function_call(&func_info, &args, position);
                    return Type {
                        kind: func_info.return_type.kind,
                        position,
                    };
                }
                Err(e) => {
                    self.add_error(e);
                    // Even if function is not found, we visit arguments to find errors in them.
                    for arg in args {
                        self.visit_expr(arg);
                    }
                    return self.error_type(position);
                }
            }
        } else {
            // Fallback for generic expr (not handled by this parser logic usually)
            return self.error_type(position);
        }
    }

    fn visit_array_access(
        &mut self,
        array: &Box<Expr>,
        index: &Box<Expr>,
        position: SourcePosition,
    ) -> Type {
        let array_type = self.visit_expr(&array);
        let index_type = self.visit_expr(&index);

        if !matches!(index_type.kind, TypeKind::Int | TypeKind::Error) {
            self.add_error(SemanticError::TypeMismatch {
                expected: "int".to_string(),
                found: index_type.to_string(),
                position: index.position,
            });
        }

        match &array_type.kind {
            TypeKind::Array(elem_type) => (**elem_type).clone(),
            TypeKind::Error => self.error_type(position),
            _ => {
                self.add_error(SemanticError::InvalidOperands {
                    operator: "[]".to_string(),
                    operand: array_type.to_string(),
                    position,
                    type_position: array_type.position,
                });
                self.error_type(position)
            }
        }
    }

    fn visit_field_access(
        &mut self,
        object: &Box<Expr>,
        field_name: &Ident,
        position: SourcePosition,
    ) -> Type {
        let obj_type = self.visit_expr(&object);

        if let TypeKind::Class(class_name) = obj_type.kind {
            let all_fields = self.symbol_table.get_all_class_fields(&class_name.name);

            if let Some((field_type, _)) = all_fields.get(&field_name.name) {
                field_type.clone()
            } else {
                self.add_error(SemanticError::UndefinedField {
                    class_name: class_name.name,
                    field_name: field_name.name.clone(),
                    position,
                    class_position: class_name.position,
                });
                self.error_type(position)
            }
        } else if let TypeKind::String | TypeKind::Array(_) = obj_type.kind {
            if field_name.name != "length" {
                self.add_error(SemanticError::UndefinedField {
                    class_name: obj_type.to_string(),
                    field_name: field_name.name.clone(),
                    position,
                    class_position: SourcePosition(0, 0),
                });
                self.error_type(position)
            } else {
                Type {
                    kind: TypeKind::Int,
                    position: position,
                }
            }
        } else if matches!(obj_type.kind, TypeKind::Error) {
            self.error_type(position)
        } else {
            self.add_error(SemanticError::InvalidOperands {
                operator: ".".to_string(),
                operand: obj_type.to_string(),
                position: position,
                type_position: obj_type.position,
            });
            self.error_type(position)
        }
    }

    fn visit_method_call(
        &mut self,
        callee: &Box<Expr>,  // A.b
        method_name: &Ident, // c
        args: &Vec<Box<Expr>>,
        position: SourcePosition,
    ) -> Type {
        let obj_type = self.visit_expr(callee);

        match &obj_type.kind {
            TypeKind::Class(class_name) => {
                let all_methods = self.symbol_table.get_all_class_methods(&class_name.name);

                if let Some(method_info) = all_methods.get(&method_name.name) {
                    self.validate_function_call(&method_info, args, position);

                    Type {
                        kind: method_info.return_type.kind.clone(),
                        position,
                    }
                } else {
                    self.add_error(SemanticError::UndefinedMethod {
                        class_name: class_name.name.clone(),
                        method_name: method_name.name.clone(),
                        position,
                        class_position: class_name.position,
                    });
                    // Visit args anyway to check them anyway
                    for arg in args {
                        self.visit_expr(arg);
                    }
                    self.error_type(position)
                }
            }
            TypeKind::Error => {
                // Visit args anyway
                for arg in args {
                    self.visit_expr(arg);
                }
                self.error_type(position)
            }
            _ => {
                self.add_error(SemanticError::InvalidOperands {
                    operator: format!("method call .{}()", method_name.name),
                    operand: obj_type.to_string(),
                    position,
                    type_position: obj_type.position,
                });
                for arg in args {
                    self.visit_expr(arg);
                }
                self.error_type(position)
            }
        }
    }

    fn visit_unary_operator(
        &mut self,
        op: &UnaryOp,
        expr: &Box<Expr>,
        position: SourcePosition,
    ) -> Type {
        let expr_type = self.visit_expr(&expr);

        if !self.type_legal_for_unary_op(op, &expr_type)
            && !matches!(expr_type.kind, TypeKind::Error)
        {
            self.add_error(SemanticError::InvalidOperands {
                operator: op.to_string(),
                operand: expr_type.to_string(),
                position: expr.position,
                type_position: expr_type.position,
            });
        }
        self.unary_op_result_type(op, &expr_type, position)
    }

    fn visit_binary_operator(
        &mut self,
        op: &BinaryOp,
        lhs: &Box<Expr>,
        rhs: &Box<Expr>,
        position: SourcePosition,
    ) -> Type {
        let lhs_type = self.visit_expr(&lhs);
        let rhs_type = self.visit_expr(&rhs);

        // Early return for error types to prevent cascading errors.
        if matches!(lhs_type.kind, TypeKind::Error) || matches!(rhs_type.kind, TypeKind::Error) {
            let type_checker = TypeChecker::new(&self.symbol_table);
            return type_checker.binary_op_result_type(op, &lhs_type, position);
        }

        self.validate_binary_operands(op, &lhs_type, &rhs_type, lhs.position, rhs.position);

        if matches!(op, BinaryOp::Div | BinaryOp::Mod) {
            if let ExprKind::LiteralInt(0) = rhs.kind {
                self.add_error(SemanticError::ZeroDivision {
                    position: rhs.position,
                });
            }
        }

        self.binary_op_result_type(op, &lhs_type, position)
    }
}
