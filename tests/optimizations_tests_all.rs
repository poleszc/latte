#![allow(non_snake_case)]
#![allow(dead_code)]
mod common;
use common::*;

// include!(concat!(env!("OUT_DIR"), "/integration_opts_all.rs"));

#[test]
fn integration_pass_all_opts_tests_good_official_core018() {
    run_integration_test_with_all_opts(r"tests/good/official/core018.lat");
}
