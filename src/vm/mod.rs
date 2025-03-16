use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
#[repr(u32)]
pub enum Instruction {
    PushStr(String) = 10,
    PushInt(i64) = 11,
    Store(String, Value) = 12,
    Load(String) = 13,
    PrintStdout = 20,
    PrintStderr = 21,
    Sleep(u64) = 30,
    ConditionalJump = 40,
    Jump = 41,
    End = 50,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
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

#[derive(Debug)]
pub enum VMError {
    StackUnderflow,
    ContextError(String),
    IoError(std::io::Error),
}

impl From<std::io::Error> for VMError {
    fn from(error: std::io::Error) -> Self {
        VMError::IoError(error)
    }
}

pub struct VM<'a, O: OutputHandler> {
    instructions: Vec<Instruction>,
    stack: Vec<Value>,
    variables: HashMap<String, Value>,
    ip: usize,
    output_handler: &'a mut O,
}

impl<'a, O: OutputHandler> VM<'a, O> {
    pub fn new(instructions: Vec<Instruction>, output_handler: &'a mut O) -> Self {
        VM {
            instructions,
            stack: Vec::new(),
            variables: HashMap::new(),
            ip: 0,
            output_handler,
        }
    }

    pub fn execute(&mut self) -> Result<(), VMError> {
        while self.ip < self.instructions.len() {
            let instruction = &self.instructions[self.ip].clone();
            self.ip += 1;

            match instruction {
                Instruction::PrintStdout => {
                    let value = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                    self.output_handler
                        .handle_output(&value.to_string(), OutputType::Stdout)?;
                }
                Instruction::PushStr(s) => {
                    self.stack.push(Value::String(s.clone()));
                }
                Instruction::PushInt(val) => self.stack.push(Value::Int(*val)),
                Instruction::PrintStderr => {
                    let val = self.stack.pop();
                    if let Some(Value::String(s)) = val {
                        self.output_handler.handle_output(&s, OutputType::Stderr)?;
                    } else {
                        self.output_handler.handle_output(
                            &format!("[Stack Underflow] - Instruction Pointer: {}", self.ip),
                            OutputType::Stderr,
                        )?;
                        break;
                    }
                }
                Instruction::Sleep(duration) => {
                    std::thread::sleep(std::time::Duration::from_millis(*duration));
                }
                Instruction::ConditionalJump => todo!(),
                Instruction::Jump => todo!(),
                Instruction::End => break,
                Instruction::Store(key, value) => {
                    self.variables.insert(key.clone(), value.clone());
                }
                Instruction::Load(key) => {
                    if let Some(value) = self.variables.get(key.into()) {
                        self.stack.push(value.clone());
                    } else {
                        self.output_handler.handle_output(
                            &format!("[Context Error] - Key not found: {}", key),
                            OutputType::Stderr,
                        )?;
                        break;
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
            let mut vm = VM::new(program, &mut output_handler);
            vm.execute().unwrap();
        }

        assert_eq!(output_handler.stdout, "Hello, world!");
        assert_eq!(output_handler.stderr.len(), 0);
    }

    #[test]
    fn test_vm_with_context() {
        let program = vec![
            Instruction::Store("name".to_string(), Value::String("John".to_string())),
            Instruction::PushStr("Hello, ".to_string()),
            Instruction::PrintStdout,
            Instruction::Load("name".to_string()),
            Instruction::PrintStdout,
            Instruction::PushStr("!".to_string()),
            Instruction::PrintStdout,
            Instruction::End,
        ];

        let mut output_handler = TestOutputHandler::new();
        let mut vm = VM::new(program, &mut output_handler);
        vm.variables
            .insert("name".to_string(), Value::String("John".to_string()));
        vm.execute().unwrap();

        assert_eq!(output_handler.stdout, "Hello, John!");
        assert_eq!(output_handler.stderr.len(), 0);
    }
}
