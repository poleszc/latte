use super::*;
use core::fmt;

impl fmt::Display for MIRValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MIRValue::Unit => write!(f, "()"),
            MIRValue::IntConst(val) => write!(f, "{}", val),
            MIRValue::BoolConst(val) => write!(f, "{}", val),
            MIRValue::StringConst(s) => write!(f, "\"{}\"", s.escape_default()),
            MIRValue::Null => write!(f, "null"),
            MIRValue::Local(id) => write!(f, "Local({})", id.0),
            MIRValue::Temp(id) => write!(f, "Temp({})", id.0),
            MIRValue::Block(id) => write!(f, "Block({})", id.0),
            MIRValue::Function(name) => write!(f, "Function({})", name),
            MIRValue::Class(name) => write!(f, "Class({})", name),
        }
    }
}

impl fmt::Display for MIRPlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MIRPlace::Local(id) => write!(f, "Local({})", id.0),
            MIRPlace::Temp(id) => write!(f, "Temp({}){:>1}", id.0, ""),
            MIRPlace::Field {
                base,
                field_name,
                field_index,
                ..
            } => write!(f, "{}.{}[{}]", base, field_name, field_index),
            MIRPlace::Index { base, index } => write!(f, "{}[{}]", base, index),
        }
    }
}

impl fmt::Display for MIROperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            MIROperation::Assign => "Assign",
            MIROperation::IntAdd => "IntAdd",
            MIROperation::IntSub => "IntSub",
            MIROperation::IntMul => "IntMul",
            MIROperation::IntDiv => "IntDiv",
            MIROperation::IntMod => "IntMod",
            MIROperation::IntNeg => "IntNeg",
            MIROperation::IntLt => "IntLt",
            MIROperation::IntLe => "IntLe",
            MIROperation::IntGt => "IntGt",
            MIROperation::IntGe => "IntGe",
            MIROperation::IntEq => "IntEq",
            MIROperation::IntNeq => "IntNeq",
            MIROperation::BoolAnd => "BoolAnd",
            MIROperation::BoolOr => "BoolOr",
            MIROperation::BoolNot => "BoolNot",
            MIROperation::StringConcat => "StringConcat",
            MIROperation::StringEq => "StringEq",
            MIROperation::StringNeq => "StringNeq",
            MIROperation::StringLength => "StringLength",
            MIROperation::Call => "Call",
            MIROperation::VCall => "VCall",
            MIROperation::NewArray => "NewArray",
            MIROperation::ArrayGet => "ArrayGet",
            MIROperation::ArraySet => "ArraySet",
            MIROperation::ArrayLength => "ArrayLength",
            MIROperation::NewObject => "NewObject",
            MIROperation::FieldGet => "FieldGet",
            MIROperation::FieldSet => "FieldSet",
            MIROperation::IsNull => "IsNull",
        };
        write!(f, "{:<18}", name)
    }
}

impl fmt::Display for MIRInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(dest) = &self.destination {
            write!(f, "{} := ", dest)?;
        } else {
            write!(f, "{:>8}    ", "")?;
        }

        write!(f, "{}", self.operation)?;

        for (i, arg) in self.operands.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }

        Ok(())
    }
}

impl fmt::Display for MIRTerminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MIRTerminator::Jump { target, .. } => {
                write!(f, "{:>8}    Jump              -> Block({})", "", target.0)
            }
            MIRTerminator::Branch {
                condition,
                if_true,
                if_false,
                ..
            } => write!(
                f,
                "{:>8}    Branch            {} -> Block({}), Block({})",
                "", condition, if_true.0, if_false.0
            ),
            MIRTerminator::Return {
                value: Some(val), ..
            } => {
                write!(f, "{:>8}    Return            {}", "", val)
            }
            MIRTerminator::Return { value: None, .. } => {
                write!(f, "{:>8}    ReturnVoid", "")
            }
        }
    }
}

impl fmt::Display for LocalInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Local({}): ", self.id.0)?;

        if let Some(name) = self.name {
            write!(f, "Name: {}, ", name)?;
        } else {
            write!(f, "Name: <temp>, ")?;
        }

        write!(f, "Type: {}", self.typ)?;

        if self.is_parameter
            && let Some(idx) = self.parameter_index
        {
            write!(f, ", Parameter #{}", idx)?;
        }

        Ok(())
    }
}

impl fmt::Display for TempInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Temp({}): Type: {}", self.id.0, self.typ)
    }
}

impl fmt::Display for MIRBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            " ========================================================="
        )?;
        writeln!(f, "  Block {} [entry]:", self.id.0)?;
        for instr in &self.instructions {
            writeln!(f, "    {}", instr)?;
        }
        if let Some(terminator) = &self.terminator {
            writeln!(f, "    {}", terminator)?;
        } else {
            writeln!(f, "    NONE")?;
        }

        Ok(())
    }
}

impl fmt::Display for MIRFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[MIR] Function {}: (", self.name)?;

        for (i, param_type) in self.parameter_types.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param_type)?;
        }

        write!(f, ") -> {}", self.return_type)?;
        writeln!(f)?;

        if !self.locals.is_empty() {
            writeln!(f, "  Locals:")?;
            for local in &self.locals {
                writeln!(f, "    {}", local)?;
            }
        }

        if !self.temps.is_empty() {
            writeln!(f, "  Temporaries:")?;
            for temp in &self.temps {
                writeln!(f, "    {}", temp)?;
            }
        }

        writeln!(f, "{{")?;

        for block_id in &self.block_order {
            if let Some(block) = self.blocks.get(block_id) {
                writeln!(f, "{}", block)?;
            }
        }

        writeln!(f, "}}")?;
        writeln!(f)?;

        Ok(())
    }
}

impl fmt::Display for MIRClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[MIR] Class {}", self.name)?;

        if let Some(parent) = self.parent {
            write!(f, " extends {}", parent)?;
        }

        writeln!(f, " {{")?;

        if !self.fields.is_empty() {
            writeln!(f, "  Fields:")?;
            for field in &self.fields {
                writeln!(f, "    [{}] {}: {}", field.index, field.name, field.typ)?;
            }
        }

        if !self.methods.is_empty() {
            writeln!(f, "  Methods:")?;
            for method in &self.methods {
                writeln!(f, "    {}", method)?;
            }
        }

        if !self.vtable.is_empty() {
            writeln!(f, "  VTable:")?;
            for (i, method) in self.vtable.iter().enumerate() {
                writeln!(f, "    [{}] {}", i, method)?;
            }
        }

        writeln!(f, "}}")?;
        writeln!(f)?;

        Ok(())
    }
}

impl fmt::Display for MIRProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "╭───────────────────────────────╮")?;
        writeln!(f, "│          MIR Program          │")?;
        writeln!(f, "╰───────────────────────────────╯")?;
        writeln!(f)?;

        if !self.classes.is_empty() {
            writeln!(f, "-------------------------------------------")?;
            writeln!(f, "                 CLASSES                   ")?;
            writeln!(f, "-------------------------------------------")?;
            writeln!(f)?;

            for class in self.classes.values() {
                writeln!(f, "{}", class)?;
            }
        }

        if !self.functions.is_empty() {
            writeln!(f, "-------------------------------------------")?;
            writeln!(f, "                FUNCTIONS                  ")?;
            writeln!(f, "-------------------------------------------")?;
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

impl MIRProgram {
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

impl MIRFunction {
    pub fn dump_to_stderr(&self) {
        eprintln!("{}", self);
    }

    pub fn to_string_pretty(&self) -> String {
        format!("{}", self)
    }
}

impl MIRProgram {
    pub fn dump_summary(&self) {
        if !self.classes.is_empty() {
            println!("Classes ({}): ", self.classes.len());
            for (name, class) in &self.classes {
                let parent_str = class
                    .parent
                    .map(|p| format!(" extends {}", p))
                    .unwrap_or_default();
                println!(
                    "  {} {} fields, {} methods{}",
                    name,
                    class.fields.len(),
                    class.methods.len(),
                    parent_str
                );
            }
            println!();
        }

        if !self.functions.is_empty() {
            println!("Functions ({}): ", self.function_order.len());
            for func_name in &self.function_order {
                if let Some(func) = self.functions.get(func_name) {
                    println!(
                        "  {}: {} params, {} locals, {} temps, {} blocks",
                        func_name,
                        func.parameter_types.len(),
                        func.locals.len(),
                        func.temps.len(),
                        func.blocks.len()
                    );
                }
            }
        }
    }
}
