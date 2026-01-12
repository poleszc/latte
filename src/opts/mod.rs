use crate::{
    LatteArgs,
    lir::LIRProgram,
    opts::{
        branch_folding::BranchFolding, constant_propagation::ConstantFoldingAndPropagation, copy_propagation::CopyPropagation, dead_block_elimination::DeadBlockElimination, dead_code_elimination::DeadCodeElimination, gcse::GlobalCSE, pass::OptimizationPass
    },
};
mod branch_folding;
mod constant_propagation;
mod dead_block_elimination;
mod dead_code_elimination;
mod gcse;
mod copy_propagation;
mod pass;

pub struct OptimizationConfig {
    pub max_iterations: usize,
    /// Print the program state after each optimization pass.
    pub verbose_opts: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            verbose_opts: false,
        }
    }
}

impl OptimizationConfig {
    pub fn new(verbose: bool) -> Self {
        let mut config = Self::default();
        config.verbose_opts = verbose;
        config
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }
}

pub struct Optimizer {
    config: OptimizationConfig,
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    pub fn new(config: OptimizationConfig, args: &LatteArgs) -> Self {
        let mut optimizer = Self {
            config,
            passes: Vec::new(),
        };
        optimizer.configure_opts(args);
        optimizer
    }

    fn configure_opts(&mut self, args: &LatteArgs) {
        if args.optimize || args.opt_const_prop {
            self.add_pass(Box::new(ConstantFoldingAndPropagation::new()));
        }
        if args.optimize || args.opt_branch_fold {
            self.add_pass(Box::new(BranchFolding::new()));
        }
        if args.optimize || args.opt_dbe {
            self.add_pass(Box::new(DeadBlockElimination::new()));
        }
        if args.optimize || args.opt_dce {
            self.add_pass(Box::new(DeadCodeElimination::new()));
        }
        if args.optimize || args.opt_gcse {
            self.add_pass(Box::new(GlobalCSE::new()));
        }
        if args.optimize || args.opt_copy_prop {
            self.add_pass(Box::new(CopyPropagation::new()));
        }
    }

    pub fn add_pass(&mut self, pass: Box<dyn OptimizationPass>) {
        self.passes.push(pass);
    }

    pub fn run(&mut self, program: &mut LIRProgram) {
        println!("[OPT] Start...");
        println!("[OPT] Max iterations: {}", self.config.max_iterations);

        if self.config.verbose_opts {
            println!("++++++++++++++++++++++++++++++++++++++");
            program.dump_to_stderr();
            println!("++++++++++++++++++++++++++++++++++++++");
        }

        for iteration in 0..self.config.max_iterations {
            let mut program_changed = false;

            println!("[OPT] ======= Iteration {} =======", iteration);

            for pass in &mut self.passes {
                if pass.run_on_program(program) {
                    println!("[OPT] {} made changes", pass.name());
                    if self.config.verbose_opts {
                        println!("++++++++++++++++++++++++++++++++++++++");
                        program.dump_to_stderr();
                        println!("++++++++++++++++++++++++++++++++++++++");
                    }
                    program_changed = true;
                }
            }

            if !program_changed {
                println!(
                    "[OPT] Reached fixed point after {} iterations",
                    iteration + 1
                );
                break;
            }

            if iteration == self.config.max_iterations - 1 {
                println!("[OPT] Reached max iterations without a fixed point");
            }
        }

        println!("[OPT] Done...");
    }
}

pub fn optimize_program(program: &mut LIRProgram, args: &LatteArgs) {
    let config = OptimizationConfig::new(args.verbose_opts);
    let mut optimizer = Optimizer::new(config, args);
    optimizer.run(program);
}
