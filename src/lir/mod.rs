use crate::{lir::lowering::LIRLowering, mir::MIRProgram};

// mod builders;
mod lir_display;
pub mod lir_structure;
mod lowering;
pub use lir_structure::*;

pub fn lower_to_lir(mir_program: MIRProgram) -> LIRProgram {
    LIRLowering::lower(mir_program)
}
