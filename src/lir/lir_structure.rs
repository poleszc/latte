use std::collections::HashMap;

use crate::{SourcePosition, parser::ast::Type, utils::StrID};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegisterID(pub usize);
impl RegisterID {
    pub fn new(id: usize) -> Self {
        RegisterID(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockID(pub usize);
impl BlockID {
    pub fn new(id: usize) -> Self {
        BlockID(id)
    }
}

#[derive(Debug, Clone)]
pub enum LIRValue {
    Register(RegisterID),
    IntConst(i32),
    BoolConst(bool),
    StringConst(String),
    Null,
    Function(StrID),
    Class(StrID),
    Block(BlockID),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LIROperation {
    Phi,
    Assign,

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

    Call,
    VCall,

    // Array
    NewArray, // new arr[10]
    ArrayGet, // arr[ix]
    ArraySet, // arr[ix] = ...
    ArrayLength,

    NewObject, // new Obj
    FieldGet,  // obj.field
    FieldSet,  // obj.field = ...
    IsNull,    // obj.field = ...
}

#[derive(Debug, Clone)]
pub enum LIRTerminator {
    Jump {
        target: BlockID,
        position: SourcePosition,
    },
    Branch {
        condition: LIRValue,
        if_true: BlockID,
        if_false: BlockID,
        position: SourcePosition,
    },
    Return {
        value: Option<LIRValue>,
        position: SourcePosition,
    },
    Unreachable {
        position: SourcePosition,
    },
}

#[derive(Debug, Clone)]
pub struct LIRInstruction {
    pub operation: LIROperation,
    pub destination: Option<RegisterID>,
    pub operands: Vec<LIRValue>,
    pub source_position: SourcePosition,
}

#[derive(Debug, Clone)]
pub struct RegisterInfo {
    pub id: RegisterID,
    pub typ: Type,
    pub name: Option<StrID>,
}

#[derive(Debug, Clone)]
pub struct LIRBlock {
    pub id: BlockID,
    pub instructions: Vec<LIRInstruction>,
    pub terminator: Option<LIRTerminator>,
    pub predecessors: Vec<BlockID>, // Direct predecessors
}

/// Single function representation in MIR.
#[derive(Debug, Clone)]
pub struct LIRFunction {
    pub name: StrID,
    pub return_type: Type,
    pub parameter_types: Vec<Type>,
    pub parameter_registers: Vec<RegisterID>,
    pub registers: Vec<RegisterInfo>,
    pub blocks: HashMap<BlockID, LIRBlock>,
    pub block_order: Vec<BlockID>,
    pub entry_block: BlockID,
    pub source_position: SourcePosition,
}

#[derive(Debug, Clone)]
pub struct LIRClass {
    pub name: StrID,
    pub fields: Vec<LIRField>,
    pub vtable: Vec<StrID>,
    pub source_position: SourcePosition,
}

#[derive(Debug, Clone)]
pub struct LIRField {
    pub name: StrID,
    pub typ: Type,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct LIRProgram {
    pub functions: HashMap<StrID, LIRFunction>,
    pub classes: HashMap<StrID, LIRClass>,
    pub function_order: Vec<StrID>,
}

impl LIRProgram {
    pub fn new() -> Self {
        LIRProgram {
            functions: HashMap::new(),
            classes: HashMap::new(),
            function_order: Vec::new(),
        }
    }

    pub fn add_function(&mut self, function: LIRFunction) {
        self.function_order.push(function.name);
        self.functions.insert(function.name, function);
    }

    pub fn add_class(&mut self, class: LIRClass) {
        self.classes.insert(class.name, class);
    }
}

impl Default for LIRProgram {
    fn default() -> Self {
        Self::new()
    }
}
