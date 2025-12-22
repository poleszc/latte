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
}
