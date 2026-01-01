use clap::Parser;
use latte::driver::run_compiler;
use latte::llvm::compile_llvm;
use latte::report_compiler_error;

fn main() {
    println!("[LATTE]: Running compiler...");
    let args = latte::LatteArgs::parse();
    if let Err(err) = run_compiler(compile_llvm, &args) {
        let args = latte::LatteArgs::parse();
        let source_code = std::fs::read_to_string(&args.input_file)
            .unwrap_or_else(|_| String::from("// Source file not available"));
        let file_path = args.input_file.to_str().unwrap();
        report_compiler_error(file_path, &source_code, &err);

        eprintln!("ERROR");
        std::process::exit(1);
    }
    eprintln!("OK");
}
