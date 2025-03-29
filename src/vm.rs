use std::collections::HashMap;

use crate::config::{Config, Count, Task};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackValue {
    String(String),
    Int(u64),
}

impl std::fmt::Display for StackValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackValue::String(s) => write!(f, "{}", s),
            StackValue::Int(n) => write!(f, "{}", n),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Push(StackValue),
    Pop,
    Dec,
    JmpIfZero(String),
    Label(String),
    StrJoin,
    Stdout,
    Sleep(u64),
    StoreVar(String, String),
    LoadVar(String),
    Dup,
    Jump(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteCodeError {
    UnsupportedConst(String),
}

impl std::error::Error for ByteCodeError {}

impl std::fmt::Display for ByteCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ByteCodeError::UnsupportedConst(val) => write!(f, "Unsupported constant: {}", val),
        }
    }
}

pub struct ByteCodeGenerator<'a> {
    task: &'a Task,
}

impl<'a> ByteCodeGenerator<'a> {
    pub fn new(task: &'a Task) -> Self {
        Self { task }
    }

    pub fn process_task(&self) -> Result<Vec<Instruction>, ByteCodeError> {
        match &self.task.count {
            Count::Amount(_) => self.task_with_count(self.task),
            Count::Const(val) => {
                if val == "Infinite" {
                    self.task_with_infinite_loop(self.task)
                } else {
                    Err(ByteCodeError::UnsupportedConst(val.clone()))
                }
            }
        }
    }

    fn task_with_infinite_loop(&self, task: &Task) -> Result<Vec<Instruction>, ByteCodeError> {
        let mut code = Vec::new();
        code.push(Instruction::StoreVar("name".into(), task.name.clone()));
        code.push(Instruction::StoreVar(
            "template".into(),
            task.template.clone(),
        ));
        code.push(Instruction::Label(format!("loop_{}", task.name)));
        code.push(Instruction::LoadVar("name".into()));
        code.push(Instruction::Push(StackValue::String(" ".to_string())));
        code.push(Instruction::LoadVar("template".into()));
        code.push(Instruction::StrJoin);
        code.push(Instruction::Stdout);
        code.push(Instruction::Sleep(task.frequency));
        code.push(Instruction::Jump(format!("loop_{}", task.name)));
        code.push(Instruction::Label(format!("end_{}", task.name)));
        Ok(code)
    }

    fn task_with_count(&self, task: &Task) -> Result<Vec<Instruction>, ByteCodeError> {
        let mut code = Vec::new();
        code.push(Instruction::StoreVar("name".into(), task.name.clone()));
        code.push(Instruction::StoreVar(
            "template".into(),
            task.template.clone(),
        ));
        let count = match &task.count {
            Count::Amount(amount) => amount,
            Count::Const(val) => {
                return Err(ByteCodeError::UnsupportedConst(val.clone()));
            }
        };
        code.push(Instruction::Push(StackValue::Int(*count)));
        code.push(Instruction::Label(format!("loop_{}", task.name)));
        code.push(Instruction::Dup);
        code.push(Instruction::JmpIfZero(format!("end_{}", task.name)));
        code.push(Instruction::Dec);
        code.push(Instruction::LoadVar("name".into()));
        code.push(Instruction::Push(StackValue::String(" ".to_string())));
        code.push(Instruction::LoadVar("template".into()));
        code.push(Instruction::StrJoin);
        code.push(Instruction::Stdout);
        code.push(Instruction::Sleep(task.frequency));
        code.push(Instruction::Jump(format!("loop_{}", task.name)));
        code.push(Instruction::Label(format!("end_{}", task.name)));
        code.push(Instruction::Pop);
        Ok(code)
    }
}

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
    use crate::config::{Count, Severity, Task};

    use super::*;

    #[test]
    fn test_config_parse() {
        let config = Config {
            tasks: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Amount(10),
                template: "User logged in".to_string(),
                vars: vec![],
                severity: Severity::Info,
            }],
        };
        let generator = ByteCodeGenerator::new(&config.tasks[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Push(10)                              // Initial counter value
        Label("loop_start")                   // Loop start
        Dup                                   // Duplicate counter on stack
        JmpIfZero("loop_end")                 // Exit if counter is zero
        Dec                                   // Decrement the counter
        LoadVar("name")                       // Load the name (was "test")
        Push(" ")                             // Push separator
        LoadVar("template")                   // Load template
        StrJoin                               // Join the strings
        Stdout                                // Print to stdout
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        Pop                                   // Clean up counter from stack
        */

        assert_eq!(code.len(), 16);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(
            code[1],
            Instruction::StoreVar("template".to_string(), "User logged in".to_string())
        );
        assert_eq!(code[2], Instruction::Push(StackValue::Int(10)));
        assert_eq!(code[3], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[4], Instruction::Dup);
        assert_eq!(code[5], Instruction::JmpIfZero("end_test".to_string()));
        assert_eq!(code[6], Instruction::Dec);
        assert_eq!(code[7], Instruction::LoadVar("name".to_string()));
        assert_eq!(
            code[8],
            Instruction::Push(StackValue::String(" ".to_string()))
        );
        assert_eq!(code[9], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[10], Instruction::StrJoin);
        assert_eq!(code[11], Instruction::Stdout);
        assert_eq!(code[12], Instruction::Sleep(1000));
        assert_eq!(code[13], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[14], Instruction::Label("end_test".to_string()));
        assert_eq!(code[15], Instruction::Pop);
    }

    #[test]
    fn test_generate_infinite_loop() {
        let config = Config {
            tasks: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Const("Infinite".to_string()),
                template: "User logged in".to_string(),
                vars: vec![],
                severity: Severity::Info,
            }],
        };
        let generator = ByteCodeGenerator::new(&config.tasks[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Label("loop_start")                   // Loop start
        LoadVar("name")                       // Load the name (was "test")
        Push(" ")                             // Push separator
        LoadVar("template")                   // Load template
        StrJoin                               // Join the strings
        Stdout                                // Print to stdout
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        */

        assert_eq!(code.len(), 11);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(
            code[1],
            Instruction::StoreVar("template".to_string(), "User logged in".to_string())
        );
        assert_eq!(code[2], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[3], Instruction::LoadVar("name".to_string()));
        assert_eq!(
            code[4],
            Instruction::Push(StackValue::String(" ".to_string()))
        );
        assert_eq!(code[5], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[6], Instruction::StrJoin);
        assert_eq!(code[7], Instruction::Stdout);
        assert_eq!(code[8], Instruction::Sleep(1000));
        assert_eq!(code[9], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[10], Instruction::Label("end_test".to_string()));
    }

    #[test]
    fn test_vm_run() {
        let mut vm = VM::new(vec![Instruction::StoreVar(
            "name".to_string(),
            "test".to_string(),
        )]);
        vm.run().unwrap();
    }
}
