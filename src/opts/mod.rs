use crate::{lir::LIRProgram, opts::{dead_block_elimination::DeadBlockElimination, dead_code_elimination::DeadCodeElimination}};
mod branch_folding;
mod constant_propagation;
mod dead_code_elimination;
mod dead_block_elimination;
mod pass;

pub use branch_folding::BranchFolding;
pub use constant_propagation::ConstantFoldingAndPropagation;
pub use pass::OptimizationPass;

pub struct OptimizationConfig {
    pub max_iterations: usize,
    pub verbose: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            verbose: true,
        }
    }
}

impl OptimizationConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_verbosity(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

pub struct Optimizer {
    config: OptimizationConfig,
    passes: Vec<Box<dyn OptimizationPass>>,
}
impl Optimizer {
    pub fn new(config: OptimizationConfig) -> Self {
        let mut optimizer = Self {
            config,
            passes: Vec::new(),
        };
        optimizer.configure_opts();
        optimizer
    }

    // TODO: Insert based on config
    fn configure_opts(&mut self) {
        self.add_pass(Box::new(ConstantFoldingAndPropagation::new()));
        self.add_pass(Box::new(BranchFolding::new()));
        self.add_pass(Box::new(DeadBlockElimination::new()));
        self.add_pass(Box::new(DeadCodeElimination::new()));
    }

    pub fn add_pass(&mut self, pass: Box<dyn OptimizationPass>) {
        self.passes.push(pass);
    }

    pub fn run(&mut self, program: &mut LIRProgram) {
        if self.config.verbose {
            println!("[OPT] Start...");
            println!("[OPT] Max iterations: {}", self.config.max_iterations);
        }

        for iteration in 0..self.config.max_iterations {
            let mut program_changed = false;

            if self.config.verbose {
                println!("[OPT] ======= Iteration {} =======", iteration);
            }

            for pass in &mut self.passes {
                let pass_name = pass.name();
                let mut pass_changed = false;

                if pass.run_on_program(program) {
                    pass_changed = true;
                }

                for func_name in &program.function_order.clone() {
                    if let Some(function) = program.functions.get_mut(func_name) {
                        if pass.run_on_function(function) {
                            pass_changed = true;
                        }
                    }
                }

                if pass_changed {
                    program_changed = true;
                    if self.config.verbose {
                        println!("[OPT] {} made changes", pass_name);
                    }
                }
            }

            if !program_changed {
                if self.config.verbose {
                    println!(
                        "[OPT] Reached fixed point after {} iterations",
                        iteration + 1
                    );
                }
                break;
            }

            if iteration == self.config.max_iterations - 1 {
                println!("[OPT] Reached max iterations without a fixed point");
            }
        }

        if self.config.verbose {
            println!("[OPT] Done...");
        }
    }
}

pub fn optimize_program(program: &mut LIRProgram) {
    let config = OptimizationConfig::new();
    let mut optimizer = Optimizer::new(config);
    optimizer.run(program);
}
