/// Main file containing the LLVM code generation logic.
use crate::error::SemanticError;
use crate::backend::{LLVMInstruction, PointerName, RegisterName};
use crate::parser::ast::{BinOp, Expr, ExprKind, Program, Stmt, StmtKind};
use std::collections::HashMap;

type RegisterIndex = i32;

#[derive(Debug, Default)]
pub struct LLVMGenerator {
    output: String,
    // Mapping of local variables to their pointer name in generated code.
    variables: HashMap<String, PointerName>,
    register_index: RegisterIndex,
}

impl LLVMGenerator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            variables: HashMap::new(),
            register_index: 1,
        }
    }

    /// Creates a new register and returns it's name.
    fn new_register(&mut self) -> RegisterName {
        let name = format!("%{}", self.register_index);
        self.register_index += 1;
        name
    }

    /// If a variable exists returns it's pointers name.
    /// If not, creates a new variable and returns it's name.
    fn get_or_create_variable(&mut self, name: &str) -> PointerName {
        if let Some(ptr_name) = self.variables.get(name) {
            ptr_name.clone()
        } else {
            let ptr_name = format!("%{}", name);
            let instr = LLVMInstruction::Alloca {
                destination: ptr_name.clone(),
            };
            self.emit_instruction(&instr);
            self.variables.insert(name.to_string(), ptr_name.clone());
            ptr_name
        }
    }

    /// Performs the lowering for the whole program.
    pub fn lower_program(&mut self, program: &mut Program) -> Result<String, SemanticError> {
        self.emit_preambule();
        for stmt in &program.0 {
            self.lower_statement(stmt)?;
        }
        self.emit_postambule();
        Ok(self.output.clone())
    }

    /// Performs the lowering of a single statement.
    fn lower_statement(&mut self, stmt: &Stmt) -> Result<(), SemanticError> {
        match &stmt.kind {
            StmtKind::Assignment { name, value } => {
                let val_reg = self.lower_expression(value)?;
                let ptr_name = self.get_or_create_variable(name);
                let instr = LLVMInstruction::Store {
                    value: val_reg,
                    destination: ptr_name,
                };
                self.emit_instruction(&instr);
            }
            StmtKind::Expression { value } => {
                let val_reg = self.lower_expression(value)?;
                let instr = LLVMInstruction::CallPrint { value: val_reg };
                self.emit_instruction(&instr);
            }
        }

        self.emit("\n");
        Ok(())
    }

    // Performs the lowering of a single expression.
    fn lower_expression(&mut self, expr: &Expr) -> Result<RegisterName, SemanticError> {
        match &expr.kind {
            ExprKind::Literal(val) => Ok(val.to_string()),
            ExprKind::Identifier(name) => {
                if let Some(ptr_name) = self.variables.get(name).cloned() {
                    let dest_reg = self.new_register();
                    let instr = LLVMInstruction::Load {
                        destination: dest_reg.clone(),
                        source: ptr_name,
                    };

                    self.emit_instruction(&instr);
                    Ok(dest_reg)
                } else {
                    Err(SemanticError::UndefinedVariable {
                        name: name.clone(),
                        position: expr.position,
                    })
                }
            }
            ExprKind::BinaryOp { op, lhs, rhs, .. } => {
                if let ExprKind::Literal(0) = rhs.kind
                    && matches!(op, BinOp::Div)
                {
                    return Err(SemanticError::ZeroDivision {
                        position: rhs.position,
                    });
                }

                let lhs_reg = self.lower_expression(lhs)?;
                let rhs_reg = self.lower_expression(rhs)?;

                let location = self.new_register();

                let instr = match op {
                    BinOp::Add => LLVMInstruction::Add {
                        location: location.clone(),
                        left: lhs_reg,
                        right: rhs_reg,
                    },
                    BinOp::Sub => LLVMInstruction::Sub {
                        location: location.clone(),
                        left: lhs_reg,
                        right: rhs_reg,
                    },
                    BinOp::Mul => LLVMInstruction::Mul {
                        location: location.clone(),
                        left: lhs_reg,
                        right: rhs_reg,
                    },
                    BinOp::Div => LLVMInstruction::Div {
                        location: location.clone(),
                        left: lhs_reg,
                        right: rhs_reg,
                    },
                };
                self.emit_instruction(&instr);
                Ok(location)
            }
        }
    }

    fn emit_instruction(&mut self, instruction: &LLVMInstruction) {
        self.output.push_str("    ");
        self.output.push_str(instruction.stringify().as_str());
        self.output.push('\n');
    }

    fn emit(&mut self, instruction: &str) {
        self.output.push_str(instruction);
        self.output.push('\n');
    }

    fn emit_preambule(&mut self) {
        self.emit("@dnl = internal constant [4 x i8] c\"%d\\0A\\00\"");
        self.emit("declare i32 @printf(i8*, ...)");
        self.emit("define void @printInt(i32 %x) {");
        self.emit("       %t0 = getelementptr [4 x i8], [4 x i8]* @dnl, i32 0, i32 0");
        self.emit("       call i32 (i8*, ...) @printf(i8* %t0, i32 %x) ");
        self.emit("       ret void");
        self.emit("}");
        self.emit("");
        self.emit("define i32 @main() {");
    }

    fn emit_postambule(&mut self) {
        self.emit("");
        self.emit_instruction(&LLVMInstruction::Ret {
            value: 0.to_string(),
        });
        self.emit("}");
    }
}
