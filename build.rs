use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=src/latte.lalrpop");
    lalrpop::process_root().unwrap();
    // TODO: Macro?

    // ===================== LEXER ======================
    generate_test_suite(
        "lexer_tests.rs",
        &["tests/bad/lexical"],
        "assert_parsing_fails",
        "lexer_fail",
    )
    .expect("Generate lexer tests failed");

    // ===================== PARSING ======================
    generate_test_suite(
        "basic_parsing_tests.rs",
        &["tests/good/official"],
        "assert_parsing_succeeds",
        "parsing_pass",
    )
    .expect("Generate basic parsing tests failed");

    generate_test_suite(
        "extensions_parsing_tests.rs",
        &["tests/good/extensions", "tests/good/extra"],
        "assert_parsing_succeeds",
        "parsing_pass",
    )
    .expect("Generate extensions parsing tests failed");

    // ===================== SEMANTIC ======================
    generate_test_suite(
        "basic_semantics_tests_good.rs",
        &["tests/bad/official"],
        "assert_semantic_analysis_fail",
        "semantic_fail",
    )
    .expect("Generate basic semantic fail tests failed");

    generate_test_suite(
        "basic_semantics_tests_bad.rs",
        &["tests/good/official"],
        "assert_semantic_analysis_pass",
        "semantic_pass",
    )
    .expect("Generate basic semantic pass tests failed");

    generate_test_suite(
        "extensions_semantics_tests_bad.rs",
        &[
            "tests/bad/semantic/basic",
            "tests/bad/semantic/extensions",
            "tests/bad/semantic/error_recovery",
        ],
        "assert_semantic_analysis_fail",
        "semantic_fail",
    )
    .expect("Generate extensions semantic fail tests failed");

    generate_test_suite(
        "extensions_semantics_tests_good.rs",
        &["tests/good/extra", "tests/good/extensions"],
        "assert_semantic_analysis_pass",
        "semantic_pass",
    )
    .expect("Generate extensions semantic pass tests failed");

    // ===================== MIR ======================
    generate_test_suite(
        "basic_mir_lowering_tests.rs",
        &["tests/good/official"],
        "assert_mir_lowering_pass",
        "mir_pass",
    )
    .expect("Generate basic MIR tests failed");

    generate_test_suite(
        "extra_mir_lowering_tests.rs",
        &["tests/good/extra"],
        "assert_mir_lowering_pass",
        "mir_pass",
    )
    .expect("Generate extra MIR tests failed");

    generate_test_suite(
        "extensions_mir_lowering_tests.rs",
        &["tests/good/extensions"],
        "assert_mir_lowering_pass",
        "mir_pass",
    )
    .expect("Generate extensions MIR tests failed");

    // ===================== LIR ======================
    generate_test_suite(
        "basic_lir_lowering_tests.rs",
        &["tests/good/official"],
        "assert_lir_lowering_pass",
        "lir_pass",
    )
    .expect("Generate basic LIR tests failed");

    generate_test_suite(
        "extra_lir_lowering_tests.rs",
        &["tests/good/extra"],
        "assert_lir_lowering_pass",
        "lir_pass",
    )
    .expect("Generate extra LIR tests failed");

    generate_test_suite(
        "extensions_lir_lowering_tests.rs",
        &["tests/good/extensions"],
        "assert_lir_lowering_pass",
        "lir_pass",
    )
    .expect("Generate extensions LIR tests failed");

    // ===================== LLVM ======================
    generate_test_suite(
        "basic_llvm_lowering_tests.rs",
        &["tests/good/official"],
        "assert_llvm_lowering_pass",
        "llvm_pass",
    )
    .expect("Generate basic LLVM tests failed");

    generate_test_suite(
        "extra_llvm_lowering_tests.rs",
        &["tests/good/extra"],
        "assert_llvm_lowering_pass",
        "llvm_pass",
    )
    .expect("Generate extra LLVM tests failed");

    generate_test_suite(
        "extensions_llvm_lowering_tests.rs",
        &[
            "tests/good/extensions/arrays",
            "tests/good/extensions/struct",
            // "tests/good/extensions",
        ],
        "assert_llvm_lowering_pass",
        "llvm_pass",
    )
    .expect("Generate extensions LLVM tests failed");

    // =============== INTEGRATION TESTS ================
    generate_test_suite(
        "integration_compilation_fail.rs",
        &[
            "tests/bad/official",
            "tests/bad/semantic",
            "tests/bad/lexical",
        ],
        "assert_compilation_fails",
        "compilation_fail",
    )
    .expect("Failed to generate compilation fail tests");

    generate_test_suite(
        "integration_runtime_fail.rs",
        &["tests/bad/runtime"],
        "assert_runtime_error",
        "runtime_fail",
    )
    .expect("Failed to generate runtime error tests");

    generate_test_suite(
        "integration_good_tests.rs",
        &[
            "tests/good/official",
            "tests/good/extra",
            "tests/good/extensions/arrays",
            "tests/good/extensions/struct",
            // "tests/good/extensions",
        ],
        "run_integration_test",
        "integration_pass",
    )
    .expect("Failed to generate integration tests for good cases");
}

/// Recursively collects all files starting from a giver directory.
fn find_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                find_files_recursive(&path, files)?;
            } else if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("lat")
            {
                files.push(path);
            }
        }
    }
    Ok(())
}

/// Generates test for the given directory and applies the `test_function` to the file contents.
fn generate_test_suite(
    output_filename: &str,
    input_dirs: &[&str],
    test_function: &str,
    test_prefix: &str,
) -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR env variable not set");
    let dest_path = Path::new(&out_dir).join(output_filename);

    let mut test_file = fs::File::create(dest_path)?;
    writeln!(&mut test_file, "/* AUTOGENERATED TESTS */\n")?;

    for dir_str in input_dirs {
        let dir_path = Path::new(dir_str);

        println!("cargo:rerun-if-changed={}", dir_str);

        let mut source_files = Vec::new();
        find_files_recursive(dir_path, &mut source_files)?;

        source_files.sort();

        for path in source_files {
            let file_stem = path.file_stem().unwrap().to_str().unwrap();

            let parent = path.parent().unwrap();
            let parent_str = parent.to_string_lossy();
            let sane_parent = parent_str.replace(|c: char| !c.is_alphanumeric(), "_");
            let sane_stem = file_stem.replace(|c: char| !c.is_alphanumeric(), "_");

            let test_name = format!("{}_{}_{}", test_prefix, sane_parent, sane_stem)
                .replace("__", "_")
                .trim_matches('_')
                .to_string();

            writeln!(
                &mut test_file,
                "#[test]\nfn {}() {{\n    {}(r\"{}\");\n}}\n",
                test_name,
                test_function,
                path.display(),
            )?;
        }
    }
    Ok(())
}
