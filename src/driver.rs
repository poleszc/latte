use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;

use crate::{
    LatteArgs,
    error::CompilerError,
    parser::{self, ast},
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
    let mut parsed_program = parser::parse(&source_code)?;

    let mut json_output_path = PathBuf::from(&output_dir);
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
    json_output_path.push(format!("{}.ast.json", file_stem));

    let json_string = serde_json::to_string_pretty(&parsed_program)
        .map_err(|e| CompilerError::Other(format!("Failed to serialize AST to JSON: {}", e)))?;

    fs::write(&json_output_path, json_string).map_err(|e| {
        CompilerError::Other(format!(
            "Failed to write AST to file '{}': {}",
            json_output_path.display(),
            e
        ))
    })?;

    println!(
        "Successfully parsed and saved AST to {}",
        json_output_path.display()
    );

    let file_stem = args
        .output
        .as_deref()
        .or_else(|| args.input_file.file_stem().and_then(|s| s.to_str()))
        .ok_or_else(|| {
            CompilerError::Other(format!(
                "Could not determine output file name from: {}",
                input_file.display()
            ))
        })?;

    // backend_compile(&mut parsed_program, &output_dir, file_stem, &args)?;

    Ok(())
}
