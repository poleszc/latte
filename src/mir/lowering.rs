use core::panic;
use std::collections::HashMap;

use crate::{
    SourcePosition,
    mir::{
        LocalID, MIRClass, MIRField, MIRFunction, MIROperation, MIRPlace, MIRProgram, MIRValue,
        function_builder::MIRFunctionBuilder, program_builder::MIRProgramBuilder,
    },
    parser::ast::{BinaryOp, Ident, Type, TypeKind, UnaryOp},
    semantics::{
        symbol_table::SymbolTable,
        tast::{TBlock, TItem, TProgram},
        tast::{
            TClassDef, TClassEntryKind, TExpr, TExprKind, TFuncDef, TItemKind, TStmt, TStmtKind,
            TTopDefKind,
        },
    },
    utils::StrID,
};

pub struct MIRLoweringContext {
    symbol_table: SymbolTable,
    // TODO: Try to remove that and make more use of the symbol table.
    variable_map: HashMap<StrID, LocalID>,
    current_function: Option<MIRFunctionBuilder>,
    current_class_fields: HashMap<StrID, (usize, Type)>,
}

/// Helpers
impl MIRLoweringContext {
    /// Get a mutable reference to the current function builder.
    /// Panic if not set, since this should never happen.
    fn builder(&mut self) -> &mut MIRFunctionBuilder {
        self.current_function
            .as_mut()
            .expect("Function builder not set")
    }

    /// Get the reference to the current function builder.
    /// Panic if not set, since this should never happen.
    fn builder_ref(&mut self) -> &MIRFunctionBuilder {
        self.current_function
            .as_ref()
            .expect("Function builder not set")
    }

    /// Variable name to LocalId.
    fn resolve_variable(&self, name: StrID) -> Option<LocalID> {
        self.variable_map.get(&name).copied()
    }

    /// Field name to (offset, type)
    fn resolve_field(&self, name: StrID) -> Option<(usize, Type)> {
        self.current_class_fields.get(&name).cloned()
    }

    /// Get 'this' local variable.
    /// Panic if not found, since those errors should be checked by the type checker.
    fn get_this_local(&self) -> LocalID {
        self.variable_map
            .get(&StrID::from("this"))
            .cloned()
            .expect("'this' not found")
    }
}

// ======================
// Basic functionality.
// ======================
impl MIRLoweringContext {
    pub fn new(symbol_table: SymbolTable) -> Self {
        MIRLoweringContext {
            symbol_table,
            variable_map: HashMap::new(),
            current_function: None,
            current_class_fields: HashMap::new(),
        }
    }

    /// Main function performing the lowering for the whole program.
    pub fn lower_program(&mut self, program: &TProgram) -> MIRProgram {
        let mut builder = MIRProgramBuilder::new();

        // Lower all classes to MIR.
        for top_def in &program.defs {
            if let TTopDefKind::Class(class_def) = &top_def.kind {
                let mir_class = self.lower_class(class_def);
                builder.add_class(mir_class);
            }
        }

        // Lower all functions to MIR.
        for top_def in &program.defs {
            if let TTopDefKind::Func(func_def) = &top_def.kind {
                let mir_function = self.lower_function(func_def);
                builder.add_function(mir_function);
            }
        }

        // Lower all methods to MIR.
        for top_def in &program.defs {
            if let TTopDefKind::Class(class_def) = &top_def.kind {
                // Insert all class fields in the scope to have access to them in methods.
                let class_fields = self
                    .symbol_table
                    .get_class_field_layout(class_def.name.name);
                for (index, (name, typ, _)) in class_fields.iter().enumerate() {
                    self.current_class_fields
                        .insert(*name, (index, typ.clone()));
                }

                for entry in &class_def.entries {
                    if let TClassEntryKind::Method(method) = &entry.kind {
                        let mir_function = self.lower_method(class_def, method);
                        builder.add_function(mir_function);
                    }
                }
                self.current_class_fields.clear();
            }
        }
        builder.build()
    }
}

// ======================
// Top Def lowering.
// ======================
impl MIRLoweringContext {
    fn lower_class(&mut self, class_def: &TClassDef) -> MIRClass {
        let class_name = class_def.name.name;
        let field_layout = self.symbol_table.get_class_field_layout(class_name);

        let fields: Vec<MIRField> = field_layout
            .into_iter()
            .enumerate()
            .map(|(index, (name, typ, _pos))| MIRField {
                name,
                typ,
                offset: index, // TODO: Change that
                index,
            })
            .collect();

        let vtable_layout = self.symbol_table.get_class_vtable_layout(class_name);
        let vtable: Vec<StrID> = vtable_layout
            .iter()
            .map(|func_info| {
                let defining_class = self
                    .symbol_table
                    .find_method_owner(class_name, func_info.ident.name)
                    .unwrap();
                self.mangle_method_name(defining_class, func_info.ident.name)
            })
            .collect();

        let all_methods_map = self.symbol_table.get_all_class_methods(class_name);
        let mut methods: Vec<StrID> = all_methods_map.keys().cloned().collect();
        methods.sort();

        MIRClass {
            name: class_name,
            parent: class_def.extends.map(|e| e.name),
            fields,
            methods,
            vtable,
            source_position: class_def.position,
        }
    }

    fn lower_method(&mut self, class_def: &TClassDef, method: &TFuncDef) -> MIRFunction {
        let mangled_name = self.mangle_method_name(class_def.name.name, method.name.name);

        // Insert this as first parameter.
        let mut parameter_types = vec![Type {
            kind: TypeKind::Class(class_def.name),
            position: class_def.position,
        }];
        parameter_types.extend(method.args.iter().map(|arg| arg.arg_type.clone()));

        let mut builder = MIRFunctionBuilder::new(
            mangled_name,
            method.return_type.clone(),
            parameter_types,
            method.position,
        );

        self.variable_map.clear();

        let this_local_id = builder.create_local(
            Some(StrID::from("this")),
            Type {
                kind: TypeKind::Class(class_def.name),
                position: class_def.position,
            },
            true,
            Some(0),
        );
        self.variable_map.insert(StrID::from("this"), this_local_id);

        for (i, arg) in method.args.iter().enumerate() {
            let local_id =
                builder.create_local(Some(arg.name.name), arg.arg_type.clone(), true, Some(i + 1));
            self.variable_map.insert(arg.name.name, local_id);
        }

        self.current_function = Some(builder);

        self.lower_block(&method.body);

        // If a void function
        if matches!(method.return_type.kind, TypeKind::Void) {
            // TODO: Make a utility which unwraps the builder.
            let builder = self.builder();
            if !builder.is_current_block_terminated() {
                builder.emit_return(None, method.position);
            }
        }

        self.current_function.take().unwrap().build()
    }

    fn lower_function(&mut self, func_def: &TFuncDef) -> MIRFunction {
        // TODO: This is duplicated with method validation. Fix that.
        let mut builder = MIRFunctionBuilder::new(
            func_def.name.name,
            func_def.return_type.clone(),
            func_def
                .args
                .iter()
                .map(|arg| arg.arg_type.clone())
                .collect(),
            func_def.position,
        );
        // Clear variable map.
        self.variable_map.clear();

        // Add arguments as locals.
        for (i, arg) in func_def.args.iter().enumerate() {
            let local_id =
                builder.create_local(Some(arg.name.name), arg.arg_type.clone(), true, Some(i));
            self.variable_map.insert(arg.name.name, local_id);
        }

        self.current_function = Some(builder);

        self.lower_block(&func_def.body);

        // If a void function
        if matches!(func_def.return_type.kind, TypeKind::Void) {
            let builder = self.builder();
            if !builder.is_current_block_terminated() {
                builder.emit_return(None, func_def.position);
            }
        }

        self.current_function.take().unwrap().build()
    }
}
// ======================
// Utils.
// ======================
impl MIRLoweringContext {}

// ======================
// Statement lowering.
// ======================
impl MIRLoweringContext {
    fn lower_block(&mut self, block: &TBlock) {
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
        }
    }

    fn lower_stmt(&mut self, stmt: &TStmt) {
        match &stmt.kind {
            TStmtKind::Empty => {}
            TStmtKind::Block(block) => self.lower_block(block),
            TStmtKind::Decl(ty, items) => self.lower_declaration(ty, items, stmt.position),
            TStmtKind::Assign { target, value } => self.lower_assign(target, value, stmt.position),
            TStmtKind::Return { expr } => self.lower_return(expr, stmt.position),
            TStmtKind::Incr(expr) => self.lower_increment(expr, stmt.position),
            TStmtKind::Decr(expr) => self.lower_decrement(expr, stmt.position),
            TStmtKind::If {
                cond,
                if_true,
                if_false,
            } => self.lower_if(cond, if_true, if_false, stmt.position),
            TStmtKind::While { cond, body } => self.lower_while(cond, body, stmt.position),
            // TODO: Add a classic for loop
            TStmtKind::ForEach {
                element_type,
                item_name,
                collection,
                body,
            } => self.lower_for_each(element_type, item_name, collection, body, stmt.position),
            TStmtKind::Expr(expr) => {
                self.lower_expr(expr);
            } // TODO: Add nested functions
        }
    }

    fn lower_declaration(&mut self, typ: &Type, items: &Vec<TItem>, stmt_position: SourcePosition) {
        for item in items {
            match &item.kind {
                TItemKind::NoInit(ident) => {
                    let local_id =
                        self.builder()
                            .create_local(Some(ident.name), typ.clone(), false, None);
                    self.variable_map.insert(ident.name, local_id);
                }
                TItemKind::Init(ident, expr) => {
                    let local_id =
                        self.builder()
                            .create_local(Some(ident.name), typ.clone(), false, None);
                    self.variable_map.insert(ident.name, local_id);

                    // Calculate the expression and assign to the var.
                    let value = self.lower_expr(expr);
                    self.builder()
                        .emit_assign(MIRPlace::Local(local_id), value, stmt_position);
                }
            }
        }
    }

    fn lower_assign(&mut self, target: &TExpr, value: &TExpr, stmt_position: SourcePosition) {
        let place = self.lower_expr_as_place(target);
        let val = self.lower_expr(value);
        self.builder().emit_assign(place, val, stmt_position);
    }

    fn lower_return(&mut self, expr: &Option<TExpr>, stmt_position: SourcePosition) {
        let value = expr.as_ref().map(|e| self.lower_expr(e));
        self.builder().emit_return(value, stmt_position);
    }

    fn lower_increment(&mut self, expr: &TExpr, stmt_position: SourcePosition) {
        // i++ => {
        //    temp = i + 1;
        //    i = temp;
        // }

        let place = self.lower_expr_as_place(expr);
        let val = self.lower_place_as_value(&place, &expr.typ);

        // temp = i + 1;
        let temp = self.builder().create_temp(Type {
            kind: TypeKind::Int,
            position: expr.position,
        });
        self.builder().emit_binary(
            MIROperation::IntAdd,
            MIRPlace::Temp(temp),
            val,
            MIRValue::IntConst(1),
            stmt_position,
        );

        // i = temp;
        self.builder()
            .emit_assign(place, MIRValue::Temp(temp), stmt_position);
    }

    fn lower_decrement(&mut self, expr: &TExpr, stmt_position: SourcePosition) {
        // i-- => {
        //    temp = i - 1;
        //    i = temp;
        // }

        let place = self.lower_expr_as_place(expr);
        let val = self.lower_place_as_value(&place, &expr.typ);

        // temp = i - 1;
        let temp = self.builder().create_temp(Type {
            kind: TypeKind::Int,
            position: expr.position,
        });
        self.builder().emit_binary(
            MIROperation::IntSub,
            MIRPlace::Temp(temp),
            val,
            MIRValue::IntConst(1),
            stmt_position,
        );

        // i = temp;
        self.builder()
            .emit_assign(place, MIRValue::Temp(temp), stmt_position);
    }

    fn lower_if(
        &mut self,
        cond: &TExpr,
        if_true: &TStmt,
        if_false: &Option<Box<TStmt>>,
        stmt_position: SourcePosition,
    ) {
        let cond_val = self.lower_expr(cond);

        if let Some(if_false) = if_false {
            // Else exists.
            let true_block = self.builder().create_block();
            let false_block = self.builder().create_block();
            let merge_block = self.builder().create_block();

            self.builder()
                .emit_branch(cond_val, true_block, false_block, stmt_position);

            // Lower the true block.
            self.builder().set_current_block(true_block);
            self.lower_stmt(if_true);

            // Set the jump, only if the block is not terminated already.
            if !self.builder().is_current_block_terminated() {
                self.builder().emit_jump(merge_block, stmt_position);
            }

            // Lower the false block.
            self.builder().set_current_block(false_block);
            self.lower_stmt(if_false);

            // Set the jump, only if the block is not terminated already.
            if !self.builder().is_current_block_terminated() {
                self.builder().emit_jump(merge_block, stmt_position);
            }

            // Continue to the merge block
            self.builder().set_current_block(merge_block);
        } else {
            // Else doesn't exists.
            let true_block = self.builder().create_block();
            let merge_block = self.builder().create_block();

            self.builder()
                .emit_branch(cond_val, true_block, merge_block, stmt_position);

            // Lower the true block.
            self.builder().set_current_block(true_block);
            self.lower_stmt(if_true);

            // Set the jump, only if the block is not terminated already.
            if !self.builder().is_current_block_terminated() {
                self.builder().emit_jump(merge_block, stmt_position);
            }

            // Continue to the merge block
            self.builder().set_current_block(merge_block);
        }
    }

    fn lower_while(&mut self, cond: &TExpr, body: &TStmt, stmt_position: SourcePosition) {
        let loop_start = self.builder().create_block();
        let loop_body = self.builder().create_block();
        let loop_exit = self.builder().create_block();

        // Jump to the condition check.
        self.builder().emit_jump(loop_start, stmt_position);

        // Loop condition check.
        self.builder().set_current_block(loop_start);
        let cond_val = self.lower_expr(cond);
        self.builder()
            .emit_branch(cond_val, loop_body, loop_exit, stmt_position);

        // Loop body.
        self.builder().set_current_block(loop_body);
        self.lower_stmt(body);

        if !self.builder().is_current_block_terminated() {
            self.builder().emit_jump(loop_start, stmt_position);
        }

        // Leave the loop.
        self.builder().set_current_block(loop_exit);
    }

    fn lower_for_each(
        &mut self,
        element_type: &Type,
        item_name: &Ident,
        collection: &TExpr,
        body: &TStmt,
        stmt_position: SourcePosition,
    ) {
        // for(T a: collection) => {
        //      i = 0;
        //      T item;
        //      while(i < collection.length) {
        //          item = arr[i];
        //          // ...
        //          i = i + 1;
        // }

        let collection_val = self.lower_expr(collection);

        // i = 0;
        let i_temp = self.builder().create_temp(Type {
            kind: TypeKind::Int,
            position: stmt_position,
        });
        self.builder()
            .emit_assign(MIRPlace::Temp(i_temp), MIRValue::IntConst(0), stmt_position);

        // T item;
        let item_local =
            self.builder()
                .create_local(Some(item_name.name), element_type.clone(), false, None);
        self.variable_map.insert(item_name.name, item_local);

        let loop_start = self.builder().create_block();
        let loop_body = self.builder().create_block();
        let loop_exit = self.builder().create_block();

        // Jump to the condition check.
        if !self.builder().is_current_block_terminated() {
            self.builder().emit_jump(loop_start, stmt_position);
        }

        // Loop condition check.
        self.builder().set_current_block(loop_start);

        // temp = arr.length;
        let length_temp = self.builder().create_temp(Type {
            kind: TypeKind::Int,
            position: stmt_position,
        });
        self.builder().emit_array_length(
            MIRPlace::Temp(length_temp),
            collection_val.clone(),
            stmt_position,
        );

        // cond = i < length;
        let cond_temp = self.builder().create_temp(Type {
            kind: TypeKind::Bool,
            position: stmt_position,
        });
        self.builder().emit_less_then(
            MIRPlace::Temp(cond_temp),
            MIRValue::Temp(i_temp),
            MIRValue::Temp(length_temp),
            stmt_position,
        );
        self.builder().emit_branch(
            MIRValue::Temp(cond_temp),
            loop_body,
            loop_exit,
            stmt_position,
        );

        // Loop body.
        self.builder().set_current_block(loop_body);

        self.builder().emit_array_get(
            MIRPlace::Local(item_local),
            collection_val.clone(),
            MIRValue::Temp(i_temp),
            stmt_position,
        );

        // Loop body.
        self.lower_stmt(body);

        // i = i + 1;
        if !self.builder().is_current_block_terminated() {
            let temp = self.builder().create_temp(Type {
                kind: TypeKind::Int,
                position: stmt_position,
            });
            self.builder().emit_binary(
                MIROperation::IntAdd,
                MIRPlace::Temp(temp),
                MIRValue::Temp(i_temp),
                MIRValue::IntConst(1),
                stmt_position,
            );
            self.builder()
                .emit_assign(MIRPlace::Temp(i_temp), MIRValue::Temp(temp), stmt_position);

            // Jump to loop start.
            self.builder().emit_jump(loop_start, stmt_position);
        }

        // Leave the loop.
        self.builder().set_current_block(loop_exit);
    }
}

// ======================
// LValue lowering.
// ======================
impl MIRLoweringContext {
    /// Lower a "variable" and returns it's place.
    /// Also handles class members accessed directly, not through "this".
    fn lower_variable_as_place(&mut self, ident: &Ident) -> MIRPlace {
        // First look for the identifier in the local variables.
        if let Some(local_id) = self.resolve_variable(ident.name) {
            return MIRPlace::Local(local_id);
        }

        // If not found look in the current class fields.
        if let Some((field_index, _)) = self.resolve_field(ident.name) {
            // Retrieve 'this' from locals. It should always exist when lowering methods.
            let this_local_id = self.get_this_local();
            let this_place = MIRPlace::Local(this_local_id);

            // Now find the field place.
            let this_info = self.builder().get_local_info(this_local_id);
            let class_name = if let TypeKind::Class(class_ident) = &this_info.typ.kind {
                class_ident.name
            } else {
                panic!("'this' is not a class type")
            };

            return MIRPlace::Field {
                base: Box::new(this_place),
                class_name,
                field_name: ident.name,
                field_index: field_index,
            };
        }
        panic!(
            "Variable {} not found in locals and class fields",
            ident.name
        );
    }

    /// Lower an array access expression and return it's place.
    fn lower_array_access_as_place(&mut self, array: &TExpr, index: &TExpr) -> MIRPlace {
        let array_val = self.lower_expr(array);
        let array_place = match array_val {
            MIRValue::Local(local_id) => MIRPlace::Local(local_id),
            MIRValue::Temp(temp_id) => MIRPlace::Temp(temp_id),
            _ => panic!("Array base must be a Local or Temp variable"),
        };
        let index_val = self.lower_expr(index);

        MIRPlace::Index {
            base: Box::new(array_place),
            index: Box::new(index_val),
        }
    }

    /// Lower an field access expression and return it's place.
    fn lower_field_access_as_place(&mut self, object: &TExpr, field_name: Ident) -> MIRPlace {
        let object_val = self.lower_expr(object);
        let object_place = match object_val {
            MIRValue::Local(local_id) => MIRPlace::Local(local_id),
            MIRValue::Temp(temp_id) => MIRPlace::Temp(temp_id),
            _ => panic!("Object base must be a Local or Temp variable"),
        };

        let class_name = if let TypeKind::Class(ident) = &object.typ.kind {
            ident.name
        } else {
            panic!("Field access on non-class type");
        };
        let field_layout = self.symbol_table.get_class_field_layout(class_name);
        // TODO: Map for faster lookup?
        let (field_index, _) = field_layout
            .iter()
            .enumerate()
            .find(|(_, (name, _, _))| *name == field_name.name)
            .unwrap();

        MIRPlace::Field {
            base: Box::new(object_place),
            class_name,
            field_name: field_name.name,
            field_index,
        }
    }

    /// Lower expression and return it's place.
    fn lower_expr_as_place(&mut self, expr: &TExpr) -> MIRPlace {
        match &expr.kind {
            TExprKind::Var(ident) => self.lower_variable_as_place(ident),
            TExprKind::ArrayAccess { array, index } => {
                self.lower_array_access_as_place(array, index)
            }
            TExprKind::FieldAccess { object, field_name } => {
                self.lower_field_access_as_place(object, *field_name)
            }
            // I assume a parenthesis expression is an lvalue. This means code like this: '(object).field' is considered valid.
            TExprKind::Paren(inner) => self.lower_expr_as_place(inner),
            TExprKind::This => MIRPlace::Local(self.get_this_local()),
            _ => panic!("Expression is not an lvalue. This should be caught by semantic checker"),
        }
    }
}

// ======================
// Expression lowering.
// ======================
impl MIRLoweringContext {
    fn lower_string_length(&mut self, string_expr: &TExpr) -> MIRValue {
        let string_val = self.lower_expr(string_expr);
        let temp = self.builder().create_temp(Type {
            kind: TypeKind::Int,
            position: string_expr.position,
        });
        self.builder()
            .emit_string_length(MIRPlace::Temp(temp), string_val, string_expr.position);
        MIRValue::Temp(temp)
    }

    fn lower_array_length(&mut self, array_expr: &TExpr) -> MIRValue {
        let array_val = self.lower_expr(array_expr);
        let temp = self.builder().create_temp(Type {
            kind: TypeKind::Int,
            position: array_expr.position,
        });
        self.builder()
            .emit_array_length(MIRPlace::Temp(temp), array_val, array_expr.position);
        MIRValue::Temp(temp)
    }

    fn lower_field_access(&mut self, base_expr: &TExpr, field_name: Ident) -> MIRValue {
        match &base_expr.typ.kind {
            TypeKind::String => {
                if field_name.name == StrID::from("length") {
                    self.lower_string_length(base_expr)
                } else {
                    panic!("String field access got invalid field: {}", field_name.name);
                }
            }
            TypeKind::Array(_) => {
                println!("Lower field access: Array length");
                if field_name.name == StrID::from("length") {
                    self.lower_array_length(base_expr)
                } else {
                    panic!("Array field access got invalid field: {}", field_name.name);
                }
            }
            TypeKind::Class(_) => {
                println!("Lower field access: Class");
                let object_place = self.lower_expr_as_place(base_expr);
                let class_name = if let TypeKind::Class(ident) = &base_expr.typ.kind {
                    ident.name
                } else {
                    panic!(
                        "Field access on non-class type. This should be caught by the semantic checker"
                    );
                };

                // TODO: Make this a map.
                let field_layout = self.symbol_table.get_class_field_layout(class_name);
                let (field_index, (field_name, typ, _)) = field_layout
                    .iter()
                    .enumerate()
                    .find(|(_, (name, _, _))| *name == field_name.name).expect("Field name not found during lowering. This should be caught by the semantic checker.");

                let field_place = MIRPlace::Field {
                    base: Box::new(object_place),
                    class_name,
                    field_name: *field_name,
                    field_index,
                };

                self.lower_place_as_value(&field_place, typ)
            }
            _ => {
                panic!("Field access got invalid type: {}", base_expr.typ);
            }
        }
    }

    fn lower_expr(&mut self, expr: &TExpr) -> MIRValue {
        match &expr.kind {
            TExprKind::LiteralInt(val) => MIRValue::IntConst(*val),
            TExprKind::LiteralTrue => MIRValue::BoolConst(true),
            TExprKind::LiteralFalse => MIRValue::BoolConst(false),
            TExprKind::This => {
                // TODO: This may be bugged.
                let local_id = self.variable_map.get(&StrID::from("this")).unwrap();
                MIRValue::Local(*local_id)
            }
            TExprKind::LiteralString(str) => MIRValue::StringConst(str.clone()),
            TExprKind::LiteralNull | TExprKind::TypedNull(_) => MIRValue::Null,
            TExprKind::Var(_) => {
                let place = self.lower_expr_as_place(expr);
                self.lower_place_as_value(&place, &expr.typ)
            }
            TExprKind::UnaryOp { op, expr } => self.lower_unary_op(op, expr),
            TExprKind::BinaryOp { op, lhs, rhs } => self.lower_binary_op(op, lhs, rhs),
            TExprKind::App { callee, args } => self.lower_call(callee, args, expr.position),
            TExprKind::NewObject(typ) => self.lower_new_object(typ, expr.position),
            TExprKind::NewArray { element_type, size } => {
                self.lower_new_array(element_type, size, expr.position)
            }
            TExprKind::ArrayAccess { .. } => {
                let place = self.lower_expr_as_place(expr);
                self.lower_place_as_value(&place, &expr.typ)
            }
            TExprKind::FieldAccess { object, field_name } => {
                self.lower_field_access(object, *field_name)
            }
            TExprKind::Method {
                object,
                method_name,
                args,
            } => self.lower_method_call(object, method_name.name, args, expr.position, &expr.typ),
            TExprKind::Paren(expr) => self.lower_expr(expr),
            TExprKind::ExprTypedNull(_) => MIRValue::Null,
        }
    }

    fn lower_place_as_value(&mut self, place: &MIRPlace, typ: &Type) -> MIRValue {
        match place {
            MIRPlace::Local(id) => MIRValue::Local(*id),
            MIRPlace::Temp(id) => MIRValue::Temp(*id),
            MIRPlace::Field {
                base,
                field_index,
                class_name,
                ..
            } => {
                let base_type = Type {
                    kind: TypeKind::Class(Ident {
                        name: *class_name,
                        position: typ.position,
                    }),
                    position: typ.position,
                };

                let obj_value = self.lower_place_as_value(base, &base_type);
                let temp_dest = self.builder().create_temp(typ.clone());
                // TODO: Probably remove position from here.
                self.builder().emit_field_get(
                    MIRPlace::Temp(temp_dest),
                    obj_value,
                    *field_index,
                    typ.position,
                );
                MIRValue::Temp(temp_dest)
            }
            MIRPlace::Index { base, index } => {
                let array_value = self.lower_place_as_value(base, typ);
                let temp_dest = self.builder().create_temp(typ.clone());
                // TODO: Probably remove position from here.
                self.builder().emit_array_get(
                    MIRPlace::Temp(temp_dest),
                    array_value,
                    *index.clone(),
                    typ.position,
                );
                MIRValue::Temp(temp_dest)
            }
        }
    }

    fn lower_logic_op(&mut self, op: &BinaryOp, lhs: &TExpr, rhs: &TExpr) -> MIRValue {
        let res_temp = self.builder().create_temp(Type {
            kind: TypeKind::Bool,
            position: lhs.position,
        });

        // Lower the left side.
        let lhs_val = self.lower_expr(lhs);

        let eval_rhs_block = self.builder().create_block();
        let end_block = self.builder().create_block();

        match op {
            BinaryOp::And => {
                // Emit the default value for short-circuit AND.
                self.builder().emit_assign(
                    MIRPlace::Temp(res_temp),
                    MIRValue::BoolConst(false),
                    lhs.position,
                );

                // If expression is true, we move to the next block. If false we skip the rhs.
                self.builder()
                    .emit_branch(lhs_val, eval_rhs_block, end_block, lhs.position);
            }
            BinaryOp::Or => {
                // Emit the default value for short-circuit OR.
                self.builder().emit_assign(
                    MIRPlace::Temp(res_temp),
                    MIRValue::BoolConst(true),
                    lhs.position,
                );
                // If expression is true, we move to the end block. If false we move to the rhs.
                self.builder()
                    .emit_branch(lhs_val, end_block, eval_rhs_block, rhs.position);
            }
            _ => unreachable!("Non logic op in lower logic op"),
        }

        // Now we evaluate the right side.
        self.builder().set_current_block(eval_rhs_block);
        let rhs_val = self.lower_expr(rhs);
        self.builder()
            .emit_assign(MIRPlace::Temp(res_temp), rhs_val, rhs.position);
        self.builder().emit_jump(end_block, rhs.position);
        self.builder().set_current_block(end_block);
        MIRValue::Temp(res_temp)
    }

    fn binary_op_to_mir(&self, op: &BinaryOp, lhs_type: &Type) -> MIROperation {
        match op {
            BinaryOp::Add => {
                if matches!(lhs_type.kind, TypeKind::String) {
                    MIROperation::StringConcat
                } else {
                    MIROperation::IntAdd
                }
            }
            BinaryOp::Sub => MIROperation::IntSub,
            BinaryOp::Mul => MIROperation::IntMul,
            BinaryOp::Div => MIROperation::IntDiv,
            BinaryOp::Mod => MIROperation::IntMod,
            BinaryOp::Eq => {
                if matches!(lhs_type.kind, TypeKind::String) {
                    MIROperation::StringEq
                } else {
                    MIROperation::IntEq
                }
            }
            BinaryOp::Neq => {
                if matches!(lhs_type.kind, TypeKind::String) {
                    MIROperation::StringNeq
                } else {
                    MIROperation::IntNeq
                }
            }
            BinaryOp::Lt => MIROperation::IntLt,
            BinaryOp::Le => MIROperation::IntLe,
            BinaryOp::Gt => MIROperation::IntGt,
            BinaryOp::Ge => MIROperation::IntGe,
            _ => unreachable!("Logic ops should be handled separately"),
        }
    }

    fn binary_op_result_type(
        &self,
        op: &BinaryOp,
        lhs_type: &Type,
        position: SourcePosition,
    ) -> Type {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                lhs_type.clone()
            }
            _ => Type {
                kind: TypeKind::Bool,
                position,
            },
        }
    }

    fn lower_binary_op(&mut self, op: &BinaryOp, lhs: &TExpr, rhs: &TExpr) -> MIRValue {
        // If a logic op is encountered we have to lower it lazily.
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            return self.lower_logic_op(op, lhs, rhs);
        }

        // Otherwise it's simple.
        let lhs_val = self.lower_expr(lhs);
        let rhs_val = self.lower_expr(rhs);
        let mir_op = self.binary_op_to_mir(op, &lhs.typ);
        let result_type = self.binary_op_result_type(op, &lhs.typ, lhs.position);

        let temp = self.builder().create_temp(result_type);
        self.builder()
            .emit_binary(mir_op, MIRPlace::Temp(temp), lhs_val, rhs_val, lhs.position);
        MIRValue::Temp(temp)
    }

    fn lower_unary_op(&mut self, op: &UnaryOp, expr: &TExpr) -> MIRValue {
        let value = self.lower_expr(expr);
        let mir_op = match op {
            UnaryOp::Neg => MIROperation::IntNeg,
            UnaryOp::Not => MIROperation::BoolNot,
        };

        let temp = self.builder().create_temp(expr.typ.clone());
        self.builder()
            .emit_unary(mir_op, MIRPlace::Temp(temp), value, expr.position);

        MIRValue::Temp(temp)
    }

    fn lower_call(&mut self, callee: &TExpr, args: &[TExpr], position: SourcePosition) -> MIRValue {
        let func_ident = match &callee.kind {
            TExprKind::Var(ident) => ident,
            _ => panic!("MIR Lowering: only direct function calls are supported"),
        };

        // Lower arguments.
        let arg_values: Vec<MIRValue> = args.iter().map(|arg| self.lower_expr(arg)).collect();

        if !matches!(callee.typ.kind, TypeKind::Fun(..)) {
            panic!("Callee is not a function type");
        }
        let return_type = if let TypeKind::Fun(ret, _) = &callee.typ.kind {
            (*ret).clone()
        } else {
            unreachable!()
        };

        if !matches!(return_type.kind, TypeKind::Void) {
            let temp = self.builder().create_temp(*return_type);
            self.builder().emit_call(
                Some(MIRPlace::Temp(temp)),
                func_ident.name,
                arg_values,
                position,
            );
            MIRValue::Temp(temp)
        } else {
            self.builder()
                .emit_call(None, func_ident.name, arg_values, position);
            MIRValue::Unit
        }
    }

    fn lower_method_call(
        &mut self,
        object: &TExpr,
        method_name: StrID,
        args: &[TExpr],
        position: SourcePosition,
        return_type: &Type,
    ) -> MIRValue {
        let object_val = self.lower_expr(object);
        let arg_values: Vec<MIRValue> = args.iter().map(|arg| self.lower_expr(arg)).collect();

        let class_name = if let TypeKind::Class(ident) = &object.typ.kind {
            ident.name
        } else {
            panic!("Method call on non-class type");
        };

        let vtable = self.symbol_table.get_class_vtable_layout(class_name);
        let (vtable_index, _) = vtable
            .iter()
            .enumerate()
            .find(|(_, func_info)| func_info.ident.name == method_name)
            .expect("Method not found in vtable");

        if !matches!(return_type.kind, TypeKind::Void) {
            let temp = self.builder().create_temp(return_type.clone());
            self.builder().emit_virtual_call(
                Some(MIRPlace::Temp(temp)),
                object_val,
                vtable_index,
                arg_values,
                position,
            );
            MIRValue::Temp(temp)
        } else {
            self.builder()
                .emit_virtual_call(None, object_val, vtable_index, arg_values, position);
            MIRValue::Unit
        }
    }

    fn lower_new_object(&mut self, typ: &Type, position: SourcePosition) -> MIRValue {
        let class_name = match &typ.kind {
            TypeKind::Class(ident) => ident.name,
            _ => panic!("New object requires class type"),
        };

        let temp = self.builder().create_temp(typ.clone());
        self.builder()
            .emit_new_object(MIRPlace::Temp(temp), class_name, position);
        MIRValue::Temp(temp)
    }

    fn lower_new_array(
        &mut self,
        element_type: &Type,
        size: &TExpr,
        position: SourcePosition,
    ) -> MIRValue {
        let size_val = self.lower_expr(size);

        let array_type = Type {
            kind: TypeKind::Array(Box::new(element_type.clone())),
            position,
        };

        let temp = self.builder().create_temp(array_type);
        self.builder()
            .emit_new_array(MIRPlace::Temp(temp), size_val, position);

        MIRValue::Temp(temp)
    }
}

/// Other
impl MIRLoweringContext {
    fn mangle_method_name(&self, class_name: StrID, method_name: StrID) -> StrID {
        StrID::from(format!("{}_{}", class_name, method_name))
    }
}
