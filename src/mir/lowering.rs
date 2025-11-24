// TODO: Remove those eventually.
#![allow(dead_code)]
#![allow(unused_variables)]
use std::{collections::HashMap, hash::Hash};

use clap::builder::{self, Str};

use crate::{
    SourcePosition,
    mir::{
        self, LocalID, MIRClass, MIRField, MIRFunction, MIRInstruction, MIROperation, MIRPlace,
        MIRProgram, MIRValue, function_builder::MIRFunctionBuilder,
        program_builder::MIRProgramBuilder,
    },
    parser::ast::{
        BinaryOp, Block, ClassDef, ClassEntryKind, Expr, ExprKind, FuncDef, Ident, Item, ItemKind,
        Program, Stmt, StmtKind, TopDefKind, Type, TypeKind, UnaryOp,
    },
    semantics::SymbolTable,
    utils::StrID,
};

pub struct MIRLoweringContext {
    symbol_table: SymbolTable,
    variable_map: HashMap<StrID, LocalID>,
    current_function: Option<MIRFunctionBuilder>,
    current_class_fields: HashMap<StrID, (usize, Type)>,
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
    pub fn lower_program(&mut self, program: &Program) -> MIRProgram {
        let mut builder = MIRProgramBuilder::new();

        // Lower all classes to MIR.
        for top_def in &program.defs {
            if let TopDefKind::Class(class_def) = &top_def.kind {
                let mir_class = self.lower_class(class_def);
                dbg!(&mir_class);
                builder.add_class(mir_class);
            }
        }

        // Lower all functions to MIR.
        for top_def in &program.defs {
            if let TopDefKind::Func(func_def) = &top_def.kind {
                let mir_function = self.lower_function(func_def);
                builder.add_function(mir_function);
            }
        }

        // Lower all methods to MIR.
        for top_def in &program.defs {
            if let TopDefKind::Class(class_def) = &top_def.kind {
                // Insert all class fields in the scope to have access to them in methods.
                let class_fields = self
                    .symbol_table
                    .get_class_field_layout(class_def.name.name);
                for (index, (name, typ, _)) in class_fields.iter().enumerate() {
                    self.current_class_fields
                        .insert(*name, (index, typ.clone()));
                }

                for entry in &class_def.entries {
                    if let ClassEntryKind::Method(method) = &entry.kind {
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
    fn mangle_method_name(&self, class_name: StrID, method_name: StrID) -> StrID {
        StrID::from(format!("{}_{}", class_name, method_name))
    }

    fn lower_class(&mut self, class_def: &ClassDef) -> MIRClass {
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

    fn lower_method(&mut self, class_def: &ClassDef, method: &FuncDef) -> MIRFunction {
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
            let builder = self
                .current_function
                .as_mut()
                .expect("Lower method: builder not set");
            builder.emit_return(None, method.position);
        }

        self.current_function.take().unwrap().build()
    }

    fn lower_function(&mut self, func_def: &FuncDef) -> MIRFunction {
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
            let builder = self
                .current_function
                .as_mut()
                .expect("Lower function: builder not set");
            builder.emit_return(None, func_def.position);
        }

        self.current_function.take().unwrap().build()
    }
}
// ======================
// Utils.
// ======================
impl MIRLoweringContext {
    fn place_to_value(&self, place: &MIRPlace) -> MIRValue {
        match place {
            MIRPlace::Local(local_id) => MIRValue::Local(*local_id),
            MIRPlace::Temp(temp_id) => MIRValue::Temp(*temp_id),
            _ => panic!("Cannot convert complex place to value"),
        }
    }
}

// ======================
// Statement lowering.
// ======================
impl MIRLoweringContext {
    fn lower_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Empty => {}
            StmtKind::Block(block) => self.lower_block(block),
            StmtKind::Decl(ty, items) => self.lower_declaration(ty, items, stmt.position),
            StmtKind::Assign { target, value } => self.lower_assign(target, value, stmt.position),
            StmtKind::Return { expr } => self.lower_return(expr, stmt.position),
            StmtKind::Incr(expr) => self.lower_increment(expr, stmt.position),
            StmtKind::Decr(expr) => self.lower_decrement(expr, stmt.position),
            StmtKind::If {
                cond,
                if_true,
                if_false,
            } => self.lower_if(cond, if_true, if_false, stmt.position),
            StmtKind::While { cond, body } => self.lower_while(cond, body, stmt.position),
            StmtKind::For {
                init,
                cond,
                post,
                body,
            } => self.lower_for(init, cond, post, body),
            StmtKind::ForEach {
                element_type,
                item_name,
                collection,
                body,
            } => self.lower_for_each(element_type, item_name, collection, body, stmt.position),
            StmtKind::Expr(expr) => {
                self.lower_expr(expr);
            }
            StmtKind::FuncDef(func_def) => todo!("Nested functions"),
        }
    }

    fn lower_declaration(&mut self, typ: &Type, items: &Vec<Item>, stmt_position: SourcePosition) {
        for item in items {
            match &item.kind {
                ItemKind::NoInit(ident) => {
                    let builder = self
                        .current_function
                        .as_mut()
                        .expect("Lower declaration no init: Builder not set");
                    let local_id = builder.create_local(Some(ident.name), typ.clone(), false, None);
                    self.variable_map.insert(ident.name, local_id);
                }
                ItemKind::Init(ident, expr) => {
                    let builder = self
                        .current_function
                        .as_mut()
                        .expect("Lower declaration init: Builder not set");
                    let local_id = builder.create_local(Some(ident.name), typ.clone(), false, None);
                    self.variable_map.insert(ident.name, local_id);

                    // Calculate the expression and assign to the var.
                    let value = self.lower_expr(expr);
                    let builder = self
                        .current_function
                        .as_mut()
                        .expect("Lower declaration init: Func builder not set");
                    builder.emit_assign(MIRPlace::Local(local_id), value, stmt_position);
                }
            }
        }
    }

    fn lower_assign(&mut self, target: &Expr, value: &Expr, stmt_position: SourcePosition) {
        let place = self.lower_expr_as_place(target);
        let val = self.lower_expr(value);
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower assign: Func builder not set");
        builder.emit_assign(place, val, stmt_position);
    }

    fn lower_return(&mut self, expr: &Option<Box<Expr>>, stmt_position: SourcePosition) {
        let value = expr.as_ref().map(|e| self.lower_expr(e));
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower return: Func builder not set");
        builder.emit_return(value, stmt_position);
    }

    fn lower_increment(&mut self, expr: &Expr, stmt_position: SourcePosition) {
        // i++ => {
        //    temp = i + 1;
        //    i = temp;
        // }

        let place = self.lower_expr_as_place(expr);
        let val = self.place_to_value(&place);
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower increment: Func builder not set");

        // temp = i + 1;
        let temp = builder.create_temp(Type {
            kind: TypeKind::Int,
            position: expr.position,
        });
        builder.emit_binary(
            MIROperation::IntAdd,
            MIRPlace::Temp(temp),
            val,
            MIRValue::IntConst(1),
            stmt_position,
        );

        // i = temp;
        builder.emit_assign(place, MIRValue::Temp(temp), stmt_position);
    }

    fn lower_decrement(&mut self, expr: &Expr, stmt_position: SourcePosition) {
        // i-- => {
        //    temp = i - 1;
        //    i = temp;
        // }

        let place = self.lower_expr_as_place(expr);
        let val = self.place_to_value(&place);
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower increment: Func builder not set");

        // temp = i - 1;
        let temp = builder.create_temp(Type {
            kind: TypeKind::Int,
            position: expr.position,
        });
        builder.emit_binary(
            MIROperation::IntSub,
            MIRPlace::Temp(temp),
            val,
            MIRValue::IntConst(1),
            stmt_position,
        );

        // i = temp;
        builder.emit_assign(place, MIRValue::Temp(temp), stmt_position);
    }

    fn lower_if(
        &mut self,
        cond: &Expr,
        if_true: &Stmt,
        if_false: &Option<Box<Stmt>>,
        stmt_position: SourcePosition,
    ) {
        // TODO: Do that better to not emit else block if it doesn't exist.
        let cond_val = self.lower_expr(cond);
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower if: Func builder not set 1");

        let true_block = builder.create_block();
        let false_block = builder.create_block();
        // Block we return to after if.
        let merge_block = builder.create_block();

        // Emit jumping to if blocks.
        builder.emit_branch(cond_val, true_block, false_block, stmt_position);

        // If true block.
        builder.set_current_block(true_block);
        self.lower_stmt(if_true);
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower if: Func builder not set 2");
        builder.emit_jump(merge_block, stmt_position);

        // If false block.
        builder.set_current_block(false_block);
        if let Some(else_stmt) = if_false {
            self.lower_stmt(&else_stmt);
        }
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower if: Func builder not set 3");
        builder.emit_jump(merge_block, stmt_position);

        // Go to the merge block
        builder.set_current_block(merge_block);
    }

    fn lower_while(&mut self, cond: &Expr, body: &Stmt, stmt_position: SourcePosition) {
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower while: Func builder not set 1");
        let loop_start = builder.create_block();
        let loop_body = builder.create_block();
        let loop_exit = builder.create_block();

        // Jump to the condition check.
        builder.emit_jump(loop_start, stmt_position);

        // Loop condition check.
        builder.set_current_block(loop_start);
        let cond_val = self.lower_expr(cond);
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower while: Func builder not set 2");
        builder.emit_branch(cond_val, loop_body, loop_exit, stmt_position);

        // Loop body.
        builder.set_current_block(loop_body);
        self.lower_stmt(body);
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower while: Func builder not set 3");
        builder.emit_jump(loop_start, stmt_position);

        // Leave the loop.
        builder.set_current_block(loop_exit);
    }

    fn lower_for(
        &mut self,
        init: &Option<Box<Stmt>>,
        cond: &Option<Box<Expr>>,
        post: &Option<Box<Expr>>,
        body: &Stmt,
    ) {
        todo!("MIR lower for")
    }

    fn lower_for_each(
        &mut self,
        element_type: &Type,
        item_name: &Ident,
        collection: &Expr,
        body: &Stmt,
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
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower for each: Func builder not set 1");

        // i = 0;
        let i_temp = builder.create_temp(Type {
            kind: TypeKind::Int,
            position: stmt_position,
        });
        builder.emit_assign(MIRPlace::Temp(i_temp), MIRValue::IntConst(0), stmt_position);

        // T item;
        let item_local =
            builder.create_local(Some(item_name.name), element_type.clone(), false, None);
        self.variable_map.insert(item_name.name, item_local);

        let loop_start = builder.create_block();
        let loop_body = builder.create_block();
        let loop_exit = builder.create_block();

        // Jump to the condition check.
        builder.emit_jump(loop_start, stmt_position);

        // Loop condition check.
        builder.set_current_block(loop_start);

        // temp = arr.length;
        let length_temp = builder.create_temp(Type {
            kind: TypeKind::Int,
            position: stmt_position,
        });
        builder.emit_array_length(
            MIRPlace::Temp(length_temp),
            collection_val.clone(),
            stmt_position,
        );

        // cond = i < length;
        let cond_temp = builder.create_temp(Type {
            kind: TypeKind::Bool,
            position: stmt_position,
        });
        builder.emit_less_then(
            MIRPlace::Temp(cond_temp),
            MIRValue::Temp(i_temp),
            MIRValue::Temp(length_temp),
            stmt_position,
        );
        builder.emit_branch(
            MIRValue::Temp(cond_temp),
            loop_body,
            loop_exit,
            stmt_position,
        );

        // Loop body.
        builder.set_current_block(loop_body);

        builder.emit_array_get(
            MIRPlace::Local(item_local),
            collection_val.clone(),
            MIRValue::Temp(i_temp),
            stmt_position,
        );

        // Loop body.
        self.lower_stmt(body);

        // i = i + 1;
        let builder = self
            .current_function
            .as_mut()
            .expect("Lower for each: Func builder not set 2");

        let temp = builder.create_temp(Type {
            kind: TypeKind::Int,
            position: stmt_position,
        });
        builder.emit_binary(
            MIROperation::IntAdd,
            MIRPlace::Temp(temp),
            MIRValue::Temp(i_temp),
            MIRValue::IntConst(1),
            stmt_position,
        );
        builder.emit_assign(MIRPlace::Temp(i_temp), MIRValue::Temp(temp), stmt_position);

        // Jump to loop start.
        builder.emit_jump(loop_start, stmt_position);

        // Leave the loop.
        builder.set_current_block(loop_exit);
    }
}

// ======================
// Expression lowering.
// ======================
impl MIRLoweringContext {
    /// Lower expression and return it's place.
    fn lower_expr_as_place(&mut self, expr: &Expr) -> MIRPlace {
        match &expr.kind {
            ExprKind::Var(ident) => {
                // First look for the identifier in the local variables.
                if let Some(local_id) = self.variable_map.get(&ident.name) {
                    return MIRPlace::Local(*local_id);
                }

                // If not found look in the class fields.
                if let Some((field_index, t)) = self.current_class_fields.get(&ident.name) {
                    // Retrieve 'this' from locals. It should always exist.
                    let this_local_id = *self
                        .variable_map
                        .get(&StrID::from("this"))
                        .expect("'this' not found in method context");
                    let this_place = MIRPlace::Local(this_local_id);

                    // Now find the field place.
                    let builder = self
                        .current_function
                        .as_ref()
                        .expect("Lower expr as place: func builder not set");
                    let this_info = builder.get_local_info(this_local_id);
                    let class_name = if let TypeKind::Class(class_name) = &this_info.typ.kind {
                        class_name.name
                    } else {
                        panic!("'this' is not a class type")
                    };

                    return MIRPlace::Field {
                        base: Box::new(this_place),
                        class_name,
                        field_name: ident.name,
                        field_index: *field_index,
                    };
                }
                panic!("Local variable not found in locals and method fields");
            }
            ExprKind::ArrayAccess { array, index } => {
                let array_place = self.lower_expr_as_place(array);
                let index_val = self.lower_expr(&index);
                MIRPlace::Index {
                    base: Box::new(array_place),
                    index: Box::new(index_val),
                }
            }
            ExprKind::FieldAccess { object, field_name } => {
                todo!("Lower expr as place: Field access")
            }
            _ => panic!("Expression is not an lvalue. This should be caught by semantic checker"),
        }
    }

    fn lower_expr(&mut self, expr: &Expr) -> MIRValue {
        match &expr.kind {
            ExprKind::LiteralInt(val) => MIRValue::IntConst(*val),
            ExprKind::LiteralTrue => MIRValue::BoolConst(true),
            ExprKind::LiteralFalse => MIRValue::BoolConst(false),
            ExprKind::This => todo!(),
            ExprKind::LiteralString(str) => MIRValue::StringConst(str.clone()),
            ExprKind::LiteralNull => MIRValue::Null,
            ExprKind::Var(ident) => {
                let local_id = self.variable_map.get(&ident.name).expect(&format!(
                    "Variable '{}' not found in the map",
                    ident.name.to_string()
                ));
                MIRValue::Local(*local_id)
            }
            ExprKind::UnaryOp { op, expr } => self.lower_unary_op(op, expr, expr.position),
            ExprKind::BinaryOp { op, lhs, rhs } => {
                self.lower_binary_op(op, lhs, rhs, expr.position)
            }
            ExprKind::App { callee, args } => self.lower_call(callee, args, expr.position),
            ExprKind::NewObject(typ) => self.lower_new_object(typ, expr.position),
            ExprKind::NewArray { element_type, size } => {
                self.lower_new_array(element_type, size, expr.position)
            }

            ExprKind::ArrayAccess { array, index } => todo!(),
            ExprKind::FieldAccess { object, field_name } => todo!(),
            ExprKind::Method {
                object,
                method_name,
                args,
            } => todo!(),
            ExprKind::Paren(expr) => todo!(),
            ExprKind::TypedNull(_) => todo!(),
            ExprKind::ExprTypedNull(expr) => todo!(),
        }
    }

    fn lower_binary_op(
        &mut self,
        op: &BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        position: SourcePosition,
    ) -> MIRValue {
        let lhs_val = self.lower_expr(lhs);
        let rhs_val = self.lower_expr(rhs);
        
        // TODO: We need notice the difference if expr is of type lhs.

        let mir_op = match op {
            BinaryOp::Add => MIROperation::IntAdd,
            BinaryOp::Sub => MIROperation::IntSub,
            BinaryOp::Mul => MIROperation::IntMul,
            BinaryOp::Div => MIROperation::IntDiv,
            BinaryOp::Mod => MIROperation::IntMod,
            BinaryOp::Eq => MIROperation::IntEq,
            BinaryOp::Lt => MIROperation::IntLt,
            BinaryOp::Le => MIROperation::IntLe,
            BinaryOp::Gt => MIROperation::IntGt,
            BinaryOp::Ge => MIROperation::IntGe,
            BinaryOp::Neq => MIROperation::IntNeq,
            BinaryOp::And => MIROperation::BoolAnd,
            BinaryOp::Or => MIROperation::BoolOr,
        };
        

        let builder = self
            .current_function
            .as_mut()
            .expect("Lower binary op: Func builder not set");

        let result_type = match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => Type {
                kind: TypeKind::Int,
                position,
            },
            _ => Type {
                kind: TypeKind::Bool,
                position,
            },
        };

        let temp = builder.create_temp(result_type);
        builder.emit_binary(mir_op, MIRPlace::Temp(temp), lhs_val, rhs_val, position);
        MIRValue::Temp(temp)
    }

    fn lower_unary_op(&mut self, op: &UnaryOp, expr: &Expr, position: SourcePosition) -> MIRValue {
        let value = self.lower_expr(expr);
        let mir_op = match op {
            UnaryOp::Neg => MIROperation::IntNeg,
            UnaryOp::Not => MIROperation::BoolNot,
        };

        let result_type = match op {
            UnaryOp::Neg => Type {
                kind: TypeKind::Int,
                position,
            },
            UnaryOp::Not => Type {
                kind: TypeKind::Bool,
                position,
            },
        };

        let builder = self
            .current_function
            .as_mut()
            .expect("Lower binary op: Func builder not set");
        let temp = builder.create_temp(result_type);
        builder.emit_unary(mir_op, MIRPlace::Temp(temp), value, position);

        MIRValue::Temp(temp)
    }

    fn lower_call(&mut self, callee: &Expr, args: &[Expr], position: SourcePosition) -> MIRValue {
        let func_ident = match &callee.kind {
            ExprKind::Var(ident) => ident,
            _ => panic!("MIR Lowering: only direct function calls are supported"),
        };

        // Lower arguments.
        let arg_values: Vec<MIRValue> = args.iter().map(|arg| self.lower_expr(arg)).collect();

        let builder = self
            .current_function
            .as_mut()
            .expect("Lower call: Func builder not set");

        let return_type = self
            .symbol_table
            .lookup_function(func_ident)
            .expect("Lower call: Func not found")
            .return_type;

        if !matches!(return_type.kind, TypeKind::Void) {
            let temp = builder.create_temp(return_type);
            builder.emit_call(
                Some(MIRPlace::Temp(temp)),
                func_ident.name,
                arg_values,
                position,
            );
            MIRValue::Temp(temp)
        } else {
            builder.emit_call(None, func_ident.name, arg_values, position);
            MIRValue::Unit
        }
    }

    fn lower_new_object(&mut self, typ: &Type, position: SourcePosition) -> MIRValue {
        let class_name = match &typ.kind {
            TypeKind::Class(ident) => ident.name,
            _ => panic!("New object requires class type"),
        };

        let builder = self
            .current_function
            .as_mut()
            .expect("Lower new object: Func builder not set");
        let temp = builder.create_temp(typ.clone());
        builder.emit_new_object(MIRPlace::Temp(temp), class_name, position);
        MIRValue::Temp(temp)
    }

    fn lower_new_array(
        &mut self,
        element_type: &Type,
        size: &Expr,
        position: SourcePosition,
    ) -> MIRValue {
        let size_val = self.lower_expr(size);

        let builder = self
            .current_function
            .as_mut()
            .expect("Lower new array: Func builder not set");

        let array_type = Type {
            kind: TypeKind::Array(Box::new(element_type.clone())),
            position,
        };

        let temp = builder.create_temp(array_type);
        builder.emit_new_array(MIRPlace::Temp(temp), size_val, position);

        MIRValue::Temp(temp)
    }
}
