mod common;
use common::*;

// #[test]
// fn big_tests() -> io::Result<()> {
//     let test_dir = Path::new("tests").join("in");
//     for entry in fs::read_dir(&test_dir)? {
//         let entry = entry?;
//         let path = entry.path();

//         if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("ins") {
//             let file_stem = path.file_stem().unwrap().to_str().unwrap();

//             // Integration tests.
//             if !file_stem.starts_with("bad_") && !file_stem.starts_with("error_") {
//                 let out_path = Path::new("tests")
//                     .join("out")
//                     .join(format!("{}.output", file_stem));


//                 if out_path.exists() {
//                     run_itest(path.to_str().unwrap(), out_path.to_str().unwrap(), Backend::Jvm);
//                     run_itest(path.to_str().unwrap(), out_path.to_str().unwrap(), Backend::Llvm);
//                 }
//             }
//         }
//     }
//     Ok(())
// }

include!(concat!(env!("OUT_DIR"), "/basic_parsing_tests.rs"));
include!(concat!(env!("OUT_DIR"), "/extensions_parsing_tests.rs"));
// include!(concat!(env!("OUT_DIR"), "/semantic_tests.rs"));
// include!(concat!(env!("OUT_DIR"), "/integration_tests.rs"));