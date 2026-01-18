use std::collections::HashMap;

use crate::{
    SourcePosition,
    parser::ast::Type,
    utils::{StrID, cfg::CFG},
};

/// Unique identifier of a single block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockID(pub usize);
impl BlockID {
    pub fn new(id: usize) -> Self {
        BlockID(id)
    }
}

/// Unique identifier of a single local variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalID(pub usize);
impl LocalID {
    pub fn new(id: usize) -> Self {
        LocalID(id)
    }
}

/// Unique identifier of a temporary local variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempID(pub usize);
impl TempID {
    pub fn new(id: usize) -> Self {
        TempID(id)
    }
}

/// All MIR operations.
#[derive(Debug, Clone, PartialEq)]
pub enum MIROperation {
    Assign,
    Call,
    VCall,
    DropString,

    // Arithmetic
    IntAdd,
    IntSub,
    IntMul,
    IntDiv,
    IntMod,
    IntNeg,

    // Comparisons
    IntLt,
    IntLe,
    IntGt,
    IntGe,
    IntEq,
    IntNeq,

    // Logical
    BoolAnd,
    BoolOr,
    BoolNot,

    // String
    StringConcat,
    StringEq,
    StringNeq,
    StringLength,

    // Array
    NewArray, // new arr[10]
    ArrayGet, // arr[ix]
    ArraySet, // arr[ix] = ...
    ArrayLength,

    NewObject, // new Obj
    FieldGet,  // obj.field
    FieldSet,  // obj.field = ...
    IsNull,
}

/// Representation of a MIR value.
#[derive(Debug, Clone, PartialEq)]
pub enum MIRValue {
    Unit, // void
    IntConst(i32),
    BoolConst(bool),
    StringConst(String),
    Null,
    Local(LocalID), // Local variable reference
    Temp(TempID),   // temporary variable reference
    Block(BlockID), // Block reference
    Function(StrID),
    Class(StrID),
}

/// Representation of a MIR place <-> lvalue.
#[derive(Debug, Clone, PartialEq)]
pub enum MIRPlace {
    Local(LocalID),
    Temp(TempID),
    Field {
        base: Box<MIRPlace>,
        class_name: StrID,
        field_name: StrID,
        field_index: usize,
    },
    Index {
        base: Box<MIRPlace>,
        index: Box<MIRValue>,
    },
}

impl MIRPlace {
    pub fn is_simple(&self) -> bool {
        matches!(self, MIRPlace::Local(_) | MIRPlace::Temp(_))
    }

    pub fn base_place(&self) -> &MIRPlace {
        match self {
            MIRPlace::Field { base, .. } => base.base_place(),
            MIRPlace::Index { base, .. } => base.base_place(),
            simple => simple,
        }
    }
}

// Single MIR instruction.
#[derive(Debug, Clone)]
pub struct MIRInstruction {
    pub operation: MIROperation,
    pub destination: Option<MIRPlace>, // Some ops don't return a value.
    pub operands: Vec<MIRValue>,
    pub source_position: SourcePosition,
}

/// Terminator instruction - ends the block.
#[derive(Debug, Clone)]
pub enum MIRTerminator {
    Jump {
        target: BlockID,
        position: SourcePosition,
    },
    Branch {
        condition: MIRValue,
        if_true: BlockID,
        if_false: BlockID,
        position: SourcePosition,
    },
    Return {
        value: Option<MIRValue>,
        position: SourcePosition,
    },
    Unreachable {
        position: SourcePosition,
    },
}

/// Single MIR block in the CFG.
#[derive(Debug, Clone)]
pub struct MIRBlock {
    pub id: BlockID,
    /// List of instructions with no terminator.
    pub instructions: Vec<MIRInstruction>,
    /// The block terminator instruction.
    pub terminator: Option<MIRTerminator>,
}

/// Single local variable information.
#[derive(Debug, Clone)]
pub struct LocalInfo {
    pub id: LocalID,
    /// Local variable name - for debugging
    pub name: Option<StrID>,
    /// Local variable type
    pub typ: Type,
    /// Is this local a function parameter.
    pub is_parameter: bool,
    /// Parameter index, if the local is a parameter.
    pub parameter_index: Option<usize>,
}

/// Single temporary variable information.
#[derive(Debug, Clone)]
pub struct TempInfo {
    pub id: TempID,
    /// Temporary variable type
    pub typ: Type,
}

/// Single function representation in MIR.
#[derive(Debug, Clone)]
pub struct MIRFunction {
    /// Function name.
    pub name: StrID,
    /// Function return type.
    pub return_type: Type,
    /// Parameter types.
    pub parameter_types: Vec<Type>,
    /// Local variables.
    pub locals: Vec<LocalInfo>,
    /// Temporary variables.
    pub temps: Vec<TempInfo>,
    /// Blocks.
    pub blocks: HashMap<BlockID, MIRBlock>,
    /// Block order.
    pub block_order: Vec<BlockID>,
    /// Entry block index.
    pub entry_block: BlockID,
    /// Position in the source code.
    pub source_position: SourcePosition,
}

impl CFG for MIRFunction {
    type BID = BlockID;

    fn entry_block(&self) -> Self::BID {
        self.entry_block
    }

    fn block_order(&self) -> Vec<Self::BID> {
        self.block_order.clone()
    }

    fn successors(&self, block_id: Self::BID) -> Vec<Self::BID> {
        let mut succs = Vec::new();
        if let Some(block) = self.blocks.get(&block_id)
            && let Some(term) = &block.terminator
        {
            match term {
                MIRTerminator::Jump { target, .. } => succs.push(*target),
                MIRTerminator::Branch {
                    if_true, if_false, ..
                } => {
                    succs.push(*if_true);
                    succs.push(*if_false);
                }
                _ => {}
            }
        }
        succs
    }
}

/// Single class information.
#[derive(Debug, Clone)]
pub struct MIRClass {
    /// Class name.
    pub name: StrID,
    /// Class fields in order.
    pub fields: Vec<MIRField>,
    /// Class method names in order. Names are after mangling.
    pub vtable: Vec<StrID>,
    pub source_position: SourcePosition,
}

/// Single class field information.
#[derive(Debug, Clone)]
pub struct MIRField {
    /// Field name.
    pub name: StrID,
    /// Field type.
    pub typ: Type,
    /// Field index.
    pub index: usize,
}

/// Represents a whole MIR program.
#[derive(Debug, Clone)]
pub struct MIRProgram {
    /// Top level functions in the program.
    pub functions: HashMap<StrID, MIRFunction>,
    /// All programs classes.
    pub classes: HashMap<StrID, MIRClass>,
    /// Order of functions in the program.
    pub function_order: Vec<StrID>,
}

impl MIRProgram {
    pub fn new() -> Self {
        MIRProgram {
            functions: HashMap::new(),
            classes: HashMap::new(),
            function_order: Vec::new(),
        }
    }

    pub fn add_function(&mut self, function: MIRFunction) {
        self.function_order.push(function.name);
        self.functions.insert(function.name, function);
    }

    pub fn add_class(&mut self, class: MIRClass) {
        self.classes.insert(class.name, class);
    }
}

impl Default for MIRProgram {
    fn default() -> Self {
        Self::new()
    }
}
