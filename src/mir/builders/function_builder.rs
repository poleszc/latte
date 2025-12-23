use crate::{SourcePosition, mir::*, parser::ast::Type, utils::StrID};
use std::collections::HashMap;

/// Builder for MIR functions.
pub struct MIRFunctionBuilder {
    name: StrID,
    return_type: Type,
    parameter_types: Vec<Type>,
    locals: Vec<LocalInfo>,
    temps: Vec<TempInfo>,
    blocks: HashMap<BlockID, MIRBlock>,
    block_order: Vec<BlockID>,
    current_block: Option<BlockID>,
    next_block_id: usize,
    next_local_id: usize,
    next_temp_id: usize,
    position: SourcePosition,
}

// ======================
// Basic functionality.
// ======================
impl MIRFunctionBuilder {
    pub fn new(
        name: StrID,
        return_type: Type,
        parameter_types: Vec<Type>,
        position: SourcePosition,
    ) -> Self {
        let mut builder = MIRFunctionBuilder {
            name,
            return_type,
            parameter_types,
            locals: Vec::new(),
            temps: Vec::new(),
            blocks: HashMap::new(),
            block_order: Vec::new(),
            current_block: None,
            next_block_id: 0,
            next_local_id: 0,
            next_temp_id: 0,
            position,
        };

        let entry = builder.create_block();
        builder.set_current_block(entry);
        builder
    }

    /// Create a new block.
    pub fn create_block(&mut self) -> BlockID {
        let id = BlockID(self.next_block_id);
        self.next_block_id += 1;

        let block = MIRBlock {
            id,
            instructions: Vec::new(),
            terminator: None,
        };

        self.blocks.insert(id, block);
        self.block_order.push(id);
        id
    }

    /// Set the block we're currently building.
    pub fn set_current_block(&mut self, id: BlockID) {
        self.current_block = Some(id)
    }

    /// Get the BlockID of the block we're currently building
    pub fn current_block(&self) -> BlockID {
        self.current_block.expect("Current block not set")
    }

    /// Add instruction to the current_block.
    pub fn emit(&mut self, instruction: MIRInstruction) {
        let block_id = self.current_block();
        let block = self
            .blocks
            .get_mut(&block_id)
            .expect("Emit: Block doesn't exist");
        block.instructions.push(instruction);
    }

    /// Set a terminator for a current block.
    pub fn set_terminator(&mut self, terminator: MIRTerminator) {
        let block_id = self.current_block();
        let block = self
            .blocks
            .get_mut(&block_id)
            .expect("Set Terminator:Block doesn't exist");
        if block.terminator.is_some() {
            panic!("Terminator set twice for a block");
        }
        block.terminator = Some(terminator);
    }

    /// Is the current block terminated.
    pub fn is_current_block_terminated(&self) -> bool {
        let block_id = self.current_block();
        let block = self
            .blocks
            .get(&block_id)
            .expect("Set Terminator:Block doesn't exist");
        block.terminator.is_some()
    }

    /// Create a new local variable for a current block.
    pub fn create_local(
        &mut self,
        name: Option<StrID>,
        typ: Type,
        is_parameter: bool,
        parameter_index: Option<usize>,
    ) -> LocalID {
        let id = LocalID(self.next_local_id);
        self.next_local_id += 1;

        self.locals.push(LocalInfo {
            id,
            name,
            typ,
            is_parameter,
            parameter_index,
        });
        id
    }

    /// Create a new temporary variable for a current block.
    pub fn create_temp(&mut self, typ: Type) -> TempID {
        let id = TempID(self.next_temp_id);
        self.next_temp_id += 1;
        self.temps.push(TempInfo { id, typ });
        id
    }

    pub fn get_local_info(&self, id: LocalID) -> &LocalInfo {
        self.locals.get(id.0).expect("Invalid LocalID")
    }

    /// Creates the build MIRFunction. Consumes the builder object.
    pub fn build(self) -> MIRFunction {
        MIRFunction {
            name: self.name,
            return_type: self.return_type,
            parameter_types: self.parameter_types,
            locals: self.locals,
            temps: self.temps,
            blocks: self.blocks,
            block_order: self.block_order,
            entry_block: BlockID(0),
            source_position: self.position,
        }
    }
}

// ============================
// Instruction emmiter helpers.
// ============================
impl MIRFunctionBuilder {
    pub fn emit_assign(&mut self, dest: MIRPlace, value: MIRValue, position: SourcePosition) {
        self.emit(MIRInstruction {
            operation: MIROperation::Assign,
            destination: Some(dest),
            operands: vec![value],
            source_position: position,
        });
    }

    pub fn emit_binary(
        &mut self,
        operation: MIROperation,
        dest: MIRPlace,
        lhs: MIRValue,
        rhs: MIRValue,
        position: SourcePosition,
    ) {
        self.emit(MIRInstruction {
            operation,
            destination: Some(dest),
            operands: vec![lhs, rhs],
            source_position: position,
        });
    }

    pub fn emit_unary(
        &mut self,
        operation: MIROperation,
        dest: MIRPlace,
        value: MIRValue,
        position: SourcePosition,
    ) {
        self.emit(MIRInstruction {
            operation,
            destination: Some(dest),
            operands: vec![value],
            source_position: position,
        });
    }

    pub fn emit_less_then(
        &mut self,
        dest: MIRPlace,
        lhs: MIRValue,
        rhs: MIRValue,
        position: SourcePosition,
    ) {
        self.emit(MIRInstruction {
            operation: MIROperation::IntLt,
            destination: Some(dest),
            operands: vec![lhs, rhs],
            source_position: position,
        });
    }

    pub fn emit_array_length(&mut self, dest: MIRPlace, value: MIRValue, position: SourcePosition) {
        self.emit(MIRInstruction {
            operation: MIROperation::ArrayLength,
            destination: Some(dest),
            operands: vec![value],
            source_position: position,
        });
    }

    pub fn emit_string_length(
        &mut self,
        dest: MIRPlace,
        value: MIRValue,
        position: SourcePosition,
    ) {
        self.emit(MIRInstruction {
            operation: MIROperation::StringLength,
            destination: Some(dest),
            operands: vec![value],
            source_position: position,
        });
    }

    pub fn emit_array_get(
        &mut self,
        dest: MIRPlace,
        array: MIRValue,
        index: MIRValue,
        position: SourcePosition,
    ) {
        self.emit(MIRInstruction {
            operation: MIROperation::ArrayGet,
            destination: Some(dest),
            operands: vec![array, index],
            source_position: position,
        });
    }

    pub fn emit_call(
        &mut self,
        dest: Option<MIRPlace>,
        function: StrID,
        args: Vec<MIRValue>,
        position: SourcePosition,
    ) {
        self.emit(MIRInstruction {
            operation: MIROperation::Call,
            destination: dest,
            operands: {
                // Add function.
                let mut operands = vec![MIRValue::Function(function)];
                // Add function arguments.
                operands.extend(args);
                operands
            },
            source_position: position,
        });
    }

    pub fn emit_virtual_call(
        &mut self,
        dest: Option<MIRPlace>,
        object: MIRValue,
        vtable_index: usize,
        args: Vec<MIRValue>,
        position: SourcePosition,
    ) {
        self.emit(MIRInstruction {
            operation: MIROperation::VCall,
            destination: dest,
            operands: {
                // [object, index, arg1, arg2, ...]
                let mut operands = vec![object, MIRValue::IntConst(vtable_index as i32)];
                // Add function arguments.
                operands.extend(args);
                operands
            },
            source_position: position,
        });
    }

    pub fn emit_new_object(&mut self, dest: MIRPlace, class_name: StrID, position: SourcePosition) {
        self.emit(MIRInstruction {
            operation: MIROperation::NewObject,
            destination: Some(dest),
            operands: vec![MIRValue::Class(class_name)],
            source_position: position,
        });
    }

    pub fn emit_new_array(&mut self, dest: MIRPlace, size: MIRValue, position: SourcePosition) {
        self.emit(MIRInstruction {
            operation: MIROperation::NewArray,
            destination: Some(dest),
            operands: vec![size],
            source_position: position,
        });
    }

    pub fn emit_field_get(
        &mut self,
        dest: MIRPlace,
        object: MIRValue,
        field_index: usize,
        position: SourcePosition,
    ) {
        self.emit(MIRInstruction {
            operation: MIROperation::FieldGet,
            destination: Some(dest),
            operands: vec![object, MIRValue::IntConst(field_index as i32)],
            source_position: position,
        });
    }

    pub fn emit_field_set(
        &mut self,
        object: MIRValue,
        field_index: usize,
        value: MIRValue,
        position: SourcePosition,
    ) {
        self.emit(MIRInstruction {
            operation: MIROperation::FieldSet,
            destination: None,
            operands: vec![object, MIRValue::IntConst(field_index as i32), value],
            source_position: position,
        });
    }
}

// ======================
// Terminator helpers.
// ======================
impl MIRFunctionBuilder {
    pub fn emit_jump(&mut self, target: BlockID, position: SourcePosition) {
        self.set_terminator(MIRTerminator::Jump { target, position });
    }

    pub fn emit_branch(
        &mut self,
        condition: MIRValue,
        if_true: BlockID,
        if_false: BlockID,
        position: SourcePosition,
    ) {
        self.set_terminator(MIRTerminator::Branch {
            condition,
            if_true,
            if_false,
            position,
        });
    }

    pub fn emit_return(&mut self, value: Option<MIRValue>, position: SourcePosition) {
        self.set_terminator(MIRTerminator::Return { value, position });
    }

    pub fn emit_unreachable(&mut self, position: SourcePosition) {
        self.set_terminator(MIRTerminator::Unreachable { position });
    }
}
