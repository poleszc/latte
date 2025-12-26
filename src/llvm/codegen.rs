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
    next_unique_id: usize,
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
            next_unique_id: 0,
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
            writeln!(
                string_constants,
                "@{} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1",
                id, len, escaped
            )
            .unwrap();
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

    fn unique_id(&mut self) -> usize {
        let id = self.next_unique_id;
        self.next_unique_id += 1;
        id
    }
}

// ======================
// LLVM IR Emitters
// ======================
impl<'a> LLVMGenerator<'a> {
    fn emit_line(&mut self, content: impl AsRef<str>) {
        writeln!(self.output, "{}", content.as_ref()).unwrap();
    }

    fn emit_instr(&mut self, content: impl AsRef<str>) {
        writeln!(self.output, "    {}", content.as_ref()).unwrap();
    }

    fn emit_label(&mut self, label: impl AsRef<str>) {
        writeln!(self.output, "  {}:", label.as_ref()).unwrap();
    }

    fn emit_preambule(&mut self) {
        let decls = [
            "declare void @printInt(i32)",
            "declare void @printString(i8*)",
            "declare void @error()",
            "declare i32 @readInt()",
            "declare i8* @readString()",
            "declare i8* @stringConcat(i8*, i8*)",
            "declare i1 @stringEqual(i8*, i8*)",
            "declare i1 @stringNotEqual(i8*, i8*)",
            "declare i32 @stringLength(i8*)",
            "declare i8* @calloc(i64, i64)",
        ];
        for decl in decls {
            self.emit_line(decl);
        }
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
            .collect::<Vec<_>>()
            .join(", ");

        self.emit_line(format!("define {} @{}({}) {{", ret_type, func.name, args));

        for block_id in &func.block_order {
            if let Some(block) = func.blocks.get(block_id) {
                self.emit_block(*block_id, block);
            }
        }
        self.emit_line("}\n");
    }

    fn emit_class_definition(&mut self, class: &LIRClass) {
        let mut fields = vec!["i8**".to_string()];
        fields.extend(class.fields.iter().map(|f| self.llvm_type(&f.typ)));
        self.emit_line(format!(
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
            let arg_types: Vec<_> = func
                .parameter_types
                .iter()
                .map(|t| self.llvm_type(t))
                .collect();

            let func_signature = format!("{} ({})*", ret_type, arg_types.join(", "));
            methods.push(format!(
                "i8* bitcast ({} @{} to i8*)",
                func_signature, method_name
            ));
        }

        let len = methods.len().max(1);
        let content = if methods.is_empty() {
            "i8* null".to_string()
        } else {
            methods.join(", ")
        };
        self.emit_line(format!(
            "@vtable.{} = constant [{} x i8*] [ {} ]",
            class.name, len, content
        ));
    }

    fn emit_block(&mut self, block_id: BlockID, block: &LIRBlock) {
        self.current_block_split_count = 0;
        self.emit_label(format!("B{}", block_id.0));

        for instr in &block.instructions {
            self.emit_instruction(instr, block_id);
        }

        if let Some(term) = &block.terminator {
            self.emit_terminator(term);
        } else {
            panic!("NO TERMINATOR NOOOOOO: {}", block_id.0);
        }
    }
}

// ======================
// Instruction Lowering
// ======================
impl<'a> LLVMGenerator<'a> {
    fn emit_instruction(&mut self, instr: &LIRInstruction, block_id: BlockID) {
        match instr.operation {
            // TODO: Remove that in the future.
            LIROperation::Assign => {
                let dest = instr.destination.unwrap();
                let val = self.format_value(&instr.operands[0]);
                let typ = self.llvm_type_from_register(dest);
                self.emit_instr(format!("%r{} = bitcast {} {} to {}", dest.0, typ, val, typ));
            }
            LIROperation::IntAdd | LIROperation::IntSub | LIROperation::IntMul => {
                self.emit_arithmetic(instr);
            }
            LIROperation::IntDiv | LIROperation::IntMod => {
                self.emit_div_mod(instr, block_id);
            }
            LIROperation::IntNeg => {
                let dest = instr.destination.unwrap();
                let val = self.format_value(&instr.operands[0]);
                self.emit_instr(format!("%r{} = sub i32 0, {}", dest.0, val));
            }
            LIROperation::Phi => self.emit_phi(instr),
            LIROperation::IntLt
            | LIROperation::IntLe
            | LIROperation::IntGt
            | LIROperation::IntGe
            | LIROperation::IntEq
            | LIROperation::IntNeq => {
                self.emit_comparison(instr);
            }
            LIROperation::BoolAnd | LIROperation::BoolOr | LIROperation::BoolNot => {
                self.emit_boolean_logic(instr);
            }
            LIROperation::StringConcat
            | LIROperation::StringEq
            | LIROperation::StringNeq
            | LIROperation::StringLength => {
                self.emit_string_op(instr);
            }
            LIROperation::Call => self.emit_call(instr),
            LIROperation::VCall => self.emit_vcall(instr),
            LIROperation::ArrayGet | LIROperation::ArraySet | LIROperation::ArrayLength => {
                self.emit_array_op(instr, block_id);
            }
            LIROperation::NewArray => self.emit_new_array(instr),
            LIROperation::NewObject => self.emit_new_object(instr),
            LIROperation::FieldGet | LIROperation::FieldSet => {
                self.emit_field_op(instr);
            }
            LIROperation::IsNull => {
                let dest = instr.destination.unwrap();
                let val = self.format_value(&instr.operands[0]);
                self.emit_instr(format!("%r{} = icmp eq i8* {}, null", dest.0, val));
            }
        }
    }

    fn emit_arithmetic(&mut self, instr: &LIRInstruction) {
        let op = match instr.operation {
            LIROperation::IntAdd => "add",
            LIROperation::IntSub => "sub",
            _ => "mul",
        };
        let dest = instr.destination.unwrap();
        let lhs = self.format_value(&instr.operands[0]);
        let rhs = self.format_value(&instr.operands[1]);
        self.emit_instr(format!("%r{} = {} i32 {}, {}", dest.0, op, lhs, rhs));
    }

    fn emit_div_mod(&mut self, instr: &LIRInstruction, block_id: BlockID) {
        let dest = instr.destination.unwrap();
        let rhs = self.format_value(&instr.operands[1]);
        let split_id = self.current_block_split_count;
        self.current_block_split_count += 1;

        let err_label = format!("B{}_div_err_{}", block_id.0, split_id);
        let ok_label = format!("B{}_ok_{}", block_id.0, split_id);

        self.emit_instr(format!(
            "%cond_z_{}_{} = icmp eq i32 {}, 0",
            block_id.0, split_id, rhs
        ));
        self.emit_instr(format!(
            "br i1 %cond_z_{}_{}, label %{}, label %{}",
            block_id.0, split_id, err_label, ok_label
        ));

        self.emit_label(err_label);
        self.emit_instr("call void @error()");
        self.emit_instr("unreachable");

        self.emit_label(ok_label);
        let op = if instr.operation == LIROperation::IntDiv {
            "sdiv"
        } else {
            "srem"
        };
        let lhs = self.format_value(&instr.operands[0]);
        self.emit_instr(format!("%r{} = {} i32 {}, {}", dest.0, op, lhs, rhs));
    }

    fn emit_phi(&mut self, instr: &LIRInstruction) {
        let dest = instr.destination.unwrap();
        let llvm_type = self.llvm_type_from_register(dest);
        let mut args = Vec::new();
        for i in (0..instr.operands.len()).step_by(2) {
            let val = self.format_value(&instr.operands[i]);
            let b_id = match &instr.operands[i + 1] {
                LIRValue::Block(id) => id,
                _ => unreachable!(),
            };
            let last_label = self
                .lir_block_to_last_llvm_label
                .get(b_id)
                .expect("Block order error");
            args.push(format!("[ {}, %{} ]", val, last_label));
        }
        self.emit_instr(format!(
            "%r{} = phi {} {}",
            dest.0,
            llvm_type,
            args.join(", ")
        ));
    }

    fn emit_comparison(&mut self, instr: &LIRInstruction) {
        let cond = match instr.operation {
            LIROperation::IntLt => "slt",
            LIROperation::IntLe => "sle",
            LIROperation::IntGt => "sgt",
            LIROperation::IntGe => "sge",
            LIROperation::IntEq => "eq",
            LIROperation::IntNeq => "ne",
            _ => unreachable!(),
        };
        let dest = instr.destination.unwrap();
        let lhs = self.format_value(&instr.operands[0]);
        let rhs = self.format_value(&instr.operands[1]);
        let llvm_type = self.llvm_type_from_value(&instr.operands[0]);
        self.emit_instr(format!(
            "%r{} = icmp {} {} {}, {}",
            dest.0, cond, llvm_type, lhs, rhs
        ));
    }

    fn emit_boolean_logic(&mut self, instr: &LIRInstruction) {
        let dest = instr.destination.unwrap();
        match instr.operation {
            LIROperation::BoolAnd => {
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_instr(format!("%r{} = and i1 {}, {}", dest.0, lhs, rhs));
            }
            LIROperation::BoolOr => {
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_instr(format!("%r{} = or i1 {}, {}", dest.0, lhs, rhs));
            }
            LIROperation::BoolNot => {
                let val = self.format_value(&instr.operands[0]);
                self.emit_instr(format!("%r{} = xor i1 {}, 1", dest.0, val));
            }
            _ => unreachable!(),
        }
    }

    fn emit_string_op(&mut self, instr: &LIRInstruction) {
        let dest = instr.destination.unwrap();
        match instr.operation {
            LIROperation::StringConcat => {
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_instr(format!(
                    "%r{} = call i8* @stringConcat(i8* {}, i8* {})",
                    dest.0, lhs, rhs
                ));
            }
            LIROperation::StringEq => {
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_instr(format!(
                    "%r{} = call i1 @stringEqual(i8* {}, i8* {})",
                    dest.0, lhs, rhs
                ));
            }
            LIROperation::StringNeq => {
                let lhs = self.format_value(&instr.operands[0]);
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_instr(format!(
                    "%r{} = call i1 @stringNotEqual(i8* {}, i8* {})",
                    dest.0, lhs, rhs
                ));
            }
            LIROperation::StringLength => {
                let val = self.format_value(&instr.operands[0]);
                self.emit_instr(format!(
                    "%r{} = call i32 @stringLength(i8* {})",
                    dest.0, val
                ));
            }
            _ => unreachable!(),
        }
    }

    fn emit_call(&mut self, instr: &LIRInstruction) {
        let func_name = match &instr.operands[0] {
            LIRValue::Function(name) => name,
            _ => unreachable!(),
        };
        let args: Vec<_> = instr.operands[1..]
            .iter()
            .map(|v| format!("{} {}", self.llvm_type_from_value(v), self.format_value(v)))
            .collect();
        let args_str = args.join(", ");

        if let Some(dest) = instr.destination {
            let ret_type = self.llvm_type_from_register(dest);
            self.emit_instr(format!(
                "%r{} = call {} @{}({})",
                dest.0, ret_type, func_name, args_str
            ));
        } else {
            self.emit_instr(format!("call void @{}({})", func_name, args_str));
        }
    }

    fn emit_vcall(&mut self, instr: &LIRInstruction) {
        let dest = instr.destination;
        let obj = &instr.operands[0];
        let obj_val = self.format_value(obj);
        let obj_reg = match obj {
            LIRValue::Register(r) => *r,
            _ => unreachable!(),
        };
        let method_idx = match &instr.operands[1] {
            LIRValue::IntConst(i) => *i,
            _ => unreachable!(),
        };
        let class_name = self.get_class_name_from_register(obj_reg);

        let id = self.unique_id();

        // TODO: Check for null ptr access.
        // Get the VTable pointer from the zeroth field.
        let vptr_ptr = format!("%vptr_ptr_vcall_{}", id);
        let vptr = format!("%vptr_vcall_{}", id);
        self.emit_instr(format!(
            "{} = getelementptr %class.{}, %class.{}* {}, i32 0, i32 0",
            vptr_ptr, class_name, class_name, obj_val
        ));
        self.emit_instr(format!("{} = load i8**, i8*** {}", vptr, vptr_ptr));

        // Get the function pointer
        let method_ptr_ptr = format!("%method_ptr_ptr_{}", id);
        let method_ptr_raw = format!("%method_ptr_raw_{}", id);
        self.emit_instr(format!(
            "{} = getelementptr i8*, i8** {}, i64 {}",
            method_ptr_ptr, vptr, method_idx
        ));
        self.emit_instr(format!(
            "{} = load i8*, i8** {}",
            method_ptr_raw, method_ptr_ptr
        ));

        // Build the methods signature from register types. TODO: This is stupid.
        let ret_type = dest.map_or("void".to_string(), |r| self.llvm_type_from_register(r));
        let mut arg_types = vec![format!("%class.{}*", class_name)];
        arg_types.extend(
            instr.operands[2..]
                .iter()
                .map(|arg| self.llvm_type_from_value(arg)),
        );
        let func_type = format!("{} ({})*", ret_type, arg_types.join(", "));

        // Cast the pointer to the expected signature.
        let method_ptr = format!("%method_ptr_{}", id);
        self.emit_instr(format!(
            "{} = bitcast i8* {} to {}",
            method_ptr, method_ptr_raw, func_type
        ));

        let mut call_args = vec![format!("%class.{}* {}", class_name, obj_val)];
        call_args.extend(
            instr.operands[2..]
                .iter()
                .map(|arg| format!("{} {}", self.llvm_type_from_value(arg), self.format_value(arg))),
        );

        let call_prefix = dest.map_or("".to_string(), |r| format!("%r{} = ", r.0));
        self.emit_instr(format!(
            "{}call {} {}({})",
            call_prefix,
            ret_type,
            method_ptr,
            call_args.join(", ")
        ));
    }

    fn emit_array_op(&mut self, instr: &LIRInstruction, block_id: BlockID) {
        match instr.operation {
            LIROperation::ArrayGet => {
                let dest = instr.destination.unwrap();
                let arr = self.format_value(&instr.operands[0]);
                let idx = self.format_value(&instr.operands[1]);
                let elem_type = self.llvm_type_from_register(dest);
                let id = self.unique_id();

                self.emit_array_bounds_check(&arr, &idx, &block_id);

                let data_raw = format!("%d_raw_{}", id);
                let data_ptr = format!("%d_ptr_{}", id);
                let elem_ptr = format!("%e_ptr_{}", id);

                self.emit_instr(format!(
                    "{} = getelementptr i8, i8* {}, i32 8",
                    data_raw, arr
                ));
                self.emit_instr(format!(
                    "{} = bitcast i8* {} to {}*",
                    data_ptr, data_raw, elem_type
                ));
                self.emit_instr(format!(
                    "{} = getelementptr {}, {}* {}, i32 {}",
                    elem_ptr, elem_type, elem_type, data_ptr, idx
                ));
                self.emit_instr(format!(
                    "%r{} = load {}, {}* {}",
                    dest.0, elem_type, elem_type, elem_ptr
                ));
            }
            LIROperation::ArraySet => {
                let arr = self.format_value(&instr.operands[0]);
                let idx = self.format_value(&instr.operands[1]);
                let val = self.format_value(&instr.operands[2]);
                let element_typ = self.llvm_type_from_value(&instr.operands[2]);
                // TODO: Get rid of that
                let id = self.next_unique_id;
                self.next_unique_id += 1;

                self.emit_array_bounds_check(&arr, &idx, &block_id);

                let data_raw = format!("%data_raw_{}", id);
                let data_ptr = format!("%data_ptr_{}", id);
                let elem_ptr = format!("%elem_ptr_{}", id);

                self.emit_instr(format!(
                    "{} = getelementptr i8, i8* {}, i32 8",
                    data_raw, arr
                ));
                self.emit_instr(format!(
                    "{} = bitcast i8* {} to {}*",
                    data_ptr, data_raw, element_typ
                ));
                self.emit_instr(format!(
                    "{} = getelementptr {}, {}* {}, i32 {}",
                    elem_ptr, element_typ, element_typ, data_ptr, idx
                ));
                self.emit_instr(format!(
                    "store {} {}, {}* {}",
                    element_typ, val, element_typ, elem_ptr
                ));
            }
            LIROperation::ArrayLength => {
                let dest = instr.destination.unwrap();
                let arr = self.format_value(&instr.operands[0]);
                let len_ptr = format!("%len_ptr_{}", dest.0);
                self.emit_instr(format!("{} = bitcast i8* {} to i32*", len_ptr, arr));
                self.emit_instr(format!("%r{} = load i32, i32* {}", dest.0, len_ptr));
            }
            _ => unreachable!(),
        }
    }

    fn emit_new_array(&mut self, instr: &LIRInstruction) {
        let dest = instr.destination.unwrap();
        let count = self.format_value(&instr.operands[0]);
        let reg_info = &self.current_function.unwrap().registers[dest.0];
        let elem_type = match &reg_info.typ.kind {
            TypeKind::Array(inner) => inner,
            _ => unreachable!(),
        };
        let elem_size = self.llvm_sizeof_type(elem_type);
        let id = dest.0;

        let element_count = format!("%element_count_{}", id);
        self.emit_instr(format!("{} = sext i32 {} to i64", element_count, count));
        let data_size = format!("%data_size_{}", id);
        self.emit_instr(format!(
            "{} = mul i64 {}, {}",
            data_size, element_count, elem_size
        ));
        let total_size = format!("%total_size_{}", id);
        self.emit_instr(format!("{} = add i64 {}, 8", total_size, data_size));

        let raw_ptr = format!("%raw_arr_{}", id);
        self.emit_instr(format!(
            "{} = call i8* @calloc(i64 1, i64 {})",
            raw_ptr, total_size
        ));

        let len_ptr = format!("%len_ptr_{}", id);
        self.emit_instr(format!("{} = bitcast i8* {} to i32*", len_ptr, raw_ptr));
        self.emit_instr(format!("store i32 {}, i32* {}", count, len_ptr));
        self.emit_instr(format!("%r{} = bitcast i8* {} to i8*", id, raw_ptr));
    }

    fn emit_new_object(&mut self, instr: &LIRInstruction) {
        let dest = instr.destination.unwrap();
        let class_name = match &instr.operands[0] {
            LIRValue::Class(c) => c,
            _ => unreachable!(),
        };
        let id = dest.0;

        // Calculate the size of the object to allocate.
        let size_tmp = format!("%size_temp_{}", id);
        let size_int = format!("%size_{}", id);
        self.emit_instr(format!(
            "{} = getelementptr %class.{}, %class.{}* null, i32 1",
            size_tmp, class_name, class_name
        ));
        self.emit_instr(format!(
            "{} = ptrtoint %class.{}* {} to i64",
            size_int, class_name, size_tmp
        ));

        // Actually allocate the object.
        let raw_ptr = format!("%raw_obj_{}", id);
        self.emit_instr(format!(
            "{} = call i8* @calloc(i64 1, i64 {})",
            raw_ptr, size_int
        ));
        self.emit_instr(format!(
            "%r{} = bitcast i8* {} to %class.{}*",
            id, raw_ptr, class_name
        ));

        let vptr_ptr = format!("%vptr_ptr_{}", id);
        self.emit_instr(format!(
            "{} = getelementptr %class.{}, %class.{}* %r{}, i32 0, i32 0",
            vptr_ptr, class_name, class_name, id
        ));
        let vtable_gep = format!(
            "getelementptr ([{} x i8*], [{} x i8*]* @vtable.{}, i32 0, i32 0)",
            class_name.len(),
            class_name.len(),
            class_name
        );
        self.emit_instr(format!("store i8** {}, i8*** {}", vtable_gep, vptr_ptr));
    }

    fn emit_field_op(&mut self, instr: &LIRInstruction) {
        let obj_val = self.format_value(&instr.operands[0]);
        let obj_reg = match &instr.operands[0] {
            LIRValue::Register(r) => *r,
            _ => unreachable!(),
        };
        let class_name = self.get_class_name_from_register(obj_reg);
        let field_idx = match &instr.operands[1] {
            LIRValue::IntConst(i) => i + 1,
            _ => unreachable!(),
        };

        match instr.operation {
            LIROperation::FieldGet => {
                let dest = instr.destination.unwrap();
                let field_ptr = format!("%field_ptr_{}", dest.0);
                let field_type = self.llvm_type_from_register(dest);
                self.emit_instr(format!(
                    "{} = getelementptr %class.{}, %class.{}* {}, i32 0, i32 {}",
                    field_ptr, class_name, class_name, obj_val, field_idx
                ));
                self.emit_instr(format!(
                    "%r{} = load {}, {}* {}",
                    dest.0, field_type, field_type, field_ptr
                ));
            }
            LIROperation::FieldSet => {
                let val = self.format_value(&instr.operands[2]);
                let val_type = self.llvm_type_from_value(&instr.operands[2]);
                let id = self.unique_id();

                let field_ptr = format!("%field_set_ptr_{}", id);
                self.emit_instr(format!(
                    "{} = getelementptr %class.{}, %class.{}* {}, i32 0, i32 {}",
                    field_ptr, class_name, class_name, obj_val, field_idx
                ));
                self.emit_instr(format!(
                    "store {} {}, {}* {}",
                    val_type, val, val_type, field_ptr
                ));
            }
            _ => unreachable!(),
        }
    }

    fn emit_terminator(&mut self, term: &LIRTerminator) {
        match term {
            LIRTerminator::Jump { target, .. } => {
                self.emit_instr(format!("br label %B{}", target.0));
            }
            LIRTerminator::Branch {
                condition,
                if_true,
                if_false,
                ..
            } => {
                let cond = self.format_value(condition);
                self.emit_instr(format!(
                    "br i1 {}, label %B{}, label %B{}",
                    cond, if_true.0, if_false.0
                ));
            }
            LIRTerminator::Return { value, .. } => {
                if let Some(val) = value {
                    let formatted_val = self.format_value(val);
                    let val_type = self.llvm_type_from_value(val);
                    self.emit_instr(format!("ret {} {}", val_type, formatted_val));
                } else {
                    self.emit_instr("ret void");
                }
            }
            LIRTerminator::Unreachable { .. } => {
                self.emit_instr("unreachable");
            }
        }
    }

    fn emit_array_bounds_check(&mut self, arr_ptr: &str, index_val: &str, block_id: &BlockID) {
        let split_id = self.current_block_split_count;
        self.current_block_split_count += 1;
        let id = self.unique_id();

        let len_ptr = format!("%len_ptr_check_{}", id);
        let len_val = format!("%len_val_check_{}", id);
        let cond_neg = format!("%cond_neg_{}", id);
        let cond_ge = format!("%cond_ge_{}", id);
        let cond_out = format!("%cond_final_{}", id);
        let err_label = format!("B{}_err_{}", block_id.0, split_id);
        let ok_label = format!("B{}_ok_{}", block_id.0, split_id);

        // Retrieve the array size.
        self.emit_instr(format!("{} = bitcast i8* {} to i32*", len_ptr, arr_ptr));
        self.emit_instr(format!("{} = load i32, i32* {}", len_val, len_ptr));
        // index >= 0
        self.emit_instr(format!("{} = icmp slt i32 {}, 0", cond_neg, index_val));
        // index < length
        self.emit_instr(format!(
            "{} = icmp sge i32 {}, {}",
            cond_ge, index_val, len_val
        ));
        self.emit_instr(format!("{} = or i1 {}, {}", cond_out, cond_neg, cond_ge));
        self.emit_instr(format!(
            "br i1 {}, label %{}, label %{}",
            cond_out, err_label, ok_label
        ));

        self.emit_label(err_label);
        self.emit_instr("call void @error()");
        self.emit_instr("unreachable");
        self.emit_label(ok_label);
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
            TypeKind::Array(_) | TypeKind::String => "i8*".to_string(),
            _ => panic!("Bad type in LLVM backend"),
        }
    }

    fn llvm_type_from_value(&self, val: &LIRValue) -> String {
        match val {
            LIRValue::IntConst(_) => "i32".to_string(),
            LIRValue::BoolConst(_) => "i1".to_string(),
            LIRValue::StringConst(_) => "i8*".to_string(),
            LIRValue::Null => "i8*".to_string(),
            LIRValue::Register(id) => self.llvm_type_from_register(*id),
            LIRValue::Class(name) => format!("%class.{}*", name),
            _ => "i8*".to_string(),
        }
    }

    fn llvm_sizeof_type(&self, typ: &Type) -> usize {
        match &typ.kind {
            TypeKind::Int => 4,
            TypeKind::Bool => 1,
            TypeKind::String | TypeKind::Class(_) | TypeKind::Array(_) => 8, // Size of pointer
            _ => panic!("Unhanded type for size"),
        }
    }

    fn format_value(&mut self, val: &LIRValue) -> String {
        match val {
            LIRValue::Register(id) => format!("%r{}", id.0),
            LIRValue::IntConst(n) => n.to_string(),
            LIRValue::BoolConst(b) => if *b { "1" } else { "0" }.to_string(),
            LIRValue::StringConst(s) => {
                let id = self.unique_id();
                let string_id = self
                    .string_constants
                    .entry(s.clone())
                    .or_insert_with(|| {
                        let name = format!("str{}", id);
                        name
                    })
                    .clone();
                // TODO: Why is inbounds here?
                let len = s.len() + 1; // \0
                format!(
                    "getelementptr inbounds ([{} x i8], [{} x i8]* @{}, i32 0, i32 0)",
                    len, len, string_id
                )
            }
            LIRValue::Null => "null".to_string(),
            _ => panic!("Unhandled value: {:?}", val),
        }
    }

    fn llvm_type_from_register(&self, reg_id: RegisterID) -> String {
        let func = self.current_function.as_ref().expect("No current function");
        self.llvm_type(&func.registers[reg_id.0].typ)
    }

    fn get_class_name_from_register(&self, reg_id: RegisterID) -> String {
        let func = self.current_function.as_ref().expect("No current function");
        if let TypeKind::Class(ident) = &func.registers[reg_id.0].typ.kind {
            ident.name.to_string()
        } else {
            panic!("Register %r{} is not a class", reg_id.0);
        }
    }
}
