use crate::lir::{LIRFunction, LIRProgram};

pub trait OptimizationPass {
    fn name(&self) -> String;
    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool;
    // TODO: Maybe remove run on program???
    fn run_on_program(&mut self, program: &mut LIRProgram) -> bool;
}
