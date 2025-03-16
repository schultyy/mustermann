use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
#[repr(u32)]
pub enum Instruction {
    PushStr(String) = 10,
    PushInt(i64) = 11,
    StoreConst(String, Value) = 12,
    Store(String) = 13,
    Load(String) = 14,
    PrintStdout = 20,
    PrintStderr = 21,
    Sleep(u64) = 30,
    Add = 40,
    LoopStart(String) = 41,
    LoopEnd(String) = 42,
    ConditionalJump = 50,
    Jump = 51,
    End = 1000,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
}

impl Value {
    pub fn add(&self, other: &Value) -> Result<Value, VMError> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            _ => Err(VMError::TypeMismatch),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Int(i) => write!(f, "{}", i),
        }
    }
}
pub enum OutputType {
    Stdout,
    Stderr,
}

pub trait OutputHandler {
    fn handle_output(
        &mut self,
        output: &str,
        output_type: OutputType,
    ) -> Result<(), std::io::Error>;
}

#[derive(Debug, PartialEq)]
pub enum VMError {
    StackUnderflow,
    ContextError(String),
    IoError(String),
    TypeMismatch,
    LoopError(String),
    MaxInstructions,
}

pub struct VM<'a, O: OutputHandler> {
    instructions: Vec<Instruction>,
    stack: Vec<Value>,
    variables: HashMap<String, Value>,
    ip: usize,
    output_handler: &'a mut O,
    loop_positions: Vec<(String, usize)>, // Map loop labels to instruction positions
    max_instructions: Option<usize>,
    instructions_count: usize,
}

impl<'a, O: OutputHandler> VM<'a, O> {
    pub fn new(
        instructions: Vec<Instruction>,
        output_handler: &'a mut O,
        max_instructions: Option<usize>,
    ) -> Self {
        VM {
            instructions,
            stack: Vec::new(),
            variables: HashMap::new(),
            ip: 0,
            output_handler,
            loop_positions: Vec::new(),
            max_instructions: max_instructions,
            instructions_count: 0,
        }
    }

    pub fn execute(&mut self) -> Result<(), VMError> {
        while self.ip < self.instructions.len() {
            let instruction = &self.instructions[self.ip].clone();
            self.ip += 1;
            self.instructions_count += 1;
            if let Some(max_instructions) = self.max_instructions {
                if self.instructions_count > max_instructions {
                    return Err(VMError::MaxInstructions);
                }
            }

            match instruction {
                Instruction::PrintStdout => {
                    let value = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                    self.output_handler
                        .handle_output(&value.to_string(), OutputType::Stdout)
                        .map_err(|e| VMError::IoError(e.to_string()))?;
                }
                Instruction::PushStr(s) => {
                    self.stack.push(Value::String(s.clone()));
                }
                Instruction::PushInt(val) => self.stack.push(Value::Int(*val)),
                Instruction::PrintStderr => {
                    let val = self.stack.pop();
                    if let Some(Value::String(s)) = val {
                        self.output_handler
                            .handle_output(&s, OutputType::Stderr)
                            .map_err(|e| VMError::IoError(e.to_string()))?;
                    } else {
                        self.output_handler
                            .handle_output(
                                &format!("[Stack Underflow] - Instruction Pointer: {}", self.ip),
                                OutputType::Stderr,
                            )
                            .map_err(|e| VMError::IoError(e.to_string()))?;
                    }
                }
                Instruction::Sleep(duration) => {
                    std::thread::sleep(std::time::Duration::from_millis(*duration));
                }
                Instruction::ConditionalJump => todo!(),
                Instruction::Jump => todo!(),
                Instruction::End => break,
                Instruction::StoreConst(key, value) => {
                    self.variables.insert(key.clone(), value.clone());
                }
                Instruction::Load(key) => {
                    if let Some(value) = self.variables.get(key.into()) {
                        self.stack.push(value.clone());
                    } else {
                        self.output_handler
                            .handle_output(
                                &format!("[Context Error] - Key not found: {}", key),
                                OutputType::Stderr,
                            )
                            .map_err(|e| VMError::IoError(e.to_string()))?;
                        break;
                    }
                }
                Instruction::Store(key) => {
                    let value = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                    self.variables.insert(key.clone(), value.clone());
                }
                Instruction::Add => {
                    let b = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                    let a = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                    self.stack.push(a.add(&b)?);
                }
                Instruction::LoopStart(label) => {
                    self.loop_positions.push((label.clone(), self.ip));
                }
                Instruction::LoopEnd(label) => {
                    if let Some((_, start_pos)) =
                        self.loop_positions.iter().find(|(label, _)| label == label)
                    {
                        self.ip = *start_pos;
                    } else {
                        self.output_handler
                            .handle_output(
                                &format!("[Loop Error] - Unknown loop label: {}", label),
                                OutputType::Stderr,
                            )
                            .map_err(|e| VMError::IoError(e.to_string()))?;
                        return Err(VMError::LoopError(label.clone()));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestOutputHandler {
        stdout: String,
        stderr: String,
    }

    impl TestOutputHandler {
        fn new() -> Self {
            Self {
                stdout: String::new(),
                stderr: String::new(),
            }
        }
    }

    impl OutputHandler for TestOutputHandler {
        fn handle_output(
            &mut self,
            output: &str,
            output_type: OutputType,
        ) -> Result<(), std::io::Error> {
            match output_type {
                OutputType::Stdout => self.stdout.push_str(output),
                OutputType::Stderr => self.stderr.push_str(output),
            }

            Ok(())
        }
    }

    #[test]
    fn test_vm() {
        let program = vec![
            Instruction::PushStr("Hello, world!".to_string()),
            Instruction::PrintStdout,
            Instruction::End,
        ];

        let mut output_handler = TestOutputHandler::new();
        {
            let mut vm = VM::new(program, &mut output_handler, None);
            vm.execute().unwrap();
        }

        assert_eq!(output_handler.stdout, "Hello, world!");
        assert_eq!(output_handler.stderr.len(), 0);
    }

    #[test]
    fn test_vm_with_context() {
        let program = vec![
            Instruction::StoreConst("name".to_string(), Value::String("John".to_string())),
            Instruction::PushStr("Hello, ".to_string()),
            Instruction::PrintStdout,
            Instruction::Load("name".to_string()),
            Instruction::PrintStdout,
            Instruction::PushStr("!".to_string()),
            Instruction::PrintStdout,
            Instruction::End,
        ];

        let mut output_handler = TestOutputHandler::new();
        let mut vm = VM::new(program, &mut output_handler, None);
        vm.variables
            .insert("name".to_string(), Value::String("John".to_string()));
        vm.execute().unwrap();

        assert_eq!(output_handler.stdout, "Hello, John!");
        assert_eq!(output_handler.stderr.len(), 0);
    }

    #[test]
    fn test_vm_with_context_error() {
        let program = vec![
            Instruction::StoreConst("name".to_string(), Value::String("John".to_string())),
            Instruction::Load("name123".to_string()),
            Instruction::PrintStdout,
            Instruction::End,
        ];

        let mut output_handler = TestOutputHandler::new();
        let mut vm = VM::new(program, &mut output_handler, None);
        vm.execute().unwrap();

        assert_eq!(output_handler.stdout.len(), 0);
        assert_eq!(
            output_handler.stderr,
            "[Context Error] - Key not found: name123"
        );
    }

    #[test]
    fn test_vm_with_loop() {
        let program = vec![
            Instruction::StoreConst("counter".to_string(), Value::Int(0)),
            Instruction::LoopStart("test_loop".to_string()),
            Instruction::Load("counter".to_string()),
            Instruction::PushInt(1),
            Instruction::Add,
            Instruction::Store("counter".to_string()),
            Instruction::PushStr("Hello, world!".to_string()),
            Instruction::PrintStdout,
            Instruction::LoopEnd("test_loop".to_string()),
            Instruction::End,
        ];

        let mut output_handler = TestOutputHandler::new();
        let mut vm = VM::new(program, &mut output_handler, Some(10));
        let result = vm.execute();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::MaxInstructions);

        assert_eq!(output_handler.stdout, "Hello, world!");
        assert_eq!(output_handler.stderr.len(), 0);
    }
}
