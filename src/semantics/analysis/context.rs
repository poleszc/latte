use crate::errors::SemanticError;
use crate::parser::ast::*;
use crate::semantics::analysis::type_checker::TypeChecker;
use crate::semantics::symbol_table::SymbolTable;
use crate::semantics::{ClassInfo, FunctionInfo, VariableInfo};
use crate::utils::StrID;

#[derive(Debug, Default)]
pub struct SemanticContext {
    pub symbol_table: SymbolTable,
    /// TODO: This should be a stack once we support nested functions.
    /// Return type of the function we're currently analyzing.
    pub current_return_type: Option<Type>,
    /// Class we're currently analyzing.
    pub current_class: Option<StrID>,
    pub errors: Vec<SemanticError>,
}

// ======================
// Basic functionality.
// ======================
impl SemanticContext {
    pub fn new() -> Self {
        SemanticContext {
            symbol_table: SymbolTable::new(),
            current_return_type: None,
            current_class: None,
            errors: Vec::new(),
        }
    }

    /// Adds an error to the context.
    pub fn add_error(&mut self, error: SemanticError) {
        self.errors.push(error);
    }

    /// Defines a variable in the current scope. Logs an error on duplicate definition.
    pub fn define_variable(&mut self, ident: Ident, typ: Type) {
        if let Err(e) = self
            .symbol_table
            .define_variable(VariableInfo { ident, typ })
        {
            self.add_error(e);
        }
    }

    /// Defines a function in the current scope. Logs an error on duplicate definition.
    pub fn define_function(&mut self, func_info: FunctionInfo) {
        if let Err(e) = self.symbol_table.define_function(func_info) {
            self.add_error(e);
        }
    }

    /// Defines a class in the current scope. Logs an error on duplicate definition.
    pub fn define_class(&mut self, class_info: ClassInfo) {
        if let Err(e) = self.symbol_table.define_class(class_info) {
            self.add_error(e);
        }
    }
}

// ======================
// Type utilities
// ======================
impl SemanticContext {
    pub fn types_equal(&self, t1: &Type, t2: &Type) -> bool {
        TypeChecker::types_equal(t1, t2)
    }

    pub fn is_subtype(&self, subtype: &Type, supertype: &Type) -> bool {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.is_subtype(subtype, supertype)
    }

    pub fn validate_variable_type(&mut self, typ: &Type) {
        let type_checker = TypeChecker::new(&self.symbol_table);
        if let Err(e) = type_checker.validate_variable_type(typ) {
            self.add_error(e);
        }
    }

    pub fn validate_function_return_type(&mut self, typ: &Type) {
        let type_checker = TypeChecker::new(&self.symbol_table);
        if let Err(e) = type_checker.validate_function_return_type(typ) {
            self.add_error(e);
        }
    }

    pub fn type_legal_for_bin_op(&self, op: &BinaryOp, typ: &Type) -> bool {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.type_legal_for_bin_op(op, typ)
    }

    pub fn type_legal_for_unary_op(&self, op: &UnaryOp, typ: &Type) -> bool {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.type_legal_for_unary_op(op, typ)
    }

    pub fn binary_op_result_type(
        &self,
        op: &BinaryOp,
        typ: &Type,
        position: SourcePosition,
    ) -> Type {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.binary_op_result_type(op, typ, position)
    }

    pub fn unary_op_result_type(&self, op: &UnaryOp, position: SourcePosition) -> Type {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.unary_op_result_type(op, position)
    }

    pub fn error_type(&self, pos: SourcePosition) -> Type {
        let type_checker = TypeChecker::new(&self.symbol_table);
        type_checker.error_type(pos)
    }
}

// ======================
// Misc / Shared Validation Logic
// ======================
impl SemanticContext {
    // TODO: Is this used?
    pub fn resolve_method(&self, class_name: StrID, method_name: StrID) -> Option<FunctionInfo> {
        self.symbol_table.lookup_method(class_name, method_name)
    }

    // TODO: Is this used?
    pub fn resolve_field(
        &self,
        class_name: StrID,
        field_name: StrID,
    ) -> Option<(Type, SourcePosition)> {
        let all_fields = self.symbol_table.get_all_class_fields(class_name);
        all_fields.get(&field_name).cloned()
    }
}
