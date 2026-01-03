/// Branch folding:
/// If a branch instruction depending on a constant value exists in
/// the program, we change it to a jump.
use crate::{lir::*, opts::OptimizationPass};

pub struct DeadCodeElimination;
