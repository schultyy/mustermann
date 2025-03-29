use crate::config::{Config, Count, Task};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackValue {
    String(String),
    Int(u64),
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
pub struct ByteCodeGenerator {
    config: Config,
}

impl ByteCodeGenerator {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn generate(&self) -> Vec<Instruction> {
        let mut code = Vec::new();
        for task in &self.config.tasks {
            code.extend_from_slice(&self.process_task(task));
        }
        code
    }

    fn process_task(&self, task: &Task) -> Vec<Instruction> {
        let mut code = Vec::new();
        code.push(Instruction::StoreVar("name".into(), task.name.clone()));
        code.push(Instruction::StoreVar(
            "template".into(),
            task.template.clone(),
        ));
        match task.count {
            Count::Amount(amount) => code.push(Instruction::Push(StackValue::Int(amount))),
            Count::Const(_) => { /* unsupported */ }
        }
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
        code
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
        let generator = ByteCodeGenerator::new(config);
        let code = generator.generate();

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
    fn test_generate() {
        let config = Config::from_file("example.yml").unwrap();
        let generator = ByteCodeGenerator::new(config);
        let code = generator.generate();
        println!("{:?}", code);
    }
}
