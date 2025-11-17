use clap::Parser;
use latte::driver::run_compiler;
use latte::llvm::compile_llvm;
use latte::report_compiler_error;

fn main() {
    eprintln!("╭───────────────────────────────╮");
    eprintln!("│ Latte Compiler - LLVM Backend │");
    eprintln!("╰───────────────────────────────╯");
    eprintln!();

    if let Err(err) = run_compiler(compile_llvm) {
        let args = latte::LatteArgs::parse();
        let source_code = std::fs::read_to_string(&args.input_file)
            .unwrap_or_else(|_| String::from("// Source file not available"));

        let file_path = args.input_file.to_str().unwrap_or("unknown");
        report_compiler_error(file_path, &source_code, &err);

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
