use super::*;
use std::fmt;

impl fmt::Display for RegisterID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

impl fmt::Display for BlockID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "B{}", self.0)
    }
}

impl fmt::Display for LIRValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LIRValue::Register(id) => write!(f, "{}", id),
            LIRValue::IntConst(v) => write!(f, "{}", v),
            LIRValue::BoolConst(v) => write!(f, "{}", v),
            LIRValue::StringConst(s) => write!(f, "\"{}\"", s.escape_default()),
            LIRValue::Null => write!(f, "null"),
            LIRValue::Function(name) => write!(f, "@{}", name),
            LIRValue::Class(name) => write!(f, "class.{}", name),
            LIRValue::Block(block_id) => write!(f, "label B{}", block_id)
        }
    }
}

impl fmt::Display for LIROperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            LIROperation::Phi => "Phi",
            LIROperation::IntAdd => "Add",
            LIROperation::IntSub => "Sub",
            LIROperation::IntMul => "Mul",
            LIROperation::IntDiv => "Div",
            LIROperation::IntMod => "Mod",
            LIROperation::IntNeg => "Neg",
            LIROperation::IntLt => "Lt",
            LIROperation::IntLe => "Le",
            LIROperation::IntGt => "Gt",
            LIROperation::IntGe => "Ge",
            LIROperation::IntEq => "Eq",
            LIROperation::IntNeq => "Neq",
            LIROperation::BoolNot => "Not",
            LIROperation::BoolAnd => "And",
            LIROperation::BoolOr => "Or",
            LIROperation::StringConcat => "Strcat",
            LIROperation::StringEq => "Streq",
            LIROperation::StringNeq => "Strneq",
            LIROperation::StringLength => "Strlen",
            LIROperation::Call => "Call",
            LIROperation::VCall => "VCall",
            LIROperation::NewArray => "NewArray",
            LIROperation::ArrayGet => "ArrGet",
            LIROperation::ArraySet => "ArrSet",
            LIROperation::ArrayLength => "ArrLen",
            LIROperation::NewObject => "NewObj",
            LIROperation::FieldGet => "getField",
            LIROperation::FieldSet => "SetField",
            LIROperation::IsNull => "IsNull",
            LIROperation::Assign => "Assign",
        };
        write!(f, "{:<12}", name)
    }
}

impl fmt::Display for LIRInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "  {} = {} ", self.destination, self.operation)?;

        if self.operation == LIROperation::Phi {
            write!(f, "[")?;
            for (i, chunk) in self.operands.chunks(2).enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                if chunk.len() == 2 {
                    write!(f, "[{}: {}]", chunk[0], chunk[1])?;
                }
            }
            write!(f, "]")?;
        } else {
            for (i, operand) in self.operands.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", operand)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for LIRTerminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LIRTerminator::Jump { target, .. } => {
                write!(f, "  jump {}", target)
            }
            LIRTerminator::Branch {
                condition,
                if_true,
                if_false,
                ..
            } => {
                write!(f, "  br {}, {}, {}", condition, if_true, if_false)
            }
            LIRTerminator::Return { value: Some(v), .. } => {
                write!(f, "  ret {}", v)
            }
            LIRTerminator::Return { value: None, .. } => {
                write!(f, "  ret void")
            }
        }
    }
}

impl fmt::Display for RegisterInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.id, self.typ)?;
        if let Some(name) = self.name {
            write!(f, " ({})", name)?;
        }
        Ok(())
    }
}

impl fmt::Display for LIRBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {{", self.id)?;

        if !self.predecessors.is_empty() {
            write!(f, " ; Predecessors = ")?;
            for (i, pred) in self.predecessors.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", pred)?;
            }
        }
        writeln!(f)?;

        for instr in &self.instructions {
            writeln!(f, "  {}", instr)?;
        }

        if let Some(term) = &self.terminator {
            writeln!(f, "  {}", term)?;
        }
        writeln!(f, "}}")?;

        Ok(())
    }
}

impl fmt::Display for LIRFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "define {} @{}(", self.return_type, self.name)?;

        for (i, (typ, reg)) in self
            .parameter_types
            .iter()
            .zip(&self.parameter_registers)
            .enumerate()
        {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{} {}", typ, reg)?;
        }

        writeln!(f, ") {{")?;

        if !self.registers.is_empty() {
            writeln!(f, "  ; Registers:")?;
            for reg in &self.registers {
                writeln!(f, "  ; {}", reg)?;
            }
            writeln!(f)?;
        }

        for block_id in &self.block_order {
            if let Some(block) = self.blocks.get(block_id) {
                writeln!(f, "{}", block)?;
            }
        }

        writeln!(f, "}}")?;

        Ok(())
    }
}

impl fmt::Display for LIRClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "class {}", self.name)?;

        if let Some(parent) = self.parent {
            write!(f, " extends {}", parent)?;
        }

        writeln!(f, " {{")?;

        if !self.fields.is_empty() {
            writeln!(f, "  ; Fields:")?;
            for field in &self.fields {
                writeln!(f, "  ;   [{}] {}: {}", field.index, field.name, field.typ)?;
            }
        }

        if !self.vtable.is_empty() {
            writeln!(f, "  ; VTable:")?;
            for (i, method) in self.vtable.iter().enumerate() {
                writeln!(f, "  ;   [{}] {}", i, method)?;
            }
        }

        writeln!(f, "}}")?;

        Ok(())
    }
}

impl fmt::Display for LIRProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "╭───────────────────────────────╮")?;
        writeln!(f, "│          LIR Program          │")?;
        writeln!(f, "╰───────────────────────────────╯")?;
        writeln!(f)?;

        if !self.classes.is_empty() {
            writeln!(f, "; ========== Classes ==========")?;
            writeln!(f)?;

            for class in self.classes.values() {
                writeln!(f, "{}", class)?;
            }
        }

        if !self.functions.is_empty() {
            writeln!(f, "; ========== Functions ==========")?;
            writeln!(f)?;

            for func_name in &self.function_order {
                if let Some(func) = self.functions.get(func_name) {
                    writeln!(f, "{}", func)?;
                }
            }
        }

        Ok(())
    }
}

impl LIRProgram {
    pub fn dump_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;
        write!(file, "{}", self)?;

        Ok(())
    }

    pub fn dump_to_stderr(&self) {
        eprintln!("{}", self);
    }
}
