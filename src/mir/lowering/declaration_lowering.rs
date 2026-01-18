use super::MIRLoweringContext;
use crate::{
    mir::{builders::*, *},
    parser::ast::{Type, TypeKind},
    semantics::tast::{TClassDef, TFuncDef},
    utils::StrID,
};

// ======================
// Top Def lowering.
// ======================
impl MIRLoweringContext {
    pub(super) fn lower_class(&mut self, class_def: &TClassDef) -> MIRClass {
        let class_name = class_def.name.name;
        let field_layout = self.symbol_table.get_class_field_layout(class_name);

        let fields: Vec<MIRField> = field_layout
            .into_iter()
            .enumerate()
            .map(|(index, (name, typ, _pos))| MIRField { name, typ, index })
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
            fields,
            vtable,
            source_position: class_def.position,
        }
    }

    pub(super) fn lower_method(&mut self, class_def: &TClassDef, method: &TFuncDef) -> MIRFunction {
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
            let builder = self.builder();
            if !builder.is_current_block_terminated() {
                builder.emit_return(None, method.position);
            }
        }

        self.current_function.take().unwrap().build()
    }

    pub(super) fn lower_function(&mut self, func_def: &TFuncDef) -> MIRFunction {
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

        // If the last block in the function is unterminated, we either have a void function with an implicit
        // return or a unreachable block, which was created when optimizing if statements.
        // We either add the void return or mark the block as unreachable.
        if !self.builder().is_current_block_terminated() {
            if matches!(func_def.return_type.kind, TypeKind::Void) {
                let params_to_drop: Vec<LocalID> = self.variable_map.values().cloned().collect();
                self.emit_drops(&params_to_drop, func_def.position);
                let builder = self.builder();
                if !builder.is_current_block_terminated() {
                    builder.emit_return(None, func_def.position);
                }
            } else {
                self.builder().emit_unreachable(func_def.position);
            }
        }

        self.current_function.take().unwrap().build()
    }
}

#[cfg(test)]
mod declaration_tests {
    use super::*;
    use crate::parser::ast::*;
    use crate::semantics::symbol_table::SymbolTable;
    use crate::semantics::symbols::ClassInfo;
    use crate::semantics::tast::*;
    use std::collections::HashMap;

    fn setup_ctx() -> MIRLoweringContext {
        MIRLoweringContext::new(SymbolTable::new(), false)
    }

    #[test]
    fn test_method_lowering_injects_this() {
        let mut ctx = setup_ctx();

        let class_name = Ident {
            name: StrID::from("A"),
            position: SourcePosition(0, 0),
        };
        let class_def = TClassDef {
            name: class_name,
            extends: None,
            entries: vec![],
            position: SourcePosition(0, 0),
        };

        let method_def = TFuncDef {
            name: Ident {
                name: StrID::from("foo"),
                position: SourcePosition(0, 0),
            },
            return_type: Type {
                kind: TypeKind::Void,
                position: SourcePosition(0, 0),
            },
            args: vec![Arg {
                arg_type: Type {
                    kind: TypeKind::Int,
                    position: SourcePosition(0, 0),
                },
                name: Ident {
                    name: StrID::from("amount"),
                    position: SourcePosition(0, 0),
                },
                position: SourcePosition(0, 0),
            }],
            body: TBlock {
                stmts: vec![],
                position: SourcePosition(0, 0),
            },
            position: SourcePosition(0, 0),
        };

        let mir_func = ctx.lower_method(&class_def, &method_def);

        // Test mangling.
        assert_eq!(mir_func.name, StrID::from("A_foo"));

        // Test if this is injected.
        assert_eq!(mir_func.parameter_types.len(), 2);
        assert!(matches!(
            mir_func.parameter_types[0].kind,
            TypeKind::Class(_)
        ));
        assert!(matches!(mir_func.parameter_types[1].kind, TypeKind::Int));
        let this_local = mir_func
            .locals
            .iter()
            .find(|l| l.name == Some(StrID::from("this")))
            .expect("Missing this");
        assert!(this_local.is_parameter);
        assert_eq!(this_local.parameter_index, Some(0));
    }

    #[test]
    fn test_void_function_implicit_return() {
        let mut ctx = setup_ctx();

        let func_def = TFuncDef {
            name: Ident {
                name: StrID::from("nothing"),
                position: SourcePosition(0, 0),
            },
            return_type: Type {
                kind: TypeKind::Void,
                position: SourcePosition(0, 0),
            },
            args: vec![],
            body: TBlock {
                stmts: vec![],
                position: SourcePosition(0, 0),
            },
            position: SourcePosition(0, 0),
        };

        let mir_func = ctx.lower_function(&func_def);
        let entry_block = &mir_func.blocks[&mir_func.entry_block];

        match &entry_block.terminator {
            Some(MIRTerminator::Return { value: None, .. }) => {}
            _ => panic!("Void function should have implicit ReturnVoid"),
        }
    }

    #[test]
    fn test_class_layout_lowering() {
        let mut st = SymbolTable::new();

        let class_info = ClassInfo {
            ident: Ident {
                name: StrID::from("Point"),
                position: SourcePosition(0, 0),
            },
            parent: None,
            fields: [
                (
                    StrID::from("x"),
                    (
                        Type {
                            kind: TypeKind::Int,
                            position: SourcePosition(0, 0),
                        },
                        SourcePosition(0, 0),
                    ),
                ),
                (
                    StrID::from("y"),
                    (
                        Type {
                            kind: TypeKind::Int,
                            position: SourcePosition(0, 0),
                        },
                        SourcePosition(0, 0),
                    ),
                ),
            ]
            .into(),
            methods: HashMap::new(),
            field_order: vec![StrID::from("x"), StrID::from("y")],
            method_order: vec![],
            position: SourcePosition(0, 0),
        };
        st.define_class(class_info).unwrap();

        let mut ctx = MIRLoweringContext::new(st, false);
        let t_class_def = TClassDef {
            name: Ident {
                name: StrID::from("Point"),
                position: SourcePosition(0, 0),
            },
            extends: None,
            entries: vec![],
            position: SourcePosition(0, 0),
        };

        let mir_class = ctx.lower_class(&t_class_def);

        assert_eq!(mir_class.fields.len(), 2);
        assert_eq!(mir_class.fields[0].name, StrID::from("x"));
        assert_eq!(mir_class.fields[0].index, 0);
        assert_eq!(mir_class.fields[1].name, StrID::from("y"));
        assert_eq!(mir_class.fields[1].index, 1);
    }
}
