use std::{fs, path::Path};

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
    backend_compile: F,
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

    dbg!(parsed_program);


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
