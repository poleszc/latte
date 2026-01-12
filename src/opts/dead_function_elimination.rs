/// Dead Function Elimination - removed functions which are not called.
use crate::{lir::*, opts::OptimizationPass, utils::StrID};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct DeadFunctionElimination {}

impl DeadFunctionElimination {
    pub fn new() -> Self {
        Self {}
    }

    fn find_live_functions(&self, program: &LIRProgram) -> HashSet<StrID> {
        let mut live_functions = HashSet::new();
        let mut worklist = VecDeque::new();

        let main_id = StrID::from("main");
        live_functions.insert(main_id);
        worklist.push_back(main_id);

        for class in program.classes.values() {
            for method_name in &class.vtable {
                if !live_functions.contains(method_name) {
                    live_functions.insert(*method_name);
                    worklist.push_back(*method_name);
                }
            }
        }

        while let Some(func_name) = worklist.pop_front() {
            if let Some(func) = program.functions.get(&func_name) {
                for block in func.blocks.values() {
                    for instr in &block.instructions {
                        if instr.operation == LIROperation::Call {
                            if let Some(LIRValue::Function(target)) = instr.operands.first() {
                                if !live_functions.contains(target) {
                                    live_functions.insert(*target);
                                    worklist.push_back(*target);
                                }
                            }
                        }
                    }
                }
            }
        }

        live_functions
    }
}

impl OptimizationPass for DeadFunctionElimination {
    fn name(&self) -> String {
        String::from("Dead Code Elimination")
    }

    fn run_on_function(&mut self, _function: &mut LIRFunction) -> bool {
        false
    }

    fn run_on_program(&mut self, program: &mut LIRProgram) -> bool {
        let live_functions = self.find_live_functions(program);

        let old_count = program.functions.len();

        program
            .functions
            .retain(|name, _| live_functions.contains(name));

        program
            .function_order
            .retain(|name| live_functions.contains(name));

        program.functions.len() != old_count
    }
}

impl Default for DeadFunctionElimination {
    fn default() -> Self {
        Self::new()
    }
}
