use super::diagnostics::{Diagnostic, ToDiagnostic};
use crate::parser::ast::SourcePosition;
use crate::semantics::builtins;
use crate::utils::StrID;
use std::ops::Range;

/// Represents errors detected during semantic analysis (type checking, scoping, etc.).
#[derive(Debug, Clone)]
pub enum SemanticError {
    // ============ Undefined ============
    UndefinedVariable {
        name: StrID,
        position: SourcePosition,
    },
    UndefinedFunction {
        name: StrID,
        position: SourcePosition,
    },
    UndefinedClass {
        name: StrID,
        position: SourcePosition,
    },
    UndefinedField {
        class_name: StrID,
        field_name: StrID,
        position: SourcePosition,
        class_position: SourcePosition,
    },
    UndefinedMethod {
        class_name: StrID,
        method_name: StrID,
        position: SourcePosition,
        class_position: SourcePosition,
    },
    // ============ Duplicate ============
    DuplicateVariable {
        name: StrID,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    DuplicateFunction {
        name: StrID,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    DuplicateClass {
        name: StrID,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    DuplicateClassField {
        class_name: StrID,
        field_name: StrID,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    DuplicateMethod {
        class_name: StrID,
        method_name: StrID,
        position: SourcePosition,
        previous_position: SourcePosition,
    },
    // ============ Type Mismatch ============
    TypeMismatch {
        expected: StrID,
        found: StrID,
        position: SourcePosition,
    },
    InvalidCallBase {
        position: SourcePosition,
    },
    ForEachTypeMismatch {
        variable_type: StrID,
        collection_item_type: StrID,
        variable_position: SourcePosition,
        collection_position: SourcePosition,
    },
    ArgumentTypeMismatch {
        expected: StrID,
        found: StrID,
        position: SourcePosition,
        function_name: StrID,
        argument_position: SourcePosition,
    },
    AssignTypeMismatch {
        lhs_type: StrID,
        rhs_type: StrID,
        lhs_position: SourcePosition,
        rhs_position: SourcePosition,
    },
    BinaryOpTypeMismatch {
        lhs_type: StrID,
        lhs_position: SourcePosition,
        rhs_type: StrID,
        rhs_position: SourcePosition,
    },
    DeclarationTypeMismatch {
        var_name: StrID,
        expected: StrID,
        decl_type_position: SourcePosition,
        found: StrID,
        expr_position: SourcePosition,
    },

    // ============ Type Mismatch ============
    InvalidOperands {
        operator: StrID,
        operand: StrID,
        position: SourcePosition,
        type_position: SourcePosition,
    },
    WrongNumberOfArguments {
        name: StrID,
        expected: usize,
        found: usize,
        call_position: SourcePosition,
        function_position: SourcePosition,
    },
    InvalidReturnValue {
        expected: StrID,
        ret_type_position: SourcePosition,
        found: StrID,
        position: SourcePosition,
    },
    InvalidVoidReturn {
        ret_type_position: SourcePosition,
    },
    InvalidLValue {
        position: SourcePosition,
    },
    ZeroDivision {
        position: SourcePosition,
    },
    UndefinedMain,
    WrongMainReturnType {
        expected: StrID,
        found: StrID,
        position: SourcePosition,
    },
    WrongNumberOfMainArguments {
        found: usize,
        position: SourcePosition,
    },
    // ============ Invalid type ============
    InvalidVariableType {
        found: StrID,
        expected: Vec<StrID>,
        position: SourcePosition,
    },
    InvalidFunctionReturnType {
        found: StrID,
        expected: Vec<StrID>,
        position: SourcePosition,
    },
    InvalidVoidArrayType {
        expected: Vec<StrID>,
        position: SourcePosition,
    },
    InvalidNullCast {
        found: StrID,
        expected: Vec<StrID>,
        type_position: SourcePosition,
        expr_position: SourcePosition,
    },
    InvalidNullCastExpr {
        expected: Vec<StrID>,
        cast_position: SourcePosition,
        expr_position: SourcePosition,
    },
    MissingReturn {
        function_name: StrID,
        position: SourcePosition,
    },

    // ============ OOP ============
    ThisUsedOutsideClass {
        position: SourcePosition,
    },
    InheritanceCycle {
        // (subclass, superclass, extends_position)
        cycle: Vec<(StrID, StrID, SourcePosition)>,
    },
    InvalidOverride {
        method_name: StrID,
        parent_name: StrID,
        reason: StrID,
        position: SourcePosition,
    },
}

// Helper to convert SourcePosition to Range<usize>
fn range(pos: &SourcePosition) -> Range<usize> {
    pos.0..pos.1
}

impl SemanticError {
    fn simple(
        msg: impl Into<String>,
        pos: &SourcePosition,
        label: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic::error(msg).label(range(pos), label)
    }

    fn duplicate(
        what: &str,
        name: &str,
        pos: &SourcePosition,
        prev_pos: &SourcePosition,
    ) -> Diagnostic {
        Diagnostic::error(format!("{} '{}' is already defined", what, name))
            .label(range(pos), format!("Cannot redefine this {}", what))
            .secondary_label(
                range(prev_pos),
                format!("'{}' was originally defined here", name),
            )
            .help("Consider renaming or using a different scope.")
    }

    fn mismatch(pos: &SourcePosition, expected: &str, found: &str) -> Diagnostic {
        Diagnostic::error("Type mismatch")
            .label(
                range(pos),
                format!("Expected type `{}`, found `{}`", expected, found),
            )
            .note(format!(
                "Expected an expression of type `{}` here.",
                expected
            ))
    }

    fn undefined_member(
        kind: &str,
        member: &str,
        cls: &str,
        pos: &SourcePosition,
        cls_pos: &SourcePosition,
    ) -> Diagnostic {
        Diagnostic::error(format!("Undefined {} '{}'", kind, member))
            .label(
                range(pos),
                format!("{} '{}' was not declared", kind, member),
            )
            .secondary_label(range(cls_pos), "In this class")
            .note(format!(
                "{} '{}' was not declared in class '{}'",
                kind, member, cls
            ))
            .help(format!("Consider declaring this {}.", kind))
    }
}

impl ToDiagnostic for SemanticError {
    fn to_diagnostic(&self) -> Diagnostic {
        match self {
            SemanticError::UndefinedVariable { name, position } => {
                            Self::simple(format!("Undefined variable '{}'", name), position, "Variable not found")
                    .note("This variable was not declared in the current scope")
            }
            SemanticError::UndefinedFunction { name, position } => {
                Self::simple(format!("Undefined function '{}'", name), position, "Function not found")
                    .note("This function was not declared in the current scope")
            }
            SemanticError::UndefinedClass { name, position } => {
                Self::simple(format!("Undefined class '{}'", name), position, "Class not found")
                    .note("This class was not declared in the current scope")
            }
            SemanticError::UndefinedField { class_name, field_name, position, class_position } => {
                Self::undefined_member("field", field_name, class_name, position, class_position)
            }
            SemanticError::UndefinedMethod { class_name, method_name, position, class_position } => {
                Self::undefined_member("method", method_name, class_name, position, class_position)
            }
            SemanticError::DuplicateVariable { name, position, previous_position } => {
                Self::duplicate("Variable", name, position, previous_position)
            }
            SemanticError::DuplicateFunction { name, position, previous_position } => {
                Self::duplicate("Function", name, position, previous_position)
            }
            SemanticError::DuplicateClass { name, position, previous_position } => {
                Self::duplicate("Class", name, position, previous_position)
            }
            SemanticError::DuplicateClassField { class_name, field_name, position, previous_position } => {
                Self::duplicate(&format!("Field in class '{}': ", class_name), field_name, position, previous_position)
            }
            SemanticError::DuplicateMethod { class_name, method_name, position, previous_position } => {
                Self::duplicate(&format!("Method for class '{}'", class_name), method_name, position, previous_position)
            }
            SemanticError::TypeMismatch { expected, found, position } => {
                Self::mismatch(position, expected, found)
            }
            SemanticError::InvalidCallBase { position } => {
                Diagnostic::error("This expression cannot be a call base").label(
                    range(position),
                    format!("Cannot call this expression"),
                )
                    .help("Perhaps you mistyped the '()'?")
            }
            SemanticError::ForEachTypeMismatch { variable_type, collection_item_type, variable_position, collection_position } => {
                Diagnostic::error("Type mismatch in for-each loop")
                    .label(range(collection_position), format!("Collection contains elements of type `{}`", collection_item_type))
                    .label(range(variable_position), format!("Loop variable declared as `{}`", variable_type))
                    .note(format!("Type `{}` cannot be assigned to variable of type `{}`", collection_item_type, variable_type))
            }
            SemanticError::ArgumentTypeMismatch { expected, found, position, function_name, argument_position } => {
                let mut diag = Diagnostic::error("Argument type mismatch")
                    .label(range(position), format!("Expected type `{}`, but found `{}`", expected, found));

                if let Some(signature) = builtins::get_builtin_signature(*function_name) {
                    diag = diag.note(format!("`{}` is a built-in function with signature: '{}'", function_name, signature));
                } else {
                    diag = diag
                        .secondary_label(range(argument_position), format!("Function `{}` declared here expecting `{}`", function_name, expected))
                        .note(format!("Expected an expression of type `{}` here.", expected));
                }
                diag
            }
            SemanticError::AssignTypeMismatch { lhs_type, rhs_type, lhs_position, rhs_position } => {
                Diagnostic::error("Type mismatch in assignment")
                    .label(range(lhs_position), format!("Left side is of type `{}`", lhs_type))
                    .label(range(rhs_position), format!("Right side is of type `{}`", rhs_type))
                    .note("Expected right side of the assignment to be a subtype of the left side")
            }
            SemanticError::BinaryOpTypeMismatch { lhs_type, lhs_position, rhs_type, rhs_position } => {
                Diagnostic::error("Type mismatch in expression")
                    .label(range(lhs_position), format!("Left side is of type `{}`", lhs_type))
                    .label(range(rhs_position), format!("Right side is of type `{}`", rhs_type))
                    .note("Expected both sides of the expression to have the same type")
            }
            SemanticError::DeclarationTypeMismatch { var_name, expected, decl_type_position, found, expr_position } => {
                Diagnostic::error("Type mismatch in declaration")
                    .label(range(expr_position), format!("Initializing expression is of type '{}'", found))
                    .label(range(decl_type_position), format!("Variable '{}' declared with type '{}'", var_name, expected))
                    .help("Consider changing the variable type or the initializing expression.")
            }
            SemanticError::InvalidOperands { operator, operand, position, type_position } => {
                Diagnostic::error(format!("Invalid operands for operator '{}'", operator))
                    .label(range(position), format!("Cannot apply '{}' to a value of type '{}'", operator, operand))
                    .secondary_label(range(type_position), format!("Operand is of type '{}'", operand))
            }
            SemanticError::ZeroDivision { position } => {
                Self::simple("Division by zero", position, "Division by zero literal detected")
                    .note("Division by zero is undefined and will cause a runtime error")
                    .help("Use a non-zero value")
            }
            SemanticError::InvalidLValue { position } => {
                Self::simple("Invalid assignment target", position, "This expression cannot be assigned to")
                    .help("Only variables, array elements, and object fields (other than the builtin \"length\" field) can be targets of an assignment.")
            }
            SemanticError::WrongNumberOfArguments { name, expected, found, call_position, function_position } => {
                Diagnostic::error(format!("Wrong number of arguments for function '{}'", name))
                    .label(range(call_position), format!("Called with {} argument(s)", found))
                    .secondary_label(range(function_position), format!("Defined to expect {} argument(s)", expected))
                    .help("Ensure the number of arguments matches the function definition.")
            }
            SemanticError::InvalidReturnValue { expected, found, position, ret_type_position } => {
                Diagnostic::error("Invalid return value")
                    .label(range(position), format!("Returning value of type `{}`", found))
                    .secondary_label(range(ret_type_position), format!("Function declared to return `{}`", expected))
                    .help("Change the return type or the returned expression.")
            }
            SemanticError::InvalidVoidReturn { ret_type_position } => {
                Self::simple("Invalid return", ret_type_position, "Cannot return an expression of type 'void'")
                    .help("Remove the return value or change the expression.")
            }
            SemanticError::MissingReturn { function_name, position } => {
                Self::simple(format!("Missing return in function '{}'", function_name), position, "Not all paths return a value")
                    .help("Ensure all execution paths end with a return statement.")
            }
            SemanticError::InvalidFunctionReturnType { found, expected, position } => {
                Diagnostic::error("Invalid function return type")
                    .label(range(position), format!("`{}` is not a valid return type", found))
                    .help(format!("Try one of: {}", expected.iter().map(|str| str.to_string()).collect::<Vec<String>>().join(", ")))
            }
            SemanticError::UndefinedMain => {
                Diagnostic::error("Missing `main` function")
                    .help("This program does not provide a `main` function. Please define one.")
            }
            SemanticError::WrongMainReturnType { expected, found, position } => {
                Diagnostic::error("Invalid `main` return type")
                    .label(range(position), format!("Found `{}`, expected `{}`", found, expected))
                    .help(format!("Change the return type to `{}`", expected))
            }
            SemanticError::WrongNumberOfMainArguments { found, position } => {
                Diagnostic::error("Invalid `main` arguments")
                    .label(range(position), format!("Found {} arguments, expected 0", found))
                    .help("Change `main` to take zero arguments.")
            }
            SemanticError::InvalidVariableType { found, expected, position } => {
                Diagnostic::error("Invalid variable type")
                    .label(range(position), format!("`{}` is not valid here", found))
                    .help(format!("Valid types are: {}", expected.iter().map(|str| str.to_string()).collect::<Vec<String>>().join(", ")))
            }
            SemanticError::InvalidVoidArrayType { expected, position } => {
                Diagnostic::error("Invalid array type")
                    .label(range(position), "`void` cannot be an array base type")
                    .help(format!("Try one of: {}", expected.iter().map(|str| str.to_string()).collect::<Vec<String>>().join(", ")))
            }
            SemanticError::InvalidNullCast { found, expected, type_position, expr_position } => {
                Diagnostic::error("Invalid 'null' cast")
                    .label(range(expr_position), "In this cast expression")
                    .secondary_label(range(type_position), format!("Cannot cast null to `{}`", found))
                    .help(format!("Valid cast targets: {}", expected.iter().map(|str| str.to_string()).collect::<Vec<String>>().join(", ")))
            }
            SemanticError::InvalidNullCastExpr { expected, cast_position, expr_position } => {
                Diagnostic::error("Invalid 'null' cast expression")
                    .label(range(expr_position), "In this expression")
                    .secondary_label(range(cast_position), "Not a valid null cast")
                    .help(format!("Try one of: {}", expected.iter().map(|str| str.to_string()).collect::<Vec<String>>().join(", ")))
            }
            SemanticError::ThisUsedOutsideClass { position } => {
                Self::simple("Invalid use of 'this'", position, "'this' used outside of a class method")
                    .help("Remove 'this' or move the code into a class method.")
            }
            SemanticError::InvalidOverride { method_name, parent_name, reason, position } => {
                Diagnostic::error("Invalid method override")
                    .label(range(position), format!("Overrides `{}` from `{}`", method_name, parent_name))
                    .note(format!("Reason: {}", reason))
                    .help("Signature must match the parent class exactly.")
            }
            SemanticError::InheritanceCycle { cycle } => {
                let mut diag = Diagnostic::error("Inheritance cycle detected")
                    .note("Inheritance hierarchy cannot contain cycles.");

                for (i, (child, parent, pos)) in cycle.iter().enumerate() {
                    let msg = if i == 0 {
                        format!("Cycle starts: `{}` extends `{}`", child, parent)
                    } else if i == cycle.len() - 1 {
                        format!("...and `{}` extends `{}`, closing the cycle", child, parent)
                    } else {
                        format!("...class `{}` extends `{}`...", child, parent)
                    };
                    diag = diag.label(range(pos), msg);
                }
                diag
            }
                    }
    }
}
