use crate::{
    lir::{LIRClass, LIRField, LIRProgram, lowering::function_lowering::LIRFunctionBuilder},
    mir::{MIRClass, MIRProgram},
};

pub struct LIRLowering;

impl LIRLowering {
    pub fn lower(mir_program: MIRProgram) -> LIRProgram {
        let mut lir_program = LIRProgram::new();

        for class in mir_program.classes.values() {
            lir_program.add_class(Self::lower_class_to_lir(class));
        }

        // Transform all functions to SSA.
        for func_name in &mir_program.function_order {
            if let Some(mir_func) = mir_program.functions.get(func_name) {
                let mut function_builder = LIRFunctionBuilder::new();
                let lir_func = function_builder.lower(mir_func);
                lir_program.add_function(lir_func);
            }
        }

        lir_program
    }

    pub fn lower_class_to_lir(mir_class: &MIRClass) -> LIRClass {
        LIRClass {
            name: mir_class.name,
            fields: mir_class
                .fields
                .iter()
                .map(|field| LIRField {
                    name: field.name,
                    typ: field.typ.clone(),
                    index: field.index,
                })
                .collect(),
            vtable: mir_class.vtable.clone(),
            source_position: mir_class.source_position,
        }
    }
}
