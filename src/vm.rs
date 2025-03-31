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
    on_stdout: Box<dyn Fn(&str, &str) -> ()>, //name, message
    on_stderr: Box<dyn Fn(&str, &str) -> ()>, //name, message
}

impl VM {
    pub fn new(
        code: Vec<Instruction>,
        on_stdout: Box<dyn Fn(&str, &str) -> ()>,
        on_stderr: Box<dyn Fn(&str, &str) -> ()>,
    ) -> Self {
        Self {
            code,
            stack: Vec::new(),
            vars: HashMap::new(),
            ip: 0,
            on_stdout,
            on_stderr,
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
            Instruction::Stdout => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::String(s) => {
                        let name = self.vars.get("name").ok_or(VMError::MissingAppName)?;
                        (self.on_stdout)(&name.to_string(), &s);
                    }
                    _ => return Err(VMError::InvalidStackValue),
                }
            }
            Instruction::Stderr => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::String(s) => {
                        let name = self.vars.get("name").ok_or(VMError::MissingAppName)?;
                        (self.on_stderr)(&name.to_string(), &s);
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
            Instruction::Printf => {
                let template = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                let template = match template {
                    StackValue::String(s) => s,
                    _ => return Err(VMError::InvalidStackValue),
                };
                let var = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                let var = match var {
                    StackValue::String(s) => s,
                    _ => return Err(VMError::InvalidStackValue),
                };

                let formatted = template.replace("%s", &var);
                self.stack.push(StackValue::String(formatted));
            }
            Instruction::RemoteCall => { /* Not Implemented */ }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_run() {
        let mut vm = VM::new(
            vec![Instruction::StoreVar(
                "name".to_string(),
                "test".to_string(),
            )],
            Box::new(|name, message| {
                println!("{}: {}", name, message);
            }),
            Box::new(|name, message| {
                println!("{}: {}", name, message);
            }),
        );
        vm.run().unwrap();
    }
}
