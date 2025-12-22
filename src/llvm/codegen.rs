use core::panic;
use std::{collections::HashMap, fmt::Write};

use crate::{
    lir::{
        LIRFunction, LIRInstruction, LIROperation, LIRProgram, LIRTerminator, LIRValue, RegisterID,
    },
    parser::{Type, TypeKind},
};

pub struct LLVMGenerator<'a> {
    output: String,
    string_constants: HashMap<String, String>,
    next_string_id: usize,
    current_function: Option<&'a LIRFunction>,
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
        }
    }

    pub fn generate_program(mut self, program: &'a LIRProgram) -> String {
        // Emit builtins.
        self.emit_preambule();

        for func_name in &program.function_order {
            if let Some(func) = program.functions.get(func_name) {
                self.current_function = Some(func);
                self.emit_function(func);
                self.current_function = None;
            }
        }

        // TODO: Emit string constants.
        // let mut final_output = String::new();
        // for (string, id) in &self.string_constants {}

        self.output
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
        self.emit_line("");
    }

    fn emit_function(&mut self, func: &LIRFunction) {
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
                self.emit_line_indented(format!("B{}:", block_id.0).as_str());

                for instr in &block.instructions {
                    self.emit_instruction(instr);
                }

                if let Some(term) = &block.terminator {
                    self.emit_terminator(term);
                } else {
                    panic!("NO TERMINATOR NOOOOOO")
                }
            }
        }
        self.emit_line("}\n");
    }

    fn emit_instruction(&mut self, instr: &LIRInstruction) {
        match instr.operation {
            // TODO: this should be removed in the future.
            LIROperation::Assign => {
                let dest = instr.destination.unwrap();
                let val = self.format_value(&instr.operands[0]);
                let typ = self.get_llvm_type_from_register(dest);
                self.emit_line_very_indented(
                    format!("%r{} = add {} 0, {}", dest.0, typ, val).as_str(),
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
                let rhs = self.format_value(&instr.operands[1]);
                self.emit_line_very_indented(
                    format!(
                        "%cond_z_{} = icmp eq i32 {}, 0",
                        instr.destination.unwrap().0,
                        rhs
                    )
                    .as_str(),
                );
                self.emit_line_very_indented(
                    format!(
                        "br i1 %cond_z_{}, label %err_{}, label %ok_{}",
                        instr.destination.unwrap().0,
                        instr.destination.unwrap().0,
                        instr.destination.unwrap().0
                    )
                    .as_str(),
                );
                self.emit_line_indented(format!("err_{}:", instr.destination.unwrap().0).as_str());
                self.emit_line_very_indented("call void @error()");
                self.emit_line_very_indented("unreachable");
                self.emit_line_indented(format!("ok_{}:", instr.destination.unwrap().0).as_str());

                let op = if instr.operation == LIROperation::IntDiv {
                    "sdiv"
                } else {
                    "srem"
                };
                let lhs = self.format_value(&instr.operands[0]);
                self.emit_line_very_indented(
                    format!(
                        "%r{} = {} i32 {}, {}",
                        instr.destination.unwrap().0,
                        op,
                        lhs,
                        rhs
                    )
                    .as_str(),
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
                        LIRValue::Block(id) => id.0,
                        _ => unreachable!(),
                    };
                    args.push(format!("[ {}, %B{} ]", val, block_id));
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
                let v1 = self.format_value(&instr.operands[0]);
                let v2 = self.format_value(&instr.operands[1]);
                self.emit_line_very_indented(
                    format!("%r{} = icmp {} i32 {}, {}", dest.0, cond, v1, v2).as_str(),
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
                    .map(|v| format!("i32 {}", self.format_value(v)))
                    .collect::<Vec<String>>()
                    .join(", ");

                if let Some(dest) = instr.destination {
                    self.emit_line_very_indented(
                        format!("%r{} = call i32 @{}({})", dest.0, func_name, args).as_str(),
                    );
                } else {
                    self.emit_line_very_indented(
                        format!("call void @{}({})", func_name, args).as_str(),
                    );
                }
            }
            LIROperation::VCall => todo!(),
            LIROperation::NewArray => todo!(),
            LIROperation::ArrayGet => todo!(),
            LIROperation::ArraySet => todo!(),
            LIROperation::ArrayLength => todo!(),
            LIROperation::NewObject => todo!(),
            LIROperation::FieldGet => todo!(),
            LIROperation::FieldSet => todo!(),
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
                    self.emit_line_very_indented(format!("ret i32 {}", formatted_val).as_str());
                } else {
                    self.emit_line_very_indented("ret void");
                }
            }
        }
    }
}
impl<'a> LLVMGenerator<'a> {
    fn llvm_type(&self, typ: &Type) -> String {
        match &typ.kind {
            TypeKind::Int => "i32".to_string(),
            TypeKind::Bool => "i1".to_string(),
            TypeKind::Void => "void".to_string(),
            TypeKind::Error => {
                panic!("Something went very wrong, Error type in LLVM backend!")
            }
            crate::parser::TypeKind::Fun(_, _) => panic!("Function type in llvm"),
            _ => "i8*".to_string(),
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
}
