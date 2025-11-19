use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;

use crate::{
    LatteArgs,
    diagnostics::CompilerError,
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
    dump_ast_to_json(&output_dir, file_stem, &parsed_program)?;

    eprintln!("[Semantic] Running semantic analysis...");
    match SemanticChecker::check(&parsed_program) {
        Ok(_) => return Ok(()),
        Err(errors) => {
            for err in errors  {
                let comp_error = CompilerError::Semantic(err);
                report_compiler_error(&args.input_file.to_str().unwrap(), &source_code, &comp_error);
            }
            return Err(CompilerError::Other("Kajak".to_string()));
        }
    }
    


    eprintln!("[Semantic] Semantic analysis completed successful");

    // backend_compile(&mut parsed_program, &output_dir, file_stem, &args)?;

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
