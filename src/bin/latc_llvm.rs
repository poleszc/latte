use clap::Parser;
use latte::driver::run_compiler;
use latte::llvm::compile_llvm;

fn main() {
    eprintln!("[LATTE]: Running compiler");
    let args = latte::LatteArgs::parse();
    if let Err(_) = run_compiler(compile_llvm, &args) {
        eprintln!("ERROR");
        std::process::exit(1);
    }
    eprintln!("OK");
}
