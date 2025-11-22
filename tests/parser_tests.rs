mod common;
use common::*;

include!(concat!(env!("OUT_DIR"), "/basic_parsing_tests.rs"));
include!(concat!(env!("OUT_DIR"), "/extensions_parsing_tests.rs"));
