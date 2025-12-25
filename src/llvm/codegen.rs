use std::{collections::HashMap, fmt::Write};

use crate::{
    lir::{
        BlockID, LIRBlock, LIRClass, LIRFunction, LIRInstruction, LIROperation, LIRProgram,
        LIRTerminator, LIRValue, RegisterID,
    },
    parser::{Type, TypeKind},
};

pub struct LLVMGenerator<'a> {
    output: String,
    string_constants: HashMap<String, String>,
    next_string_id: usize,
    current_function: Option<&'a LIRFunction>,
    // A mapping from LIR BlockID to the name of the last label ending the block.
    // This is a super ugly approach, since array accesses and integer division
    // cause new blocks to appear which makes the phi functions generated in LIR invalid.
    // This mapping is created on the first pass of lowering, new block end labels are created
    // and used in the second pass when lowering the phi functions.
    lir_block_to_last_llvm_label: HashMap<BlockID, String>,
    current_block_split_count: usize,
}

impl<'a> Default for LLVMGenerator<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> LLVMGenerator<'a> {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            string_constants: HashMap::new(),
            next_string_id: 0,
            current_function: None,
            lir_block_to_last_llvm_label: HashMap::new(),
            current_block_split_count: 0,
        }
    }

    pub fn generate_program(mut self, program: &'a LIRProgram) -> String {
        // Emit builtins.
        self.emit_preambule();

        let mut sorted_classes: Vec<_> = program.classes.keys().cloned().collect();
        sorted_classes.sort_by_key(|id| id.to_string());
        for class_id in &sorted_classes {
            let class = &program.classes[class_id];
            self.emit_class_definition(class);
        }

        for class_id in &sorted_classes {
            let class = &program.classes[class_id];
            self.emit_vtable_definition(class, program);
        }

        for func_name in &program.function_order {
            if let Some(func) = program.functions.get(func_name) {
                self.current_function = Some(func);
                self.emit_function(func);
                self.current_function = None;
            }
        }

        let mut string_constants = String::new();
        for (string_val, id) in &self.string_constants {
            let escaped = string_val.replace("\\", "\\\\").replace("\"", "\\22");
            let len = string_val.len() + 1;
            string_constants.push_str(&format!(
                "@{} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1\n",
                id, len, escaped
            ));
        }

        format!("{}\n{}", string_constants, self.output)
    }

    /// Performs a pass through all function blocks looking for instructions which
    /// cause the control flow to be changed. Precalculates the label names which
    /// will end the blocks after they will be broken by the bound checks/zero 
    /// division checks. This mapping is then used when lowering phi functions.
    fn precalculate_exit_labels(&mut self, func: &LIRFunction) {
        for (block_id, block) in &func.blocks {
            let mut current_label = format!("B{}", block_id.0);
            let mut split_count = 0;

            for instr in &block.instructions {
                match instr.operation {
                    LIROperation::ArrayGet
                    | LIROperation::ArraySet
                    | LIROperation::IntDiv
                    | LIROperation::IntMod => {
                        current_label = format!("B{}_ok_{}", block_id.0, split_count);
                        split_count += 1;
                    }
                    _ => {}
                }
            }
            self.lir_block_to_last_llvm_label
                .insert(*block_id, current_label);
        }
    }
}

// ======================
// Emmiters
// ======================
impl<'a> LLVMGenerator<'a> {
    fn emit_line(&mut self, content: &str) {
        writeln!(self.output, "{}", content).unwrap();
    }

    fn emit_line_indented(&mut self, content: &str) {
        writeln!(self.output, "  {}", content).unwrap();
    }

    fn emit_line_very_indented(&mut self, content: &str) {
        writeln!(self.output, "    {}", content).unwrap();
    }

    fn emit_preambule(&mut self) {
        self.emit_line("declare void @printInt(i32)");
        self.emit_line("declare void @printString(i8*)"); // TODO: Opaque pointer?
        self.emit_line("declare void @error()");
        self.emit_line("declare i32 @readInt()");
        self.emit_line("declare i8* @readString()"); // TODO: opaque?
        self.emit_line("declare i8* @stringConcat(i8*, i8*)"); // TODO: opaque?
        self.emit_line("declare i1 @stringEqual(i8*, i8*)"); // TODO: opaque?
        self.emit_line("declare i1 @stringNotEqual(i8*, i8*)"); // TODO: opaque?
        self.emit_line("declare i32 @stringLength(i8*)"); // TODO: opaque?
        self.emit_line("declare i8* @calloc(i64, i64)"); // TODO: opaque?
        self.emit_line("");
    }

    fn emit_function(&mut self, func: &LIRFunction) {
        self.lir_block_to_last_llvm_label.clear();
        self.precalculate_exit_labels(func);

        let ret_type = self.llvm_type(&func.return_type);
        let args = func
            .parameter_registers
            .iter()
            .zip(&func.parameter_types)
            .map(|(reg, typ)| format!("{} %r{}", self.llvm_type(typ), reg.0))
            .collect::<Vec<String>>()
            .join(", ");

        self.emit_line(format!("define {} @{}({}) {{", ret_type, func.name, args).as_str());

        for block_id in &func.block_order {
            if let Some(block) = func.blocks.get(block_id) {
                self.emit_block(*block_id, block);
            }
        }
        self.emit_line("}\n");
    }
    
    fn emit_class_definition(&mut self, class: &LIRClass) {
        let mut fields = vec!["i8**".to_string()];
        for field in &class.fields {
            fields.push(self.llvm_type(&field.typ));
        }
        self.emit_line(&format!(
            "%class.{} = type {{ {} }}",
            class.name,
            fields.join(", ")
        ));
    }

    fn emit_vtable_definition(&mut self, class: &LIRClass, program: &LIRProgram) {
        let mut methods = Vec::new();
        for method_name in &class.vtable {
            let func = program
                .functions
                .get(method_name)
                .expect("Virtual method not found");
            let ret_type = self.llvm_type(&func.return_type);
            let arg_types: Vec<String> = func
                .parameter_types
                .iter()
                .map(|typ| self.llvm_type(typ))
                .collect();

            let func_signature = format!("{} ({})*", ret_type, arg_types.join(", "));
            methods.push(format!(
                "i8* bitcast ({} @{} to i8*)",
                func_signature, method_name
            ));
        }
        let len = class.vtable.len();
        self.emit_line(&format!(
            "@vtable.{} = constant [{} x i8*] [ {} ]",
            class.name,
            if len == 0 { 1 } else { len },
            if len == 0 {
                "i8* null".to_string()
            } else {
                methods.join(", ")
            }
        ));
    }

    fn emit_block(&mut self, block_id: BlockID, block: &LIRBlock) {
        self.current_block_split_count = 0;
        let base_label = format!("B{}", block_id.0);
        self.emit_line_indented(format!("{}:", base_label).as_str());

        for instr in &block.instructions {
            self.emit_instruction(instr, block_id);
        }

        if let Some(term) = &block.terminator {
            self.emit_terminator(term);
        } else {
            panic!("NO TERMINATOR NOOOOOO")
        }
    }

    fn emit_instruction(&mut self, instr: &LIRInstruction, block_id: BlockID) {
        match instr.operation {
            // TODO: this should be removed in the future.
            LIROperation::Assign => {
                let dest = instr.destination.unwrap();
                let val = self.format_value(&instr.operands[0]);
                let typ = self.get_llvm_type_from_register(dest);
                self.emit_line_very_indented(
                    format!("%r{} = bitcast {} {} to {}", dest.0, typ, val, typ).as_str(),
                );
            }
            LIROperation::IntAdd | LIROperation::IntSub | LIROperation::IntMul => {
                let op = match instr.operation {
                    LIROperation::IntAdd => "add",
                    LIROperation::IntSub => "sub",
                    _ => "mul",
                };
                let dest = instr.destination.unwrap();
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_line_very_indented(
                    format!("%r{} = {} i32 {}, {}", dest.0, op, lhs, rhs).as_str(),
                );
            }
            LIROperation::IntDiv | LIROperation::IntMod => {
                let dest = instr.destination.unwrap();
                let rhs = self.format_value(&instr.operands[1]);

                let split_id = self.current_block_split_count;
                self.current_block_split_count += 1;

                let err_label = format!("B{}_div_err_{}", block_id.0, split_id);
                let ok_label = format!("B{}_ok_{}", block_id.0, split_id);

                self.emit_line_very_indented(
                    format!(
                        "%cond_z_{}_{} = icmp eq i32 {}, 0",
                        block_id.0, split_id, rhs
                    )
                    .as_str(),
                );
                self.emit_line_very_indented(
                    format!(
                        "br i1 %cond_z_{}_{}, label %{}, label %{}",
                        block_id.0, split_id, err_label, ok_label
                    )
                    .as_str(),
                );
                self.emit_line_indented(format!("{}:", err_label).as_str());
                self.emit_line_very_indented("call void @error()");
                self.emit_line_very_indented("unreachable");
                self.emit_line_indented(format!("{}:", ok_label).as_str());

                let op = if instr.operation == LIROperation::IntDiv {
                    "sdiv"
                } else {
                    "srem"
                };
                let lhs = self.format_value(&instr.operands[0]);
                self.emit_line_very_indented(
                    format!("%r{} = {} i32 {}, {}", dest.0, op, lhs, rhs).as_str(),
                );
            }
            LIROperation::IntNeg => {
                let dest = instr.destination.unwrap();
                let val = self.format_value(&instr.operands[0]);
                self.emit_line_very_indented(format!("%r{} = sub i32 0, {}", dest.0, val).as_str());
            }
            LIROperation::Phi => {
                let dest = instr.destination.unwrap();
                let llvm_type = self.get_llvm_type_from_register(dest);

                let mut phi = format!("%r{} = phi {} ", dest.0, llvm_type);
                let mut args = Vec::new();
                for i in (0..instr.operands.len()).step_by(2) {
                    let val = self.format_value(&instr.operands[i]);
                    let block_id = match &instr.operands[i + 1] {
                        LIRValue::Block(id) => id,
                        _ => unreachable!(),
                    };

                    let last_label = self
                        .lir_block_to_last_llvm_label
                        .get(block_id)
                        .expect("Block order error: {}");

                    args.push(format!("[ {}, %{} ]", val, last_label));
                }
                phi.push_str(&args.join(", "));
                self.emit_line_very_indented(phi.as_str());
            }
            LIROperation::IntLt
            | LIROperation::IntLe
            | LIROperation::IntGt
            | LIROperation::IntGe
            | LIROperation::IntEq
            | LIROperation::IntNeq => {
                let cond = match instr.operation {
                    LIROperation::IntLt => "slt",
                    LIROperation::IntLe => "sle",
                    LIROperation::IntGt => "sgt",
                    LIROperation::IntGe => "sge",
                    LIROperation::IntEq => "eq",
                    _ => "ne",
                };
                let dest = instr.destination.unwrap();
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                let typ = match &instr.operands[0] {
                    LIRValue::Register(id) => self.get_llvm_type_from_register(*id),
                    LIRValue::IntConst(_) => "i32".to_string(),
                    LIRValue::BoolConst(_) => "i1".to_string(),
                    LIRValue::StringConst(_) => "i8*".to_string(),
                    LIRValue::Null => "i8*".to_string(),
                    _ => unreachable!(),
                };
                self.emit_line_very_indented(
                    format!("%r{} = icmp {} {} {}, {}", dest.0, cond, typ, lhs, rhs).as_str(),
                );
            }
            LIROperation::BoolAnd => {
                let dest = instr.destination.unwrap();
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_line_very_indented(
                    format!("%r{} = and i1 {}, {}", dest.0, lhs, rhs).as_str(),
                );
            }
            LIROperation::BoolOr => {
                let dest = instr.destination.unwrap();
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_line_very_indented(
                    format!("%r{} = or i1 {}, {}", dest.0, lhs, rhs).as_str(),
                );
            }
            LIROperation::BoolNot => {
                let dest = instr.destination.unwrap();
                let val = self.format_value(&instr.operands[0]);
                // TODO: Does XOR work here?
                self.emit_line_very_indented(format!("%r{} = xor i1 {}, 1", dest.0, val).as_str());
            }
            LIROperation::StringConcat => {
                let dest = instr.destination.unwrap();
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_line_very_indented(
                    format!(
                        "%r{} = call i8* @stringConcat(i8* {}, i8* {})",
                        dest.0, lhs, rhs
                    )
                    .as_str(),
                );
            }
            LIROperation::StringEq => {
                let dest = instr.destination.unwrap();
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_line_very_indented(
                    format!(
                        "%r{} = call i1 @stringEqual(i8* {}, i8* {})",
                        dest.0, lhs, rhs
                    )
                    .as_str(),
                );
            }
            LIROperation::StringNeq => {
                let dest = instr.destination.unwrap();
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_line_very_indented(
                    format!(
                        "%r{} = call i1 @stringNotEqual(i8* {}, i8* {})",
                        dest.0, lhs, rhs
                    )
                    .as_str(),
                );
            }
            LIROperation::StringLength => {
                let dest = instr.destination.unwrap();
                let val = self.format_value(&instr.operands[0]);
                self.emit_line_very_indented(
                    format!("%r{} = call i32 @stringLength(i8* {})", dest.0, val).as_str(),
                );
            }
            LIROperation::Call => {
                let func_name = match &instr.operands[0] {
                    LIRValue::Function(name) => name.as_str(),
                    _ => unreachable!(),
                };

                let args = instr.operands[1..]
                    .iter()
                    .map(|v| {
                        let val_str = self.format_value(v);
                        // TODO: Unify, since its duplicated in Return as well
                        let typ_str = match v {
                            LIRValue::Register(register_id) => {
                                self.get_llvm_type_from_register(*register_id)
                            }
                            LIRValue::IntConst(_) => "i32".to_string(),
                            LIRValue::BoolConst(_) => "i1".to_string(),
                            LIRValue::StringConst(_) => "i8*".to_string(),
                            LIRValue::Class(class_name) => format!("%class.{}*", class_name),
                            _ => unreachable!(),
                        };
                        format!("{} {}", typ_str, val_str)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                if let Some(dest) = instr.destination {
                    let ret_type = self.get_llvm_type_from_register(dest);
                    self.emit_line_very_indented(
                        format!("%r{} = call {} @{}({})", dest.0, ret_type, func_name, args)
                            .as_str(),
                    );
                } else {
                    self.emit_line_very_indented(
                        format!("call void @{}({})", func_name, args).as_str(),
                    );
                }
            }
            LIROperation::VCall => {
                let dest = instr.destination;
                let obj = &instr.operands[0];
                let obj_val = self.format_value(obj);

                let obj_reg_id = match obj {
                    LIRValue::Register(r) => *r,
                    _ => unreachable!(),
                };

                let method_idx = match &instr.operands[1] {
                    LIRValue::IntConst(i) => *i,
                    _ => unreachable!(),
                };

                let method_args = &instr.operands[2..];
                let class_name = self.get_class_name_from_register(obj_reg_id);
                let id = self.next_string_id;
                self.next_string_id += 1;

                // TODO: Check for null ptr access.
                // Get the VTable pointer from the zeroth field.
                let vptr_ptr = format!("%vptr_ptr_vcall_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = getelementptr %class.{}, %class.{}* {}, i32 0, i32 0",
                    vptr_ptr, class_name, class_name, obj_val
                ));
                let vptr = format!("%vptr_vcall_{}", id);
                self.emit_line_very_indented(&format!("{} = load i8**, i8*** {}", vptr, vptr_ptr));

                // Get the function pointer.
                let method_ptr_ptr = format!("%method_ptr_ptr_vcall_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = getelementptr i8*, i8** {}, i64 {}",
                    method_ptr_ptr, vptr, method_idx
                ));

                let method_ptr_raw = format!("%method_ptr_raw_vcall_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = load i8*, i8** {}",
                    method_ptr_raw, method_ptr_ptr
                ));

                // Build the signature again. TODO: This is stupid
                let ret_type =
                    dest.map_or("void".to_string(), |r| self.get_llvm_type_from_register(r));

                let mut arg_types = vec![self.get_llvm_type_from_register(obj_reg_id)];
                arg_types.extend(method_args.iter().map(|arg| self.get_value_type(arg)));
                let func_type = format!("{} ({})*", ret_type, arg_types.join(", "));

                // Cast the pointer to the expected signature.
                let method_ptr = format!("%method_ptr_vcall_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = bitcast i8* {} to {}",
                    method_ptr, method_ptr_raw, func_type
                ));

                let mut call_args = vec![format!(
                    "{} {}",
                    self.get_llvm_type_from_register(obj_reg_id),
                    obj_val
                )];
                call_args.extend(
                    method_args.iter().map(|arg| {
                        format!("{} {}", self.get_value_type(arg), self.format_value(arg))
                    }),
                );

                if let Some(dest_reg) = dest {
                    self.emit_line_very_indented(&format!(
                        "%r{} = call {} {}({})",
                        dest_reg.0,
                        ret_type,
                        method_ptr,
                        call_args.join(", ")
                    ));
                } else {
                    self.emit_line_very_indented(&format!(
                        "call void {}({})",
                        method_ptr,
                        call_args.join(", ")
                    ));
                }
            }
            LIROperation::ArrayGet => {
                let dest = instr.destination.unwrap();
                let arr_val = self.format_value(&instr.operands[0]);
                let idx_val = self.format_value(&instr.operands[1]);
                let elem_type = self.get_llvm_type_from_register(dest);
                let id = self.next_string_id;
                self.next_string_id += 1;

                self.emit_array_bounds_check(&arr_val, &idx_val, &block_id);

                // First 8 bytes are for the length. Skip them.
                let data_ptr_raw = format!("%data_ptr_raw_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = getelementptr i8, i8* {}, i32 8",
                    data_ptr_raw, arr_val
                ));
                let data_ptr = format!("%data_ptr_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = bitcast i8* {} to {}*",
                    data_ptr, data_ptr_raw, elem_type
                ));

                let elem_ptr = format!("%elem_ptr_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = getelementptr {}, {}* {}, i32 {}",
                    elem_ptr, elem_type, elem_type, data_ptr, idx_val
                ));
                self.emit_line_very_indented(&format!(
                    "%r{} = load {}, {}* {}",
                    dest.0, elem_type, elem_type, elem_ptr
                ));
            }
            LIROperation::ArraySet => {
                let arr_val = self.format_value(&instr.operands[0]);
                let idx_val = self.format_value(&instr.operands[1]);
                let val_to_store = self.format_value(&instr.operands[2]);
                let elem_type = self.get_value_type(&instr.operands[2]);
                let id = self.next_string_id;
                self.next_string_id += 1;

                self.emit_array_bounds_check(&arr_val, &idx_val, &block_id);

                let data_ptr_raw = format!("%set_data_ptr_raw_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = getelementptr i8, i8* {}, i32 8",
                    data_ptr_raw, arr_val
                ));

                let data_ptr = format!("%set_data_ptr_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = bitcast i8* {} to {}*",
                    data_ptr, data_ptr_raw, elem_type
                ));

                let elem_ptr = format!("%elem_ptr_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = getelementptr {}, {}* {}, i32 {}",
                    elem_ptr, elem_type, elem_type, data_ptr, idx_val
                ));

                self.emit_line_very_indented(&format!(
                    "store {} {}, {}* {}",
                    elem_type, val_to_store, elem_type, elem_ptr
                ));
            }
            LIROperation::ArrayLength => {
                let dest = instr.destination.unwrap();
                let arr_ptr = self.format_value(&instr.operands[0]);

                let len_ptr = format!("%len_ptr_{}", dest.0);
                self.emit_line_very_indented(&format!(
                    "{} = bitcast i8* {} to i32*",
                    len_ptr, arr_ptr
                ));
                self.emit_line_very_indented(&format!("%r{} = load i32, i32* {}", dest.0, len_ptr));
            }
            LIROperation::NewArray => {
                let dest = instr.destination.unwrap();
                let count_val = self.format_value(&instr.operands[0]);

                let reg_info = &self.current_function.unwrap().registers[dest.0];
                let elem_type = match &reg_info.typ.kind {
                    TypeKind::Array(inner) => inner,
                    _ => unreachable!(),
                };

                let element_size = self.sizeof_type(elem_type);
                let id = dest.0;

                self.emit_line_very_indented(&format!(
                    "%element_count_{} = sext i32 {} to i64",
                    id, count_val
                ));
                self.emit_line_very_indented(&format!(
                    "%data_size_{} = mul i64 %element_count_{}, {}",
                    id, id, element_size
                ));
                self.emit_line_very_indented(&format!(
                    "%total_size_{} = add i64 %data_size_{}, 8",
                    id, id
                ));

                let raw_ptr = format!("%raw_arr_{}", dest.0);
                self.emit_line_very_indented(&format!(
                    "{} = call i8* @calloc(i64 1, i64 %total_size_{})",
                    raw_ptr, id
                ));

                let len_ptr = format!("%len_ptr_{}", id);
                self.emit_line_very_indented(&format!(
                    "{} = bitcast i8* {} to i32*",
                    len_ptr, raw_ptr
                ));
                self.emit_line_very_indented(&format!("store i32 {}, i32* {}", count_val, len_ptr));

                self.emit_line_very_indented(&format!(
                    "%r{} = bitcast i8* {} to i8*",
                    dest.0, raw_ptr
                ));
            }
            LIROperation::NewObject => {
                let dest = instr.destination.unwrap();
                let class_name = match &instr.operands[0] {
                    LIRValue::Class(class_name) => class_name,
                    _ => unreachable!(),
                };

                // Calculate the size of the object to allocate.
                let size_tmp = format!("%size_{}", dest.0);
                self.emit_line_very_indented(&format!(
                    "{} = getelementptr %class.{}, %class.{}* null, i32 1",
                    size_tmp, class_name, class_name
                ));
                let size_int = format!("%size_i_{}", dest.0);
                self.emit_line_very_indented(&format!(
                    "{} = ptrtoint %class.{}* {} to i64",
                    size_int, class_name, size_tmp
                ));

                // Actually alloc the object
                let raw_ptr = format!("%raw_{}", dest.0);
                self.emit_line_very_indented(&format!(
                    "{} = call i8* @calloc(i64 1, i64 {})",
                    raw_ptr, size_int
                ));

                let obj_ptr = format!("%r{}", dest.0);
                self.emit_line_very_indented(&format!(
                    "{} = bitcast i8* {} to %class.{}*",
                    obj_ptr, raw_ptr, class_name
                ));

                let vptr_ptr = format!("%vptr_ptr_{}", dest.0);
                self.emit_line_very_indented(&format!(
                    "{} = getelementptr %class.{}, %class.{}* {}, i32 0, i32 0",
                    vptr_ptr, class_name, class_name, obj_ptr
                ));
                self.emit_line_very_indented(&format!(
                    "store i8** getelementptr ([{} x i8*], [{} x i8*]* @vtable.{}, i32 0, i32 0), i8*** {}", 
                    class_name.len(), class_name.len(), class_name, vptr_ptr
                ));
            }
            LIROperation::FieldGet => {
                let dest = instr.destination.unwrap();
                let obj_val = self.format_value(&instr.operands[0]);
                let field_idx = match &instr.operands[1] {
                    LIRValue::IntConst(i) => i + 1, // +1 because vtable pointer is on the first one.
                    _ => unreachable!(),
                };

                let class_name = self.get_class_name_from_register(match &instr.operands[0] {
                    LIRValue::Register(r) => *r,
                    _ => unreachable!(),
                });

                let field_ptr = format!("%field_ptr_{}", dest.0);
                let field_type = self.get_llvm_type_from_register(dest);

                self.emit_line_very_indented(&format!(
                    "{} = getelementptr %class.{}, %class.{}* {}, i32 0, i32 {}",
                    field_ptr, class_name, class_name, obj_val, field_idx
                ));
                self.emit_line_very_indented(&format!(
                    "%r{} = load {}, {}* {}",
                    dest.0, field_type, field_type, field_ptr
                ));
            }
            LIROperation::FieldSet => {
                let obj_val = self.format_value(&instr.operands[0]);
                let field_idx = match &instr.operands[1] {
                    LIRValue::IntConst(i) => i + 1,
                    _ => unreachable!(),
                };
                let val_to_store = self.format_value(&instr.operands[2]);
                let val_type = self.get_value_type(&instr.operands[2]);

                let class_name = self.get_class_name_from_register(match &instr.operands[0] {
                    LIRValue::Register(r) => *r,
                    _ => unreachable!(),
                });

                // TODO: Separate counter?
                let field_ptr = format!("%field_set_ptr_{}", self.next_string_id);
                self.next_string_id += 1;

                self.emit_line_very_indented(&format!(
                    "{} = getelementptr %class.{}, %class.{}* {}, i32 0, i32 {}",
                    field_ptr, class_name, class_name, obj_val, field_idx
                ));
                self.emit_line_very_indented(&format!(
                    "store {} {}, {}* {}",
                    val_type, val_to_store, val_type, field_ptr
                ));
            }
            LIROperation::IsNull => {
                let dest = instr.destination.unwrap();
                let val = self.format_value(&instr.operands[0]);
                let typ = "i8*"; // TODO: Temporary
                self.emit_line_very_indented(
                    format!("%r{} = icmp eq {} {}, null", dest.0, typ, val).as_str(),
                );
            }
        }
    }

    fn emit_terminator(&mut self, term: &LIRTerminator) {
        match term {
            LIRTerminator::Jump { target, .. } => {
                self.emit_line_very_indented(format!("br label %B{}", target.0).as_str());
            }
            LIRTerminator::Branch {
                condition,
                if_true,
                if_false,
                ..
            } => {
                let cond = self.format_value(condition);
                self.emit_line_very_indented(
                    format!(
                        "br i1 {}, label %B{}, label %B{}",
                        cond, if_true.0, if_false.0
                    )
                    .as_str(),
                );
            }
            LIRTerminator::Return { value, .. } => {
                if let Some(val) = value {
                    let formatted_val = self.format_value(val);
                    let typ_str = match val {
                        LIRValue::Register(register_id) => {
                            self.get_llvm_type_from_register(*register_id)
                        }
                        LIRValue::IntConst(_) => "i32".to_string(),
                        LIRValue::BoolConst(_) => "i1".to_string(),
                        LIRValue::StringConst(_) => "i8*".to_string(),
                        LIRValue::Class(class_name) => format!("%class.{}*", class_name),
                        LIRValue::Null => "ptr".to_string(),
                        _ => unreachable!(),
                    };
                    self.emit_line_very_indented(
                        format!("ret {} {}", typ_str, formatted_val).as_str(),
                    );
                } else {
                    self.emit_line_very_indented("ret void");
                }
            }
            LIRTerminator::Unreachable { .. } => {
                self.emit_line_very_indented("unreachable");
            }
        }
    }

    fn emit_array_bounds_check(&mut self, arr_ptr: &str, index_val: &str, block_id: &BlockID) {
        let split_id = self.current_block_split_count;
        self.current_block_split_count += 1;
        let id = self.next_string_id;
        self.next_string_id += 1;

        let len_ptr = format!("%len_ptr_check_{}", id);
        let len_val = format!("%len_val_check_{}", id);
        let cond_neg = format!("%cond_neg_{}", id);
        let cond_ge = format!("%cond_ge_{}", id);
        let cond_out = format!("%cond_out_{}", id);
        let err_label = format!("B{}_err_{}", block_id.0, split_id);
        let ok_label = format!("B{}_ok_{}", block_id.0, split_id);

        // Retrieve the array size.
        self.emit_line_very_indented(&format!("{} = bitcast i8* {} to i32*", len_ptr, arr_ptr));
        self.emit_line_very_indented(&format!("{} = load i32, i32* {}", len_val, len_ptr));

        // index >= 0
        self.emit_line_very_indented(&format!("{} = icmp slt i32 {}, 0", cond_neg, index_val));
        // index < length
        self.emit_line_very_indented(&format!(
            "{} = icmp sge i32 {}, {}",
            cond_ge, index_val, len_val
        ));

        self.emit_line_very_indented(&format!("{} = or i1 {}, {}", cond_out, cond_neg, cond_ge));

        self.emit_line_very_indented(&format!(
            "br i1 {}, label %{}, label %{}",
            cond_out, err_label, ok_label
        ));

        self.emit_line_indented(&format!("{}:", err_label));
        self.emit_line_very_indented("call void @error()");
        self.emit_line_very_indented("unreachable");

        self.emit_line_indented(&format!("{}:", ok_label));
    }
}

// ======================
// Helpers
// ======================
impl<'a> LLVMGenerator<'a> {
    fn llvm_type(&self, typ: &Type) -> String {
        match &typ.kind {
            TypeKind::Int => "i32".to_string(),
            TypeKind::Bool => "i1".to_string(),
            TypeKind::Void => "void".to_string(),
            TypeKind::Class(class_name) => format!("%class.{}*", class_name.name),
            TypeKind::Array(_) => "i8*".to_string(),
            TypeKind::Error => {
                panic!("Something went very wrong, Error type in LLVM backend!")
            }
            crate::parser::TypeKind::Fun(_, _) => panic!("Function type in llvm"),
            _ => "i8*".to_string(),
        }
    }

    fn get_value_type(&self, val: &LIRValue) -> String {
        match val {
            LIRValue::IntConst(_) => "i32".to_string(),
            LIRValue::BoolConst(_) => "i1".to_string(),
            LIRValue::StringConst(_) => "i8*".to_string(),
            LIRValue::Null => "i8*".to_string(),
            LIRValue::Register(id) => self.get_llvm_type_from_register(*id),
            LIRValue::Class(name) => format!("%class.{}*", name),
            _ => "i8*".to_string(),
        }
    }

    fn sizeof_type(&self, typ: &Type) -> usize {
        match &typ.kind {
            TypeKind::Int => 4,
            TypeKind::Bool => 1,
            TypeKind::String | TypeKind::Class(_) | TypeKind::Array(_) => 8,
            _ => panic!("Unhanded type for size"),
        }
    }

    fn format_value(&mut self, val: &LIRValue) -> String {
        match val {
            LIRValue::Register(id) => format!("%r{}", id.0),
            LIRValue::IntConst(n) => n.to_string(),
            LIRValue::BoolConst(b) => {
                if *b {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            LIRValue::StringConst(s) => {
                let id = self
                    .string_constants
                    .entry(s.clone())
                    .or_insert_with(|| {
                        let name = format!("str{}", self.next_string_id);
                        self.next_string_id += 1;
                        name
                    })
                    .clone();
                // TODO: Why is inbounds here
                format!(
                    "getelementptr inbounds ([{} x i8], [{} x i8]* @{}, i32 0, i32 0)",
                    s.len() + 1,
                    s.len() + 1,
                    id
                )
            }
            LIRValue::Null => "null".to_string(),
            _ => panic!("Unhandled case in format_value"),
        }
    }

    fn get_llvm_type_from_register(&self, reg_id: RegisterID) -> String {
        let func = self
            .current_function
            .as_ref()
            .expect("Current function not set");
        let reg_info = &func.registers[reg_id.0];
        self.llvm_type(&reg_info.typ)
    }

    fn get_class_name_from_register(&self, reg_id: RegisterID) -> String {
        let func = self.current_function.as_ref().expect("No current function");
        let reg_info = &func.registers[reg_id.0];

        if let TypeKind::Class(ident) = &reg_info.typ.kind {
            ident.name.to_string()
        } else {
            panic!(
                "Register %r{} is not a class type, it is {}",
                reg_id.0, reg_info.typ
            );
        }
    }
}
