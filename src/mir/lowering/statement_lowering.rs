use clap::builder::Str;

use super::MIRLoweringContext;
use crate::{
    SourcePosition,
    mir::*,
    parser::ast::{Ident, Type, TypeKind},
    semantics::tast::{TBlock, TExpr, TItem, TItemKind, TStmt, TStmtKind},
    utils::StrID,
};

// ======================
// Statement lowering.
// ======================
impl MIRLoweringContext {
    pub(super) fn lower_block(&mut self, block: &TBlock) {
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
        let i_local = self.builder().create_local(
            Some(StrID::from("for_each_i_temp")),
            Type {
                kind: TypeKind::Int,
                position: stmt_position,
            },
            false,
            None,
        );
        self.builder().emit_assign(
            MIRPlace::Local(i_local),
            MIRValue::IntConst(0),
            stmt_position,
        );

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
            MIRValue::Local(i_local),
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
            MIRValue::Local(i_local),
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
                MIRValue::Local(i_local),
                MIRValue::IntConst(1),
                stmt_position,
            );
            self.builder().emit_assign(
                MIRPlace::Local(i_local),
                MIRValue::Temp(temp),
                stmt_position,
            );

            // Jump to loop start.
            self.builder().emit_jump(loop_start, stmt_position);
        }

        // Leave the loop.
        self.builder().set_current_block(loop_exit);
    }
}
