/// Dead Code elimination:
/// Eliminates unused code.
use crate::{lir::*, opts::OptimizationPass};
use std::collections::HashSet;

pub struct DeadCodeElimination {
    live_registers: HashSet<BlockID>,
}

impl DeadCodeElimination {
    pub fn new() -> Self {
        Self {
            live_registers: HashSet::new(),
        }
    }

    fn find_live_registers(&mut self, function: &LIRFunction) {
        todo!()
    }

    fn eliminate_dead_instructions(&mut self, function: &LIRFunction) -> bool {
        todo!()
    }

    fn eliminate_dead_phis(&mut self, function: &LIRFunction) -> bool {
        todo!()
    }
}

impl OptimizationPass for DeadCodeElimination {
    fn name(&self) -> String {
        String::from("Dead Code Elimination")
    }

    fn run_on_function(&mut self, function: &mut LIRFunction) -> bool {
        self.find_live_registers(function);
        let instructions_eliminated = self.eliminate_dead_instructions(function);
        let phis_eliminated = self.eliminate_dead_phis(function);

        instructions_eliminated || phis_eliminated
    }
}

impl Default for DeadCodeElimination {
    fn default() -> Self {
        Self::new()
    }
}
