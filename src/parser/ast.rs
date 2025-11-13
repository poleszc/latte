/// Main file containing the AST definition.
use std::fmt::Debug;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SourcePosition(pub usize, pub usize);
impl From<(usize, usize)> for SourcePosition {
    fn from(value: (usize, usize)) -> Self {
        SourcePosition(value.0, value.1)
    }
}

#[derive(Debug, PartialEq)]
pub struct Program(pub Vec<Stmt>);

#[derive(Debug, PartialEq, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StmtKind {
    Assignment { name: String, value: Box<Expr> },
    Expression { value: Box<Expr> },
}

#[derive(Debug, PartialEq, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub position: SourcePosition,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExprKind {
    Literal(i32),
    Identifier(String),
    BinaryOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        weight: Option<u64>,
    },
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// =============== Weight calculation ===================
/// This part implements the preprocessing of the AST. For each node in the AST, 
/// it calculates the height of it's subtree (including the node itself). This 
/// data is then used for optimizations in Jasmine code generation.
impl Program {
    pub fn calculate_weights(&mut self) {
        for stmt in &mut self.0 {
            stmt.calculate_weights();
        }
    }
}

impl Stmt {
    fn calculate_weights(&mut self) {
        match &mut self.kind {
            StmtKind::Assignment { value, .. } => {
                value.calculate_weights();
            }
            StmtKind::Expression { value } => {
                value.calculate_weights();
            }
        }
    }
}

impl Expr {
    pub fn calculate_weights(&mut self) -> u64 {
        match &mut self.kind {
            ExprKind::Literal(_) => 1,
            ExprKind::Identifier(_) => 1,
            ExprKind::BinaryOp {
                lhs, rhs, weight, ..
            } => {
                let lhs_weight = lhs.calculate_weights();
                let rhs_weight = rhs.calculate_weights();

                let current_weight = if lhs_weight == rhs_weight {
                    lhs_weight + 1
                } else {
                    lhs_weight.max(rhs_weight)
                };

                *weight = Some(current_weight);

                current_weight
            }
        }
    }

    pub fn get_weight(&self) -> Option<u64> {
        match &self.kind {
            ExprKind::BinaryOp { weight, .. } => *weight,
            _ => Some(1),
        }
    }
}
