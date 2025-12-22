use latte::{
    CompilerError, lir,
    llvm::LLVMGenerator,
    mir::{self},
    parser,
    semantics::SemanticChecker,
};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
use tempfile::tempdir;

pub fn assert_parsing_succeeds(input_path: &str) {
    let source_code = std::fs::read_to_string(input_path)
        .unwrap_or_else(|err| panic!("Failed to read a test file '{}': {}", input_path, err));

    let result = parser::parse(&source_code);
    assert!(
        result.is_ok(),
        "Expected parsing to succeed for '{}', but it failed with: {:?}",
        input_path,
        result.err()
    );
}

pub fn assert_parsing_fails(input_path: &str) {
    let source_code = std::fs::read_to_string(input_path)
        .unwrap_or_else(|err| panic!("Failed to read a test file '{}': {}", input_path, err));

    let result = parser::parse(&source_code);
    assert!(
        matches!(result, Err(CompilerError::Parsing(_))),
        "Expected parsing to fail for '{}', but it succeeded",
        input_path
    );
}

pub fn assert_semantic_analysis_pass(input_path: &str) {
    let source_code = std::fs::read_to_string(input_path)
        .unwrap_or_else(|err| panic!("Failed to read a test file '{}': {}", input_path, err));

    let ast = parser::parse(&source_code).expect("Expected parsing to succeed, but it failed");
    match SemanticChecker::check(&ast) {
        Ok(_) => return,
        Err(err) => assert!(
            err.len() == 0,
            "Expected semantic analysis to succeed for '{}' but it failed with: {:#?}",
            input_path,
            err
        ),
    }
}

pub fn assert_semantic_analysis_fail(input_path: &str) {
    let source_code = std::fs::read_to_string(input_path)
        .unwrap_or_else(|err| panic!("Failed to read a test file '{}': {}", input_path, err));

    let ast = match parser::parse(&source_code) {
        Ok(ast) => ast,
        Err(_) => return, // Test passes if parsing failed.
    };

    let result = SemanticChecker::check(&ast);
    assert!(
        result.is_err(),
        "Expected semantic analysis to fail for '{}', but it succeeded",
        input_path
    );
}

pub fn assert_semantic_analysis_fail_with_n_errors(input_path: &str, error_count: usize) {
    let source_code = std::fs::read_to_string(input_path)
        .unwrap_or_else(|err| panic!("Failed to read a test file '{}': {}", input_path, err));

    let ast = match parser::parse(&source_code) {
        Ok(ast) => ast,
        Err(_) => return, // Test passes if parsing failed.
    };

    let result = SemanticChecker::check(&ast);
    assert!(
        result.is_err(),
        "Expected semantic analysis to fail for '{}', but it succeeded",
        input_path
    );

    let got_errors = result.err().unwrap().len();
    assert!(
        got_errors == error_count,
        "Expected semantic analysis to fail with '{}' errors, but it returned '{}' errors",
        error_count,
        got_errors
    );
}

pub fn assert_mir_lowering_pass(input_path: &str) {
    let source_code = std::fs::read_to_string(input_path)
        .unwrap_or_else(|err| panic!("Failed to read a test file '{}': {}", input_path, err));

    let ast = parser::parse(&source_code).expect("Expected parsing to succeed, but it failed");
    let semantic_result =
        SemanticChecker::check(&ast).expect("Expected semantic analysis to succeed, but it failed");

    let _ = mir::lower_to_mir(&semantic_result.tast, semantic_result.symbol_table);
    // If we got here, it means mir lowering passed.
}

pub fn assert_lir_lowering_pass(input_path: &str) {
    let source_code = std::fs::read_to_string(input_path)
        .unwrap_or_else(|err| panic!("Failed to read a test file '{}': {}", input_path, err));

    let ast = parser::parse(&source_code).expect("Expected parsing to succeed, but it failed");
    let semantic_result =
        SemanticChecker::check(&ast).expect("Expected semantic analysis to succeed, but it failed");

    let mir_program = mir::lower_to_mir(&semantic_result.tast, semantic_result.symbol_table);
    let _ = lir::lower_to_lir(mir_program);
    // If we got here, it means the lir lowering passed.
}

pub fn assert_llvm_lowering_pass(input_path: &str) {
    let source_code = std::fs::read_to_string(input_path)
        .unwrap_or_else(|err| panic!("Failed to read a test file '{}': {}", input_path, err));

    let ast = parser::parse(&source_code).expect("Expected parsing to succeed, but it failed");
    let semantic_result =
        SemanticChecker::check(&ast).expect("Expected semantic analysis to succeed, but it failed");

    let mir_program = mir::lower_to_mir(&semantic_result.tast, semantic_result.symbol_table);
    let lir_program = lir::lower_to_lir(mir_program);
    let llvm_generator = LLVMGenerator::new();
    llvm_generator.generate_program(&lir_program);
    // If we got here, it means the llvm lowering passed.
}

/// Integration tests.
const LLVM_COMPILER_BIN: &str = env!("CARGO_BIN_EXE_latc_llvm");

pub fn assert_compilation_fails(input_path: &str) {
    let output = Command::new(LLVM_COMPILER_BIN)
        .arg(&input_path)
        .output()
        .expect("Failed to execute the compiler binary");
    assert!(
        !output.status.success(),
        "Expected LLVM compilation to fail for '{}', but if succeeded",
        input_path
    );
}

pub fn run_integration_test(input_path: &str) {
    let input_path_buf = PathBuf::from(input_path);
    let expected_output_path = input_path_buf.with_extension("output");
    let input_data_path = input_path_buf.with_extension("input");

    if !expected_output_path.exists() {
        panic!("Missing .output file for: {}", input_path)
    }

    let temp_dir = tempdir().expect("Failed to create a temp directory for tests");
    let expected_output = fs::read_to_string(&expected_output_path).unwrap_or_else(|err| {
        panic!(
            "Failed to read a file '{}': {}",
            expected_output_path.display(),
            err
        )
    });

    let mut command = Command::new(LLVM_COMPILER_BIN);
    command
        .arg(input_path)
        .arg("--run")
        .arg("--output-dir")
        .arg(temp_dir.path());

    if input_data_path.exists() {
        let input_data = fs::read_to_string(input_data_path).unwrap();
        command.stdin(Stdio::piped());
        let mut child = command.spawn().expect("Failed to spawn compiler process");
        let mut stdin = child.stdin.take().expect("Failed to open stdin");

        std::thread::spawn(move || {
            stdin
                .write_all(input_data.as_bytes())
                .expect("Failed to write to stdin");
        });

        let output = child.wait_with_output().expect("Failed to read stdout");
        assert!(
            output.status.success(),
            "Expected compilation to succeed for '{}', but it failed.\n[STDOUT]:\n{}\n[STDERR]:\n{}",
            input_path,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let actual_output = String::from_utf8_lossy(&output.stdout);

        assert_eq!(
            actual_output.trim(),
            expected_output.trim(),
            "Actual output differs from the expected one for test '{}'",
            input_path
        );
    } else {
        let output = command.output().expect("Failed to execute compiler binary");
        assert!(
            output.status.success(),
            "Expected compilation to succeed for '{}', but it failed.\n[STDOUT]:\n{}\n[STDERR]:\n{}",
            input_path,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let actual_output = String::from_utf8_lossy(&output.stdout);

        assert_eq!(
            actual_output.trim(),
            expected_output.trim(),
            "Actual output differs from the expected one for test '{}'",
            input_path
        );
    }
}

pub fn assert_runtime_error(input_path: &str) {
    let temp_dir = tempdir().expect("Failed to create a temp directory for tests");

    let output = Command::new(LLVM_COMPILER_BIN)
        .arg(input_path)
        .arg("--run")
        .arg("--output-dir")
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute the compiler binary");

    assert!(
        !output.status.success(),
        "Expected program execution to fail for '{}', but it succeeded",
        input_path
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("runtime error"),
        "Expected 'runtime error' message on stderr for '{}', but got: {}",
        input_path,
        stderr
    );
}
