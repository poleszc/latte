use clap::Parser;
use latte::driver::run_compiler;
use latte::llvm::compile_llvm;
use latte::report_error;

fn main() {
    eprintln!("[LLVM Backend] Running Latte Compiler");
    if let Err(err) = run_compiler(compile_llvm) {
        let args = latte::LatteArgs::parse();
        let source_code = std::fs::read_to_string(&args.input_file).unwrap_or_default();
        report_error(args.input_file.to_str().unwrap(), &source_code, err);
        eprintln!("\n[LLVM Backend] Compilation failed");
        std::process::exit(1);
    }
    eprintln!("[LLVM Backend] Compilation Successful");
}
