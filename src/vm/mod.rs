#[derive(Debug, Clone, PartialEq)]
#[repr(u32)]
pub enum Instruction {
    PushStr(String) = 10,
    PushInt(i64) = 11,
    PrintStdout = 20,
    PrintStderr = 21,
    Sleep(u64) = 30,
    ConditionalJump = 40,
    Jump = 41,
    End = 50,
}

pub enum StackValue {
    String(String),
    Int(i64),
}

pub struct VM {
    program: Vec<Instruction>,
    stdout: Option<Box<dyn Fn(&str) -> Result<(), std::io::Error>>>, //callback function for stdout
    stderr: Option<Box<dyn Fn(&str) -> Result<(), std::io::Error>>>, //callback function for stderr
}

impl VM {
    pub fn new(program: Vec<Instruction>) -> Self {
        Self {
            program,
            stdout: None,
            stderr: None,
        }
    }

    pub fn with_stdout(
        mut self,
        stdout: Option<Box<dyn Fn(&str) -> Result<(), std::io::Error>>>,
    ) -> Self {
        self.stdout = stdout;
        self
    }

    pub fn with_stderr(
        mut self,
        stderr: Option<Box<dyn Fn(&str) -> Result<(), std::io::Error>>>,
    ) -> Self {
        self.stderr = stderr;
        self
    }

    pub fn run(&self) -> Result<(), std::io::Error> {
        let mut pc = 0;
        let mut stack = Vec::new();

        while pc < self.program.len() {
            match self.program[pc].to_owned() {
                Instruction::PushStr(s) => {
                    stack.push(StackValue::String(s));
                }
                Instruction::PushInt(val) => stack.push(StackValue::Int(val)),
                Instruction::PrintStdout => {
                    let val = stack.pop();
                    if let Some(StackValue::String(s)) = val {
                        self.print_stdout(&s)?;
                    } else {
                        self.print_stderr(&format!(
                            "[Stack Underflow] - Instruction Pointer: {}",
                            pc
                        ))?;
                        break;
                    }
                }
                Instruction::PrintStderr => {
                    let val = stack.pop();
                    if let Some(StackValue::String(s)) = val {
                        self.stderr.as_ref().unwrap()(&s)?;
                    } else {
                        self.print_stderr(&format!(
                            "[Stack Underflow] - Instruction Pointer: {}",
                            pc
                        ))?;
                        break;
                    }
                }
                Instruction::Sleep(duration) => {
                    std::thread::sleep(std::time::Duration::from_millis(duration));
                }
                Instruction::ConditionalJump => todo!(),
                Instruction::Jump => todo!(),
                Instruction::End => break,
            }
            pc += 1;
        }

        Ok(())
    }

    fn print_stdout(&self, value: &str) -> Result<(), std::io::Error> {
        if let Some(stdout) = self.stdout.as_ref() {
            stdout(value)
        } else {
            Ok(())
        }
    }

    fn print_stderr(&self, value: &str) -> Result<(), std::io::Error> {
        if let Some(stderr) = self.stderr.as_ref() {
            stderr(value)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm() {
        let program = vec![
            Instruction::PushStr("Hello, world!".to_string()),
            Instruction::PrintStdout,
            Instruction::End,
        ];

        let stdout = |val: &str| -> Result<(), std::io::Error> {
            assert_eq!(val, "Hello, world!");
            Ok(())
        };

        let vm = VM::new(program).with_stdout(Some(Box::new(stdout)));
        vm.run().unwrap();

        // Check stdout
    }
}
