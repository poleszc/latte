mod common;
use common::*;

include!(concat!(env!("OUT_DIR"), "/basic_semantics_tests.rs"));
include!(concat!(env!("OUT_DIR"), "/extensions_semantics_tests.rs"));
