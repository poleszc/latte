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
            // TODO: Make a utility which unwraps the builder.
            let builder = self.builder();
            if !builder.is_current_block_terminated() {
                builder.emit_return(None, method.position);
            }
        }

        self.current_function.take().unwrap().build()
    }

    pub(super) fn lower_function(&mut self, func_def: &TFuncDef) -> MIRFunction {
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
