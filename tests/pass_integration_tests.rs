#![allow(non_snake_case)]
#![allow(dead_code)]
mod common;
use common::*;

// include!(concat!(env!("OUT_DIR"), "/integration_good_tests.rs"));

#[test]
fn integration_pass_tests_good_extensions_arrays_simple_arrays() {
    run_integration_test(r"tests/good/extensions/arrays/simple_arrays.lat");
}
