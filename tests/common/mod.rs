use latte::{CompilerError, parser, semantics::SemanticChecker};

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

// pub fn assert_compilation_fails(input_path: &str) {
//     let insc_jvm = env!("CARGO_BIN_EXE_insc_jvm");
//     let insc_llvm = env!("CARGO_BIN_EXE_insc_llvm");

//     let output = Command::new(insc_jvm)
//         .arg(input_path)
//         .output()
//         .expect("Failed to execute the JVM compiler binary");

//     assert!(
//         !output.status.success(),
//         "Expected JVM compilation to fail for '{}', but it succeeded",
//         input_path
//     );

//     let output = Command::new(insc_llvm)
//         .arg(input_path)
//         .output()
//         .expect("Failed to execute the LLVM compiler binary");

//     assert!(
//         !output.status.success(),
//         "Expected LLVM compilation to fail for '{}', but it succeeded",
//         input_path
//     );
// }

// pub fn run_itest(input_path: &str, expected_output_path: &str, backend: Backend) {
//     let bin_name = match backend {
//         Backend::Jvm => env!("CARGO_BIN_EXE_insc_jvm"),
//         Backend::Llvm => env!("CARGO_BIN_EXE_insc_llvm"),
//     };
//     let temp_dir = tempdir().expect("Failed to create a temp directory for tests");
//     let expected_output = fs::read_to_string(expected_output_path)
//         .unwrap_or_else(|err| panic!("Failed to read a file '{}': {}", expected_output_path, err));

//     let output = Command::new(bin_name)
//         .arg(input_path)
//         .arg("--run")
//         .arg("--output-dir")
//         .arg(temp_dir.path())
//         .output()
//         .expect("Failed to execute the LLVM compiler binary");

//     assert!(
//         output.status.success(),
//         "Expected compilation to succeed for '{}', but it failed.\n[STDOUT]:\n{}\n[STDERR]:\n{}",
//         input_path,
//         String::from_utf8_lossy(&output.stdout),
//         String::from_utf8_lossy(&output.stderr)
//     );

//     let actual_output = String::from_utf8_lossy(&output.stdout);

//     assert_eq!(
//         actual_output, expected_output,
//         "Actual output differs from the expected one"
//     );
// }
