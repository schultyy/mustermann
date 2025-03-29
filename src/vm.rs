use std::collections::HashMap;

use crate::code_gen::{Instruction, StackValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMError {
    StackUnderflow,
    InvalidStackValue,
    MissingAppName,
    MissingVar(String),
}

impl std::error::Error for VMError {}

impl std::fmt::Display for VMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VMError::StackUnderflow => write!(f, "Stack underflow"),
            VMError::InvalidStackValue => write!(f, "Invalid stack value"),
            VMError::MissingAppName => write!(f, "Missing app name"),
            VMError::MissingVar(var) => write!(f, "Missing variable: {}", var),
        }
    }
}
pub struct VM {
    code: Vec<Instruction>,
    stack: Vec<StackValue>,
    vars: HashMap<String, StackValue>,
    ip: usize,
}

impl VM {
    pub fn new(code: Vec<Instruction>) -> Self {
        Self {
            code,
            stack: Vec::new(),
            vars: HashMap::new(),
            ip: 0,
        }
    }

    pub fn run(&mut self) -> Result<(), VMError> {
        while self.ip < self.code.len() {
            let instruction = self.code[self.ip].clone();
            self.ip += 1;
            self.execute_instruction(instruction)?;
        }
        Ok(())
    }

    fn execute_instruction(&mut self, instruction: Instruction) -> Result<(), VMError> {
        match instruction {
            Instruction::Push(stack_value) => {
                self.stack.push(stack_value.to_owned());
            }
            Instruction::Pop => {
                self.stack.pop();
            }
            Instruction::Dec => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::Int(n) => self.stack.push(StackValue::Int(n - 1)),
                    _ => return Err(VMError::InvalidStackValue),
                }
            }
            Instruction::JmpIfZero(label) => {
                let top = self.stack.last().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::Int(0) => {
                        self.ip = self
                            .code
                            .iter()
                            .position(|i| i == &Instruction::Label(label.clone()))
                            .unwrap();
                    }
                    _ => {}
                }
            }
            Instruction::Label(_) => {
                // Labels are used for jumps and are not executed
            }
            Instruction::StrJoin => {
                let mut joined = String::new();
                while let Some(StackValue::String(s)) = self.stack.pop() {
                    joined.push_str(&s);
                }
                self.stack.push(StackValue::String(joined));
            }
            Instruction::Stdout => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::String(s) => {
                        let name = self.vars.get("name").ok_or(VMError::MissingAppName)?;
                        tracing::info!(app_name = name.to_string(), "{}", s);
                    }
                    _ => return Err(VMError::InvalidStackValue),
                }
            }
            Instruction::Stderr => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::String(s) => {
                        let name = self.vars.get("name").ok_or(VMError::MissingAppName)?;
                        tracing::error!(app_name = name.to_string(), "{}", s);
                    }
                    _ => return Err(VMError::InvalidStackValue),
                }
            }
            Instruction::Sleep(ms) => {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
            Instruction::StoreVar(key, value) => {
                self.vars
                    .insert(key.clone(), StackValue::String(value.clone()));
            }
            Instruction::LoadVar(key) => {
                let value = self
                    .vars
                    .get(&key)
                    .ok_or(VMError::MissingVar(key.clone()))?;
                self.stack.push(value.clone());
            }
            Instruction::Dup => {
                let top = self.stack.last().ok_or(VMError::StackUnderflow)?;
                self.stack.push(top.clone());
            }
            Instruction::Jump(label) => {
                self.ip = self
                    .code
                    .iter()
                    .position(|i| i == &Instruction::Label(label.clone()))
                    .unwrap();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_run() {
        let mut vm = VM::new(vec![Instruction::StoreVar(
            "name".to_string(),
            "test".to_string(),
        )]);
        vm.run().unwrap();
    }
}
