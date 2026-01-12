use crate::{lir::lowering::LIRLowering, mir::MIRProgram};

mod lir_display;
pub mod lir_structure;
pub mod lowering;
pub use lir_structure::*;

pub fn lower_to_lir(mir_program: MIRProgram) -> LIRProgram {
    LIRLowering::lower(mir_program)
}
