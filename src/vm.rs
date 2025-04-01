use std::{collections::HashMap, sync::Arc};

use tokio::sync::mpsc;

use crate::{
    code_gen::instruction::{Instruction, StackValue},
    vm_coordinator::ServiceMessage,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMError {
    StackUnderflow,
    InvalidStackValue,
    MissingAppName,
    MissingVar(String),
    RemoteCallError,
    MissingLabel(String),
}

impl std::error::Error for VMError {}

impl std::fmt::Display for VMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VMError::StackUnderflow => write!(f, "Stack underflow"),
            VMError::InvalidStackValue => write!(f, "Invalid stack value"),
            VMError::MissingAppName => write!(f, "Missing app name"),
            VMError::MissingVar(var) => write!(f, "Missing variable: {}", var),
            VMError::RemoteCallError => write!(f, "Remote call error"),
            VMError::MissingLabel(label) => write!(f, "Missing label: {}", label),
        }
    }
}

pub struct VM {
    code: Vec<Instruction>,
    stack: Vec<StackValue>,
    vars: HashMap<String, StackValue>,
    ip: usize,
    // on_stdout: Arc<Box<dyn Fn(&str, &str) -> ()>>, //name, message
    // on_stderr: Arc<Box<dyn Fn(&str, &str) -> ()>>, //name, message
    tx: Option<mpsc::Sender<ServiceMessage>>,
    rx: Option<mpsc::Receiver<String>>,
    message_check_counter: usize,
}

impl VM {
    pub fn new(
        code: Vec<Instruction>,
        tx: Option<mpsc::Sender<ServiceMessage>>,
        rx: Option<mpsc::Receiver<String>>,
    ) -> Self {
        Self {
            code,
            stack: Vec::new(),
            vars: HashMap::new(),
            ip: 0,
            tx,
            rx,
            message_check_counter: 0,
        }
    }

    pub async fn run(&mut self) -> Result<(), VMError> {
        while self.ip < self.code.len() {
            let instruction = self.code[self.ip].clone();
            self.ip += 1;
            self.message_check_counter += 1;
            if self.message_check_counter > 10000 {
                if let Some(rx) = &mut self.rx {
                    if let Ok(msg) = rx.try_recv() {
                        self.handle_service_message(msg)?;
                    }
                }
                self.message_check_counter = 0;
            }
            self.execute_instruction(instruction).await?;
        }
        Ok(())
    }

    fn handle_service_message(&mut self, msg: String) -> Result<(), VMError> {
        self.ip = self
            .code
            .iter()
            .position(|i| i == &Instruction::Label(msg.clone()))
            .ok_or(VMError::MissingLabel(msg.clone()))?;
        Ok(())
    }

    async fn execute_instruction(&mut self, instruction: Instruction) -> Result<(), VMError> {
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
                let name = self.vars.get("name").ok_or(VMError::MissingAppName)?;
                match top {
                    StackValue::String(s) => {
                        tracing::info!("{}: {}", name, s);
                    }
                    StackValue::Int(n) => {
                        tracing::info!("{}: {}", name, n);
                    }
                    _ => return Err(VMError::InvalidStackValue),
                }
            }
            Instruction::Stderr => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::String(s) => {
                        let name = self.vars.get("name").ok_or(VMError::MissingAppName)?;
                        tracing::error!("{}: {}", name, s);
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
                    .ok_or(VMError::MissingLabel(label.clone()))?;
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
            Instruction::RemoteCall => {
                if let Some(tx) = self.tx.as_ref() {
                    let method = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                    let service = self.stack.pop().ok_or(VMError::StackUnderflow)?;

                    //find last label in code
                    let function_name = self
                        .code
                        .iter()
                        .rev()
                        .find(|i| matches!(i, Instruction::Label(_)))
                        .unwrap();

                    let function_name = match function_name {
                        Instruction::Label(s) => s,
                        _ => return Err(VMError::InvalidStackValue),
                    };
                    let root_span = tracing::info_span!(
                        "vm_remote_call",
                        service = %self.vars.get("name").unwrap(),
                        method = %function_name,
                    );

                    let _guard = root_span.enter();

                    let service = match service {
                        StackValue::String(s) => s,
                        _ => return Err(VMError::InvalidStackValue),
                    };
                    let method = match method {
                        StackValue::String(s) => s,
                        _ => return Err(VMError::InvalidStackValue),
                    };

                    tx.send(ServiceMessage::Call {
                        to: service,
                        function: method,
                        parent: root_span.clone(),
                    })
                    .await
                    .or(Err(VMError::RemoteCallError))?;

                    tracing::info!("Remote call initiated");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vm_run() {
        let mut vm = VM::new(
            vec![Instruction::StoreVar(
                "name".to_string(),
                "test".to_string(),
            )],
            None,
            None,
        );
        vm.run().await.unwrap();
    }
}
