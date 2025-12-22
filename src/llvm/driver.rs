/// Main compilation handling the compilation to LLVM-IR
/// - Determines the output file names
/// - Runs the code generation via the LLVMGenerator object.
/// - Runs `llvm-as` to assemble the bitcode
/// - Runs the `lli` on the generated bitcode if requested.
use crate::config::LatteArgs;
use crate::errors::CompilerError;
use crate::lir::LIRProgram;
use crate::llvm::LLVMGenerator;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn compile_llvm(
    lir_program: &LIRProgram,
    dir: &Path,
    stem: &str,
    args: &LatteArgs,
) -> Result<(), CompilerError> {
    eprintln!("[LLVM] Lowering to LLVM...");
    let llvm_generator = LLVMGenerator::new();
    let llvm_ir = llvm_generator.generate_program(&lir_program);
    eprintln!("[LLVM] Lowering to LLVM completed successfully...");

    eprintln!("[LLVM] Dumped LLVM IR to file...");
    let ll_path = dir.join(format!("{}.ll", stem));
    let bc_path = dir.join(format!("{}.bc", stem));
    let linked_bc_path = dir.join(format!("{}.linked.bc", stem));
    fs::write(&ll_path, llvm_ir)?;
    eprintln!("[LLVM] Generated file: {}", ll_path.display());

    // eprintln!("[LLVM] Assembling bitcode...");
    // let as_output = Command::new("llvm-as")
    //     .arg(&ll_path)
    //     .arg("-o")
    //     .arg(&bc_path)
    //     .output()?;

    // if !as_output.status.success() {
    //     return Err(CompilerError::Other(format!(
    //         "LLVM assembler failed.\n[STDERR]:\n{}",
    //         String::from_utf8_lossy(&as_output.stderr)
    //     )));
    // }
    // eprintln!("[LLVM] Generated file: {}", bc_path.display());

    // eprintln!("[LLVM] Linking with runtime lib...");

    // TODO: Make this not hardcoded
    let runtime_path_ll = "lib/runtime.ll";
    let runtime_path_bc = "lib/runtime.bc";
    let as_output = Command::new("llvm-as")
        .arg(&runtime_path_ll)
        .arg("-o")
        .arg(&runtime_path_bc)
        .output()?;

    if !as_output.status.success() {
        return Err(CompilerError::Other(format!(
            "LLVM assembler failed.\n[STDERR]:\n{}",
            String::from_utf8_lossy(&as_output.stderr)
        )));
    }
    eprintln!("[LLVM] Generated file: {}", bc_path.display());

    if !Path::new(runtime_path_bc).exists() {
        return Err(CompilerError::Other(
            "runtime.ll not found.".to_string(),
        ));
    }

    let link_output = Command::new("llvm-link")
        .arg(&bc_path)
        .arg(runtime_path_bc)
        .arg("-o")
        .arg(&linked_bc_path)
        .output()?;

    if !link_output.status.success() {
        return Err(CompilerError::Other(format!(
            "LLVM linker failed.\n[STDERR]:\n{}",
            String::from_utf8_lossy(&link_output.stderr)
        )));
    }
    eprintln!(
        "[LLVM] Generated linked bitcode: {}",
        linked_bc_path.display()
    );

    if args.run {
        eprintln!("[LLVM] Running program");
        let run_output = Command::new("lli").arg(&linked_bc_path).output()?;

        if !run_output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(&run_output.stdout))
        }

        if !run_output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&run_output.stderr))
        }

        if !run_output.status.success() {
            return Err(CompilerError::Other(
                "Program execution exited with a non-zero status.".to_string(),
            ));
        }
    }

    Ok(())
}
