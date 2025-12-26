/// Main file for defining the compiler settings.
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct LatteArgs {
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
    // Path to the LLVM runtime library.
    #[arg(long, value_name = "PATH", default_value = "lib/runtime.ll")]
    pub runtime_path: PathBuf,
    // Emits the AST to a .json file.
    #[arg(long)]
    pub emit_ast: bool,
    // Emits the TAST to a .json file.
    #[arg(long)]
    pub emit_tast: bool,
    // Emits the Mid Intermediate Representation to a .mir text file.
    #[arg(long)]
    pub emit_mir: bool,
    // Emits the Low Intermediate Representation to a .lir text file.
    #[arg(long)]
    pub emit_lir: bool,
}
