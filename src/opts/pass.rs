use crate::lir::{LIRFunction, LIRProgram};

pub trait OptimizationPass {
    /// Returns the name of the optimization.
    fn name(&self) -> String;

    /// Runs the optimization on a single function.
    /// Returns true if optimization made changes, false otherwise.
    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool;

    /// Runs the optimization on the whole program.
    /// Returns true if optimization made changes, false otherwise.
    fn run_on_program(&mut self, program: &mut LIRProgram) -> bool {
        let mut changed = false;
        for func_name in &program.function_order {
            if let Some(function) = program.functions.get_mut(func_name)
                && self.run_on_function(function)
            {
                changed = true;
            }
        }
        changed
    }
}
