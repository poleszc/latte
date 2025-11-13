mod driver;
mod instructions;
mod lowering;

use lowering::LLVMGenerator;
use instructions::{LLVMInstruction, PointerName, RegisterName};

pub use driver::compile_llvm;
