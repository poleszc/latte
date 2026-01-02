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
        let old_variable_map = self.variable_map.clone();
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
        }
        self.variable_map = old_variable_map;
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
            TStmtKind::ForEach {
                element_type,
                item_name,
                collection,
                body,
            } => self.lower_for_each(element_type, item_name, collection, body, stmt.position),
            TStmtKind::Expr(expr) => {
                self.lower_expr(expr);
            }
        }
    }

    fn lower_declaration(&mut self, typ: &Type, items: &Vec<TItem>, stmt_position: SourcePosition) {
        for item in items {
            match &item.kind {
                TItemKind::NoInit(ident) => {
                    let local_id =
                        self.builder()
                            .create_local(Some(ident.name), typ.clone(), false, None);
                    let default_value = self.get_default_initialization_value(typ);

                    self.builder().emit_assign(
                        MIRPlace::Local(local_id),
                        default_value,
                        stmt_position,
                    );
                    self.variable_map.insert(ident.name, local_id);
                }
                TItemKind::Init(ident, expr) => {
                    // Calculate the expression and assign to the var.
                    // Lower the expression in the previous context, so things like int i = i + 1 are handled well.
                    let value = self.lower_expr(expr);
                    let local_id =
                        self.builder()
                            .create_local(Some(ident.name), typ.clone(), false, None);

                    self.variable_map.insert(ident.name, local_id);
                    self.builder()
                        .emit_assign(MIRPlace::Local(local_id), value, stmt_position);
                }
            }
        }
    }

    // Return the default initialization value for a type.
    fn get_default_initialization_value(&self, typ: &Type) -> MIRValue {
        match &typ.kind {
            TypeKind::Int => MIRValue::IntConst(0),
            TypeKind::Bool => MIRValue::BoolConst(false),
            TypeKind::String => MIRValue::StringConst("".to_string()),
            TypeKind::Array(_) | TypeKind::Class(_) => MIRValue::Null,
            _ => panic!("Unhandled default initialization value"),
        }
    }

    fn lower_assign(&mut self, target: &TExpr, value: &TExpr, stmt_position: SourcePosition) {
        let place = self.lower_expr_as_place(target);
        let val = self.lower_expr(value);
        self.lower_write_to_place(place, val, stmt_position);
    }

    fn lower_return(&mut self, expr: &Option<TExpr>, stmt_position: SourcePosition) {
        let value = expr.as_ref().map(|e| self.lower_expr(e));
        self.builder().emit_return(value, stmt_position);
    }

    fn lower_write_to_place(&mut self, place: MIRPlace, value: MIRValue, pos: SourcePosition) {
        match place {
            MIRPlace::Local(_) | MIRPlace::Temp(_) => {
                self.builder().emit_assign(place, value, pos);
            }
            MIRPlace::Field {
                base,
                field_index,
                class_name,
                ..
            } => {
                let base_typ = Type {
                    kind: TypeKind::Class(Ident {
                        name: class_name,
                        position: pos,
                    }),
                    position: pos,
                };
                let base_val = self.lower_place_as_value(&base, &base_typ);

                self.builder()
                    .emit_field_set(base_val, field_index, value, pos);
            }
            MIRPlace::Index { base, index } => {
                let base_val = self.lower_place_as_value(
                    &base,
                    &Type {
                        kind: TypeKind::Int,
                        position: pos,
                    },
                );
                self.builder().emit_array_set(base_val, *index, value, pos);
            }
        }
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
        self.lower_write_to_place(place, MIRValue::Temp(temp), stmt_position);
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
        self.lower_write_to_place(place, MIRValue::Temp(temp), stmt_position);
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
        let old_variable_map = self.variable_map.clone();

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
        self.variable_map = old_variable_map;
    }
}

#[cfg(test)]
mod statement_tests {
    use super::*;
    use crate::mir::builders::MIRFunctionBuilder;
    use crate::parser::ast::{Ident, SourcePosition, Type, TypeKind};
    use crate::semantics::symbol_table::SymbolTable;
    use crate::semantics::tast::*;

    #[test]
    fn test_lower_increment() {
        // i++ -> i = i + 1
        let mut ctx = MIRLoweringContext::new(SymbolTable::new());
        let ty_int = Type {
            kind: TypeKind::Int,
            position: SourcePosition(0, 0),
        };

        let mut builder = MIRFunctionBuilder::new(
            StrID::from("f"),
            ty_int.clone(),
            vec![],
            SourcePosition(0, 0),
        );
        let i_local = builder.create_local(Some(StrID::from("i")), ty_int.clone(), false, None);
        ctx.current_function = Some(builder);
        ctx.variable_map.insert(StrID::from("i"), i_local);

        let i_expr = TExpr {
            kind: TExprKind::Var(Ident {
                name: StrID::from("i"),
                position: SourcePosition(0, 0),
            }),
            typ: ty_int.clone(),
            position: SourcePosition(0, 0),
        };

        ctx.lower_increment(&i_expr, SourcePosition(0, 0));

        let mir_func = ctx.current_function.unwrap().build();
        let instr = &mir_func.blocks[&mir_func.entry_block].instructions;

        // temp = i + 1; i = temp;
        assert_eq!(instr.len(), 2);
        assert_eq!(instr[0].operation, MIROperation::IntAdd);
        assert_eq!(instr[1].operation, MIROperation::Assign);
    }
}
