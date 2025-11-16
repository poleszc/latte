/// Main compilation handling the compilation to LLVM-IR
/// - Determines the output file names
/// - Runs the code generation via the LLVMGenerator object.
/// - Runs `llvm-as` to assemble the bitcode
/// - Runs the `lli` on the generated bitcode if requested.
use crate::config::LatteArgs;
use crate::error::CompilerError;
use crate::parser::ast::definitions::Program;
use std::{fs, path::Path, process::Command};

pub fn compile_llvm(
    program: &mut Program,
    dir: &Path,
    stem: &str,
    args: &LatteArgs,
) -> Result<(), CompilerError> {
    eprintln!("[LLVM] Skipping");
    // eprintln!("[LLVM] Generating LLVM IR code...");
    // let ll_path = dir.join(format!("{}.ll", stem));
    // let bc_path = dir.join(format!("{}.bc", stem));
    // let mut generator = LLVMGenerator::new();
    // let llvm_code = generator.lower_program(program)?;

    // fs::write(&ll_path, llvm_code)?;
    // eprintln!("[LLVM] Generated file: {}", ll_path.display());

    // eprintln!("[LLVM] Assembling bitcode...");
    // let output = Command::new("llvm-as")
    //     .arg(&ll_path)
    //     .arg("-o")
    //     .arg(&bc_path)
    //     .output()?;

    // if !output.status.success() {
    //     return Err(CompilerError::ExternalCommand(format!(
    //         "LLVM assembler failed.\n[STDERR]:\n{}",
    //         String::from_utf8_lossy(&output.stderr)
    //     )));
    // }

    // eprintln!("[LLVM] Generated file: {}", bc_path.display());

    // if args.run {
    //     eprintln!("[LLVM] Running program");
    //     let run_output = Command::new("lli").arg(&bc_path).output()?;

    //     if !run_output.stdout.is_empty() {
    //         print!("{}", String::from_utf8_lossy(&run_output.stdout))
    //     }

    //     if !run_output.stderr.is_empty() {
    //         eprintln!("{}", String::from_utf8_lossy(&run_output.stderr))
    //     }

    //     if !run_output.status.success() {
    //         return Err(CompilerError::ExternalCommand(
    //             "Program execution exited with a non-zero status.".to_string(),
    //         ));
    //     }
    // }

    Ok(())
}
