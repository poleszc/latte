use crate::mir::MIRProgram;

mod builders;
mod lir_display;
pub mod lir_structure;
mod lowering;
pub use lir_structure::*;

pub fn lower_to_lir(_program: &MIRProgram) -> LIRProgram {
    todo!();
}
