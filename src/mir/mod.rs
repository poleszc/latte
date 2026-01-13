use crate::{
    mir::opts::inline_functions,
    semantics::{symbol_table::SymbolTable, tast::TProgram},
};

pub mod builders;
mod lowering;
mod mir_display;
pub mod mir_structure;
mod opts;
pub use mir_structure::*;

/// Main function performing the lowering from TAST to MIR.
/// Initializes the context and performs the lowering.
pub fn lower_to_mir(
    program: &TProgram,
    symbol_table: SymbolTable,
    inline: bool,
    verbose: bool,
) -> MIRProgram {
    let mut mir_lowering_context = lowering::MIRLoweringContext::new(symbol_table);
    let mut mir_program = mir_lowering_context.lower_program(program);

    if inline {
        if verbose {
            println!("++++++++++++++++++++++++++++++++++++++");
            mir_program.dump_to_stderr();
            println!("++++++++++++++++++++++++++++++++++++++");
        }
        inline_functions(&mut mir_program);
        if verbose {
            println!("++++++++++++++++++++++++++++++++++++++");
            mir_program.dump_to_stderr();
            println!("++++++++++++++++++++++++++++++++++++++");
        }
    }
    mir_program
}
