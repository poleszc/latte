use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;

use crate::{
    LatteArgs,
    errors::CompilerError,
    mir::{self, MIRProgram, lowering::MIRLoweringContext},
    parser::{
        self,
        ast::{self, Program},
    },
    report_compiler_error,
    semantics::SemanticChecker,
};

/// Main driver for the compiler. Performs all the generic operations for both backends.
/// - Resolves the output directories
/// - Performs the parsing
/// - Runs the compilation function(`backend_compile`).
pub fn run_compiler<
    F: FnOnce(&mut ast::Program, &Path, &str, &LatteArgs) -> Result<(), CompilerError>,
>(
    _backend_compile: F,
) -> Result<(), CompilerError> {
    // TODO: Create a CONFIG structure instead of this multiple arg parsing.
    let args = LatteArgs::parse();

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
    eprintln!("[Parsing] Parsing program...");
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
    eprintln!("[Parsing] Parsing successful...");
    dump_ast_to_json(&output_dir, file_stem, &parsed_program)?;
    eprintln!("[Parsing] Dumped AST to JSON...");

    // ============= SEMANTIC ANALYSIS ===============
    eprintln!("[Semantic] Running semantic analysis...");
    let symbol_table = match SemanticChecker::check(&parsed_program) {
        Ok(table) => table,
        Err(errors) => {
            for err in errors {
                let comp_error = CompilerError::Semantic(err);
                report_compiler_error(args.input_file.to_str().unwrap(), &source_code, &comp_error);
            }
            // TODO: remove kajak...
            return Err(CompilerError::Other("Kajak".to_string()));
        }
    };

    eprintln!("[Semantic] Semantic analysis completed successfully");

    // ============= MIR LOWERING ===============
    eprintln!("[MIR] Lowering to MIR...");
    let mut lowering_context = MIRLoweringContext::new(symbol_table);
    let mir_program = lowering_context.lower_program(&parsed_program);
    eprintln!("[MIR] Lowering to MIR completed successfully...");
    dump_mir(&output_dir, file_stem, &mir_program)?;
    eprintln!("[MIR] Dumped MIR to JSON...");

    // TODO:
    // - MIR Opts
    // - LIR lowering (SSA)
    // - LLVM lowering (SSA)

    Ok(())
}

/// Dumps the AST to a JSON file in a specified output_dir and a given file name.
fn dump_ast_to_json(
    output_dir: &PathBuf,
    file_stem: &str,
    program: &Program,
) -> Result<(), CompilerError> {
    println!("[LATTE]: Dumping AST to file");
    let mut json_output_path = PathBuf::from(&output_dir);
    json_output_path.push(format!("{}.ast.json", file_stem));

    let json_string = serde_json::to_string_pretty(&program)
        .map_err(|e| CompilerError::Other(format!("Failed to serialize AST to JSON: {}", e)))?;

    fs::write(&json_output_path, json_string).map_err(|e| {
        CompilerError::Other(format!(
            "Failed to write AST to file '{}': {}",
            json_output_path.display(),
            e
        ))
    })?;

    println!(
        "[LATTE]: Successfully parsed and saved AST to {}",
        json_output_path.display()
    );
    Ok(())
}

/// Dumps the MIR to a JSON file and a txt file in a specified output_dir and a given file name.
fn dump_mir(
    output_dir: &PathBuf,
    file_stem: &str,
    mir_program: &MIRProgram,
) -> Result<(), CompilerError> {
    println!("[MIR]: Dumping MIR to file");
    let mut json_output_path = PathBuf::from(&output_dir);
    json_output_path.push(format!("{}.mir.json", file_stem));

    let json_string = serde_json::to_string_pretty(&mir_program)
        .map_err(|e| CompilerError::Other(format!("Failed to serialize MIR to JSON: {}", e)))?;

    fs::write(&json_output_path, json_string).map_err(|e| {
        CompilerError::Other(format!(
            "Failed to write MIR to file '{}': {}",
            json_output_path.display(),
            e
        ))
    })?;

    mir_program.dump_to_file(
        PathBuf::from(&output_dir)
            .join(format!("{}.mir.txt", file_stem))
            .as_path(),
    )?;

    println!(
        "[LATTE]: Successfully saved MIR to {}",
        json_output_path.display()
    );
    Ok(())
}
