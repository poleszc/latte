use clap::Parser;
use latte::driver::run_compiler;
use latte::llvm::compile_llvm;

fn main() {
    eprintln!("╭───────────────────────────────╮");
    eprintln!("│ Latte Compiler - LLVM Backend │");
    eprintln!("╰───────────────────────────────╯");
    eprintln!();

    let args = latte::LatteArgs::parse();
    if let Err(_) = run_compiler(compile_llvm, &args) {
        eprintln!("ERROR");
        eprintln!();
        eprintln!("╭─────────────────────╮");
        eprintln!("│ Compilation failed  │");
        eprintln!("╰─────────────────────╯");
        std::process::exit(1);
    }

    eprintln!("OK");
    eprintln!("╭────────────────────────╮");
    eprintln!("│ Compilation successful │");
    eprintln!("╰────────────────────────╯");
}
