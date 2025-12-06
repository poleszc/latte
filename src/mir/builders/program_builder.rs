use crate::mir::{MIRClass, MIRFunction, MIRProgram};

pub struct MIRProgramBuilder {
    program: MIRProgram,
}

impl MIRProgramBuilder {
    pub fn new() -> Self {
        Self {
            program: MIRProgram::new(),
        }
    }

    pub fn add_function(&mut self, function: MIRFunction) {
        self.program.add_function(function);
    }

    pub fn add_class(&mut self, class: MIRClass) {
        self.program.add_class(class);
    }

    /// Returns the built program. Consumes the MIRProgramBuilder object.
    pub fn build(self) -> MIRProgram {
        self.program
    }
}

impl Default for MIRProgramBuilder {
    fn default() -> Self {
        Self::new()
    }
}
