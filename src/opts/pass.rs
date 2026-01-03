use crate::lir::{LIRFunction, LIRProgram};

pub trait OptimizationPass {
    fn name(&self) -> String;
    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool;
    // TODO: Maybe remove run on program???
    fn run_on_program(&mut self, program: &mut LIRProgram) -> bool {
        let mut changed = false;
        for func_name in &program.function_order {
            if let Some(function) = program.functions.get_mut(func_name) {
                if self.run_on_function(function) {
                    changed = true;
                }
            }
        }
        changed
    }
}
