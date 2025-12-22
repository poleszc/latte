/// Main driver handling the compilation to LLVM-IR
/// - Determines the output file names
/// - Runs the code generation via the LLVMGenerator object.
/// - Runs `llvm-as` to assemble the bitcode of the compiled code.
/// - Runs `llvm-as` to assemble the bitcode of the runtime library.
/// - Link runtime.bc with the compiled code.
/// - Runs the `lli` on the generated bitcode if requested.
use crate::config::LatteArgs;
use crate::errors::CompilerError;
use crate::lir::LIRProgram;
use crate::llvm::LLVMGenerator;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn compile_llvm(
    lir_program: &LIRProgram,
    dir: &Path,
    stem: &str,
    args: &LatteArgs,
) -> Result<(), CompilerError> {
    let ll_path = dir.join(format!("{}.ll", stem));
    lower_lir_to_llvm_and_save(lir_program, &ll_path)?;

    let bc_path = dir.join(format!("{}.bc", stem));
    llvm_assemble(&ll_path, &bc_path)?;

    let runtime_path_ll = &args.runtime_path;
    if !runtime_path_ll.exists() {
        return Err(CompilerError::Other("runtime.ll not found.".to_string()));
    }
    let runtime_path_bc = runtime_path_ll.with_extension("bc");
    llvm_assemble(&runtime_path_ll, &runtime_path_bc)?;

    let linked_bc_path = dir.join(format!("{}_linked.bc", stem));
    llvm_link(&linked_bc_path, vec![bc_path, runtime_path_bc])?;

    if args.run {
        llvm_run(&linked_bc_path)?;
    }

    Ok(())
}

fn llvm_assemble(ll_path: &PathBuf, bc_path: &PathBuf) -> Result<(), CompilerError> {
    eprintln!("[LLVM] Assembling bitcode for: {}", ll_path.display());
    let as_output = Command::new("llvm-as")
        .arg(&ll_path)
        .arg("-o")
        .arg(&bc_path)
        .output()?;

    if !as_output.status.success() {
        return Err(CompilerError::Other(format!(
            "LLVM assembler failed.\n[STDERR]:\n{}",
            String::from_utf8_lossy(&as_output.stderr)
        )));
    }
    eprintln!("[LLVM] Generated file: {}", bc_path.display());
    Ok(())
}

fn lower_lir_to_llvm_and_save(
    lir_program: &LIRProgram,
    ll_path: &PathBuf,
) -> Result<(), CompilerError> {
    eprintln!("[LLVM] Lowering to LLVM...");
    let llvm_generator = LLVMGenerator::new();
    let llvm_ir = llvm_generator.generate_program(&lir_program);
    eprintln!("[LLVM] Lowering to LLVM completed successfully...");
    fs::write(&ll_path, llvm_ir)?;
    eprintln!("[LLVM] Dumped LLVM IR to file: {}", ll_path.display());
    Ok(())
}

fn llvm_link(linked_path: &PathBuf, paths_to_link: Vec<PathBuf>) -> Result<(), CompilerError> {
    let mut command = Command::new("llvm-link");
    for path in paths_to_link {
        command.arg(path);
    }
    let link_output = command.arg("-o").arg(&linked_path).output()?;

    if !link_output.status.success() {
        return Err(CompilerError::Other(format!(
            "LLVM linker failed.\n[STDERR]:\n{}",
            String::from_utf8_lossy(&link_output.stderr)
        )));
    }
    eprintln!("[LLVM] Generated linked bitcode: {}", linked_path.display());
    Ok(())
}

fn llvm_run(bc_path: &PathBuf) -> Result<(), CompilerError> {
    eprintln!("[LLVM] Running program");
    let run_output = Command::new("lli").arg(&bc_path).output()?;
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
    Ok(())
}
