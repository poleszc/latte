use crate::semantics::{symbol_table::SymbolTable, tast::TProgram};

mod builders;
mod lowering;
mod mir_display;
pub mod mir_structure;
pub use mir_structure::*;

/// Main function performing the lowering from TAST to MIR.
/// Initializes the context and performs the lowering.
pub fn lower_to_mir(program: &TProgram, symbol_table: SymbolTable) -> MIRProgram {
    let mut mir_lowering_context = lowering::MIRLoweringContext::new(symbol_table);
    mir_lowering_context.lower_program(program)
}
