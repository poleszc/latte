use crate::mir::{MIRProgram, opts::function_inlining::MIRFunctionInlining};

mod function_inlining;

pub fn inline_functions(program: &mut MIRProgram) {
    MIRFunctionInlining::new().run(program);
}