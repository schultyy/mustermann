use tabled::Tabled;

use crate::code_gen::instruction::Instruction;

#[derive(Tabled)]
pub struct AnnotatedInstruction {
    instruction: String,
    description: String,
}

impl AnnotatedInstruction {
    pub fn new(instruction: String, description: String) -> Self {
        Self {
            instruction,
            description,
        }
    }
}

impl Into<AnnotatedInstruction> for &Instruction {
    fn into(self) -> AnnotatedInstruction {
        match self {
            Instruction::Push(stack_value) => AnnotatedInstruction {
                instruction: "Push".to_string(),
                description: format!("Push {:?}", stack_value),
            },
            Instruction::Pop => AnnotatedInstruction {
                instruction: "Pop".to_string(),
                description: "Pop the top of the stack".to_string(),
            },
            Instruction::Dec => AnnotatedInstruction {
                instruction: "Dec".to_string(),
                description: "Decrement the top of the stack".to_string(),
            },
            Instruction::JmpIfZero(label) => AnnotatedInstruction {
                instruction: "JmpIfZero".to_string(),
                description: format!("Jump if the top of the stack is zero to {}", label),
            },
            Instruction::Label(label) => AnnotatedInstruction {
                instruction: "Label".to_string(),
                description: format!("Label {}", label),
            },
            Instruction::Stdout => AnnotatedInstruction {
                instruction: "Stdout".to_string(),
                description: "Print the top of the stack to stdout".to_string(),
            },
            Instruction::Stderr => AnnotatedInstruction {
                instruction: "Stderr".to_string(),
                description: "Print the top of the stack to stderr".to_string(),
            },
            Instruction::Sleep(ms) => AnnotatedInstruction {
                instruction: "Sleep".to_string(),
                description: format!("Sleep for {}ms", ms),
            },
            Instruction::StoreVar(var, _) => AnnotatedInstruction {
                instruction: "StoreVar".to_string(),
                description: format!("Store the top of the stack in the variable {}", var),
            },
            Instruction::LoadVar(var) => AnnotatedInstruction {
                instruction: "LoadVar".to_string(),
                description: format!("Load the variable {} into the top of the stack", var),
            },
            Instruction::Dup => AnnotatedInstruction {
                instruction: "Dup".to_string(),
                description: "Duplicate the top of the stack".to_string(),
            },
            Instruction::Jump(label) => AnnotatedInstruction {
                instruction: "Jump".to_string(),
                description: format!("Jump to {}", label),
            },
            Instruction::Printf => AnnotatedInstruction {
                instruction: "Printf".to_string(),
                description:
                    "Takes the top two values of the stack, and pushes the formatted string back onto the stack"
                        .to_string(),
            },
            Instruction::RemoteCall => AnnotatedInstruction {
                instruction: "RemoteCall".to_string(),
                description: "Call a remote service".to_string(),
            },
            Instruction::StartContext => AnnotatedInstruction {
                instruction: "StartContext".to_string(),
                description: "Start a new context".to_string(),
            },
            Instruction::EndContext => AnnotatedInstruction {
                instruction: "EndContext".to_string(),
                description: "End the current context".to_string(),
            },
            Instruction::Nop => AnnotatedInstruction {
                instruction: "Nop".to_string(),
                description: "No operation".to_string(),
            },
            Instruction::Call(label) => AnnotatedInstruction {
                instruction: "Call".to_string(),
                description: format!("Call {}", label),
            },
            Instruction::Ret => AnnotatedInstruction {
                instruction: "Ret".to_string(),
                description: "Return from the current function".to_string(),
            },
        }
    }
}
