/// Main file for defining the compiler settings.
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct LatteArgs {
    /// ========== GENERAL ==========
    /// Input file for the compiler.
    pub input_file: PathBuf,
    /// Run the program after successful compilation.
    #[arg(short, long)]
    pub run: bool,
    /// Output file name.
    /// If not specified, name of the file matches the input file.
    #[arg(short, long)]
    pub output: Option<String>,
    /// Output directory.
    /// If not specified, produces the output files in the same directory as the source code.
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<PathBuf>,
    /// Path to the LLVM runtime library.
    #[arg(long, value_name = "PATH", default_value = "lib/runtime.ll")]
    pub runtime_path: PathBuf,

    /// ========== IR ==========
    /// Emits the AST to a .json file.
    #[arg(long)]
    pub emit_ast: bool,
    /// Emits the TAST to a .json file.
    #[arg(long)]
    pub emit_tast: bool,
    /// Emits the Mid Intermediate Representation to a .mir text file.
    #[arg(long)]
    pub emit_mir: bool,
    /// Emits the Low Intermediate Representation to a .lir text file.
    #[arg(long)]
    pub emit_lir: bool,
    /// Emit the Low Intermediate Representation after optimizations to .opt.lit text file.
    #[arg(long)]
    pub emit_lir_opt: bool,
    /// Emits all the intermediate representations.
    #[arg(long)]
    pub emit_all: bool,

    /// ========== OPTS ==========
    /// Enable all optimizations.
    #[arg(short = 'O', long)]
    pub optimize: bool,
    /// Enable constant propagation and constant folding.
    #[arg(long)]
    pub opt_const_prop: bool,
    /// Enable branch folding
    #[arg(long)]
    pub opt_branch_fold: bool,
    /// Enable Dead Block Elimination.
    #[arg(long)]
    pub opt_dbe: bool,
    /// Enable Dead Code Elimination.
    #[arg(long)]
    pub opt_dce: bool,
    /// Enable Global Common Sub-expression Elimination.
    #[arg(long)]
    pub opt_gcse: bool,
    /// Enable Copy Propagation
    #[arg(long)]
    pub opt_copy_prop: bool,
    /// Enable Dead Function Elimination.
    #[arg(long)]
    pub opt_dead_function: bool,
    /// Enable Function Inlining.
    #[arg(long)]
    pub opt_inline: bool,
    /// Enable Phi Elimination.
    #[arg(long)]
    pub opt_phi_elimination: bool,
    /// Enable Empty Block Elimination.
    #[arg(long)]
    pub opt_empty_block: bool,
    /// Enable Block Merging.
    #[arg(long)]
    pub opt_block_merge: bool,
    /// Maximum iterations for the optimizer.
    #[arg(long, default_value_t = 100)]
    pub opt_max_iterations: usize,
    /// Print program state after every optimization pass.
    #[arg(short = 'v', long)]
    pub verbose_opts: bool,
    /// Enable Garbage Collection.
    #[arg(long)]
    pub opt_gc: bool,
}
