/// An abstraction over the Jasmine instructions.
pub type RegisterName = String;
pub type PointerName = String;

#[derive(Debug)]
pub enum LLVMInstruction {
    Add {
        location: RegisterName,
        left: RegisterName,
        right: RegisterName,
    },
    Sub {
        location: RegisterName,
        left: RegisterName,
        right: RegisterName,
    },
    Div {
        location: RegisterName,
        left: RegisterName,
        right: RegisterName,
    },
    Mul {
        location: RegisterName,
        left: RegisterName,
        right: RegisterName,
    },
    Load {
        destination: RegisterName,
        source: PointerName,
    },
    Store {
        value: RegisterName,
        destination: PointerName,
    },
    Alloca {
        destination: PointerName,
    },
    Ret {
        value: RegisterName,
    },
    CallPrint {
        value: RegisterName,
    },
}
impl LLVMInstruction {
    pub fn stringify(&self) -> String {
        match self {
            LLVMInstruction::Add {
                location,
                left,
                right,
            } => format!("{} = add i32 {}, {}", location, left, right),
            LLVMInstruction::Sub {
                location,
                left,
                right,
            } => format!("{} = sub i32 {}, {}", location, left, right),
            LLVMInstruction::Mul {
                location,
                left,
                right,
            } => format!("{} = mul i32 {}, {}", location, left, right),
            LLVMInstruction::Div {
                location,
                left,
                right,
            } => format!("{} = sdiv i32 {}, {}", location, left, right),
            LLVMInstruction::Load {
                destination,
                source,
            } => format!("{} = load i32, ptr {}", destination, source),
            LLVMInstruction::Store { value, destination } => {
                format!("store i32 {}, ptr {}", value, destination)
            }
            LLVMInstruction::Alloca { destination } => {
                format!("{} = alloca i32", destination)
            }
            LLVMInstruction::CallPrint { value } => {
                format!("call void @printInt(i32 {})", value)
            }
            LLVMInstruction::Ret { value } => format!("ret i32 {value}"),
        }
    }
}
