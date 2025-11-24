// TODO: Remove those eventually.
#![allow(dead_code)]
#![allow(unused_variables)]
use std::collections::HashMap;

use crate::{
    mir::{
        LocalID, MIRClass, MIRFunction, MIRPlace, MIRProgram, MIRValue,
        function_builder::MIRFunctionBuilder, program_builder::MIRProgramBuilder,
    },
    parser::ast::{
        Block, ClassDef, ClassEntryKind, Expr, ExprKind, FuncDef, Ident, Item, Program, Stmt,
        StmtKind, TopDefKind, Type, TypeKind,
    },
    semantics::SymbolTable,
    utils::StrID,
};

pub struct MIRLoweringContext {
    symbol_table: SymbolTable,
    variable_map: HashMap<StrID, LocalID>,
    current_function: Option<MIRFunctionBuilder>,
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
        }
    }

    /// Main function performing the lowering for the whole program.
    pub fn lower_program(&mut self, program: &Program) -> MIRProgram {
        let mut builder = MIRProgramBuilder::new();

        // Lower all classes to MIR.
        for top_def in &program.defs {
            if let TopDefKind::Class(class_def) = &top_def.kind {
                let mir_class = self.lower_class(class_def);
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
                for entry in &class_def.entries {
                    if let ClassEntryKind::Method(method) = &entry.kind {
                        let mir_function = self.lower_method(class_def, method);
                        builder.add_function(mir_function);
                    }
                }
            }
        }
        builder.build()
    }
}

// ======================
// Top Def lowering.
// ======================
impl MIRLoweringContext {
    fn lower_class(&mut self, class_def: &ClassDef) -> MIRClass {
        todo!()
    }

    fn lower_method(&mut self, class_def: &ClassDef, method: &FuncDef) -> MIRFunction {
        // TODO: Add this as first param.
        self.lower_function(method)
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
            StmtKind::Decl(ty, items) => self.lower_declaration(ty, items),
            StmtKind::Assign { target, value } => self.lower_assign(target, value),
            StmtKind::Return { expr } => self.lower_return(expr),
            StmtKind::Incr(expr) => self.lower_increment(expr),
            StmtKind::Decr(expr) => self.lower_decrement(expr),
            StmtKind::If {
                cond,
                if_true,
                if_false,
            } => self.lower_if(cond, if_true, if_false),
            StmtKind::While { cond, body } => self.lower_while(cond, body),
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
            } => self.lower_for_each(element_type, item_name, collection, body),
            StmtKind::Expr(expr) => {
                self.lower_expr(expr);
            }
            StmtKind::FuncDef(func_def) => todo!("Nested functions"),
        }
    }

    fn lower_declaration(&mut self, typ: &Type, items: &Vec<Item>) {
        todo!()
    }

    fn lower_assign(&mut self, target: &Expr, value: &Expr) {
        todo!()
    }

    fn lower_return(&mut self, target: &Option<Box<Expr>>) {
        todo!()
    }

    fn lower_increment(&mut self, expr: &Expr) {
        todo!()
    }

    fn lower_decrement(&mut self, expr: &Expr) {
        todo!()
    }

    fn lower_if(&mut self, cond: &Expr, if_true: &Stmt, if_false: &Option<Box<Stmt>>) {
        todo!()
    }

    fn lower_while(&mut self, cond: &Expr, body: &Stmt) {
        todo!()
    }

    fn lower_for(
        &mut self,
        init: &Option<Box<Stmt>>,
        cond: &Option<Box<Expr>>,
        post: &Option<Box<Expr>>,
        body: &Stmt,
    ) {
        todo!()
    }

    fn lower_for_each(
        &mut self,
        element_type: &Type,
        item_name: &Ident,
        collection: &Expr,
        body: &Stmt,
    ) {
        todo!()
    }
}

// ======================
// Expression lowering.
// ======================
impl MIRLoweringContext {
    /// Lower expression and return it's place.
    fn lower_expr_as_place(&mut self, expr: &Expr) -> MIRPlace {
        todo!()
    }

    fn lower_expr(&mut self, expr: &Expr) -> MIRValue {
        match &expr.kind {
            ExprKind::Var(ident) => todo!(),
            ExprKind::LiteralInt(_) => todo!(),
            ExprKind::LiteralTrue => todo!(),
            ExprKind::LiteralFalse => todo!(),
            ExprKind::This => todo!(),
            ExprKind::LiteralString(_) => todo!(),
            ExprKind::LiteralNull => todo!(),
            ExprKind::App { callee, args } => todo!(),
            ExprKind::ArrayAccess { array, index } => todo!(),
            ExprKind::FieldAccess { object, field_name } => todo!(),
            ExprKind::Method {
                object,
                method_name,
                args,
            } => todo!(),
            ExprKind::Paren(expr) => todo!(),
            ExprKind::UnaryOp { op, expr } => todo!(),
            ExprKind::BinaryOp { op, lhs, rhs } => todo!(),
            ExprKind::NewObject(_) => todo!(),
            ExprKind::NewArray { element_type, size } => todo!(),
            ExprKind::TypedNull(_) => todo!(),
            ExprKind::ExprTypedNull(expr) => todo!(),
        }
    }
}
