use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    LatteArgs,
    errors::CompilerError,
    lir::{self, *},
    mir::{self, *},
    opts::optimize_program,
    parser::{self, ast::Program},
    report_compiler_error,
    semantics::{SemanticChecker, tast::TProgram},
};

/// Main driver for the compiler.
/// - Resolves the output directories
/// - Performs the parsing
/// - Runs the compilation function(`backend_compile`).
pub fn run_compiler<
    F: FnOnce(&LIRProgram, &Path, &str, &LatteArgs) -> Result<(), CompilerError>,
>(
    backend_driver: F,
    args: &LatteArgs,
) -> Result<(), CompilerError> {
    let output_dir = match args.output_dir {
        Some(ref path) => path.clone(),
        None => args
            .input_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
    };

    fs::create_dir_all(&output_dir)?;

    // ============= PARSING ===============
    println!("[LATTE]: Parsing program...");
    let input_file = &args.input_file;
    let source_code = fs::read_to_string(input_file)?;
    let parsed_program = parser::parse(&source_code)?;

    let file_stem = args
        .input_file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            CompilerError::Other(format!(
                "Could not determine file stem from: {}",
                input_file.display()
            ))
        })?;

    if args.emit_ast || args.emit_all {
        dump_ast_to_json(&output_dir, file_stem, &parsed_program)?;
    }

    // ============= SEMANTIC ANALYSIS ===============
    println!("[LATTE]: Running semantic analysis...");
    let semantic_analysis_result = match SemanticChecker::check(&parsed_program) {
        Ok(table) => table,
        Err(errors) => {
            eprintln!("ERROR\n");
            for err in errors {
                let comp_error = CompilerError::Semantic(err);
                report_compiler_error(args.input_file.to_str().unwrap(), &source_code, &comp_error);
            }
            return Err(CompilerError::Silent);
        }
    };

    if args.emit_tast || args.emit_all {
        dump_tast_to_json(&output_dir, file_stem, &semantic_analysis_result.tast)?;
    }

    // ============= MIR LOWERING ===============
    println!("[LATTE]: Lowering to MIR...");
    let mir_program = mir::lower_to_mir(
        &semantic_analysis_result.tast,
        semantic_analysis_result.symbol_table,
        args.opt_inline || args.optimize,
        args.verbose_opts,
    );
    if args.emit_mir || args.emit_all {
        dump_mir(&output_dir, file_stem, &mir_program)?;
    }

    // ============= LIR LOWERING ===============
    println!("[LATTE]: Lowering to LIR...");
    let mut lir_program = lir::lower_to_lir(mir_program);
    if args.emit_lir || args.emit_all {
        dump_lir(&output_dir, file_stem, &lir_program, false)?;
    }

    // ============= LIR OPTIMIZATIONS ===============
    if are_opts_on(args) {
        println!("[LATTE]: Optimizing LIR...");
        optimize_program(&mut lir_program, args);
        if args.emit_lir_opt || args.emit_all {
            dump_lir(&output_dir, file_stem, &lir_program, true)?;
        }
    }

    // ============= LLVM BACKEND ===============
    println!("[LATTE]: Generating LLVM...");
    backend_driver(&lir_program, &output_dir, file_stem, args)?;

    Ok(())
}

fn are_opts_on(args: &LatteArgs) -> bool {
    args.opt_branch_fold || args.opt_const_prop || args.opt_dbe || args.opt_dce || args.optimize
}

/// Dumps the AST to a JSON file in a specified output_dir and a given file name.
fn dump_ast_to_json(
    output_dir: &PathBuf,
    file_stem: &str,
    program: &Program,
) -> Result<(), CompilerError> {
    println!("[LATTE]: Dumping AST to file");
    program.dump_to_file(
        PathBuf::from(&output_dir)
            .join(format!("{}.ast.json", file_stem))
            .as_path(),
    )?;
    Ok(())
}

/// Dumps the TAST to a JSON file in a specified output_dir and a given file name.
fn dump_tast_to_json(
    output_dir: &PathBuf,
    file_stem: &str,
    program: &TProgram,
) -> Result<(), CompilerError> {
    println!("[LATTE]: Dumping TAST to file");
    program.dump_to_file(
        PathBuf::from(&output_dir)
            .join(format!("{}.tast.json", file_stem))
            .as_path(),
    )?;

    Ok(())
}

/// Dumps the MIR to a JSON file and a txt file in a specified output_dir and a given file name.
fn dump_mir(
    output_dir: &PathBuf,
    file_stem: &str,
    mir_program: &MIRProgram,
) -> Result<(), CompilerError> {
    println!("[MIR]: Dumping MIR to file");
    mir_program.dump_to_file(
        PathBuf::from(&output_dir)
            .join(format!("{}.mir", file_stem))
            .as_path(),
    )?;

    Ok(())
}

/// Dumps the LIR to a JSON file and a txt file in a specified output_dir and a given file name.
fn dump_lir(
    output_dir: &PathBuf,
    file_stem: &str,
    lir_program: &LIRProgram,
    optimized: bool,
) -> Result<(), CompilerError> {
    println!("[LIR]: Dumping LIR to file");
    let file_name = if optimized {
        format!("{}.opt.lir", file_stem)
    } else {
        format!("{}.lir", file_stem)
    };

    lir_program.dump_to_file(PathBuf::from(&output_dir).join(file_name).as_path())?;
    Ok(())
}
