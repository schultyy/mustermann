use crate::config::{Count, Method, Service, Severity, Task};

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
    Stdout,
    Stderr,
    Sleep(u64),
    StoreVar(String, String),
    LoadVar(String),
    Dup,
    Jump(String),
    Printf,
    RemoteCall,
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

pub struct ServiceByteCodeGenerator<'a> {
    service: &'a Service,
}

impl<'a> ServiceByteCodeGenerator<'a> {
    pub fn new(service: &'a Service) -> Self {
        Self { service }
    }

    pub fn process_service(&self) -> Result<Vec<Instruction>, ByteCodeError> {
        let mut code = Vec::new();
        code.push(Instruction::StoreVar(
            "name".into(),
            self.service.name.clone(),
        ));
        code.push(Instruction::Jump("main".into()));
        for method in &self.service.methods {
            let method_generator = MethodByteCodeGenerator::new(method);
            let method_code = method_generator.process_method()?;
            code.extend(method_code);
        }

        code.push(Instruction::Label("main".into()));
        for method in &self.service.methods {
            code.push(Instruction::Jump(format!("{}", method.name)));
        }
        code.push(Instruction::Label("end_main".into()));
        Ok(code)
    }
}

pub struct MethodByteCodeGenerator<'a> {
    method: &'a Method,
}

impl<'a> MethodByteCodeGenerator<'a> {
    pub fn new(method: &'a Method) -> Self {
        Self { method }
    }

    pub fn process_method(&self) -> Result<Vec<Instruction>, ByteCodeError> {
        let mut code = Vec::new();
        code.push(Instruction::Label(format!("{}", self.method.name)));

        if let Some(stdout) = &self.method.stdout {
            code.push(Instruction::Push(StackValue::String(stdout.clone())));
            code.push(Instruction::Stdout);
        }

        if let Some(sleep_ms) = self.method.sleep_ms {
            code.push(Instruction::Sleep(sleep_ms));
        }

        if let Some(calls) = &self.method.calls {
            for call in calls {
                code.push(Instruction::Push(StackValue::String(call.name.clone())));
                code.push(Instruction::Push(StackValue::String(call.method.clone())));
                code.push(Instruction::RemoteCall);
            }
        }
        code.push(Instruction::Jump("main".into()));
        code.push(Instruction::Label(format!("end_{}", self.method.name)));

        Ok(code)
    }
}
pub struct LogByteCodeGenerator<'a> {
    task: &'a Task,
    has_vars: bool,
}

impl<'a> LogByteCodeGenerator<'a> {
    pub fn new(task: &'a Task) -> Self {
        Self {
            task,
            has_vars: task.vars.len() > 0,
        }
    }

    pub fn process_task(&self) -> Result<Vec<Instruction>, ByteCodeError> {
        let mut code = Vec::new();
        code.push(Instruction::StoreVar("name".into(), self.task.name.clone()));
        code.push(Instruction::StoreVar(
            "template".into(),
            self.task.template.clone(),
        ));

        match &self.task.count {
            Count::Amount(_) => self.task_with_count(&mut code, self.task)?,
            Count::Const(val) => {
                if val == "Infinite" {
                    self.task_with_infinite_loop(&mut code, self.task)?
                } else {
                    return Err(ByteCodeError::UnsupportedConst(val.clone()));
                }
            }
        }
        Ok(code)
    }

    fn task_with_infinite_loop(
        &self,
        code: &mut Vec<Instruction>,
        task: &Task,
    ) -> Result<(), ByteCodeError> {
        self.generate_var_store_instructions(code, task)?;
        code.push(Instruction::Label(format!("loop_{}", task.name)));
        self.generate_print_statement(code, task)?;
        code.push(Instruction::Jump(format!("loop_{}", task.name)));
        code.push(Instruction::Label(format!("end_{}", task.name)));
        Ok(())
    }

    fn task_with_count(
        &self,
        code: &mut Vec<Instruction>,
        task: &Task,
    ) -> Result<(), ByteCodeError> {
        let loop_max_counter = match &task.count {
            Count::Amount(amount) => amount,
            Count::Const(val) => {
                return Err(ByteCodeError::UnsupportedConst(val.clone()));
            }
        };
        self.generate_var_store_instructions(code, task)?;
        code.push(Instruction::Push(StackValue::Int(*loop_max_counter)));
        code.push(Instruction::Label(format!("loop_{}", task.name)));
        code.push(Instruction::Dup);
        code.push(Instruction::JmpIfZero(format!("end_{}", task.name)));
        code.push(Instruction::Dec);
        self.generate_print_statement(code, task)?;
        code.push(Instruction::Jump(format!("loop_{}", task.name)));
        code.push(Instruction::Label(format!("end_{}", task.name)));
        code.push(Instruction::Pop);
        Ok(())
    }

    fn generate_var_store_instructions(
        &self,
        code: &mut Vec<Instruction>,
        task: &Task,
    ) -> Result<(), ByteCodeError> {
        task.vars.iter().enumerate().for_each(|(index, var)| {
            code.push(Instruction::StoreVar(format!("var_{}", index), var.clone()));
        });
        Ok(())
    }

    fn generate_print_statement(
        &self,
        code: &mut Vec<Instruction>,
        task: &Task,
    ) -> Result<(), ByteCodeError> {
        if self.has_vars {
            for (index, _var) in task.vars.iter().enumerate() {
                code.push(Instruction::LoadVar(format!("var_{}", index)));
                code.push(Instruction::LoadVar("template".into()));
                code.push(Instruction::Printf);

                match task.severity {
                    Severity::Info => code.push(Instruction::Stdout),
                    Severity::Error => code.push(Instruction::Stderr),
                }
                code.push(Instruction::Sleep(task.frequency));
            }
        } else {
            code.push(Instruction::LoadVar("template".into()));
            match task.severity {
                Severity::Info => code.push(Instruction::Stdout),
                Severity::Error => code.push(Instruction::Stderr),
            }
            code.push(Instruction::Sleep(task.frequency));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Call, Config, Method, Service};

    use super::*;

    #[test]
    fn test_config_parse() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Amount(10),
                template: "User logged in".to_string(),
                vars: vec![],
                severity: Severity::Info,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Push(10)                              // Initial counter value
        Label("loop_start")                   // Loop start
        Dup                                   // Duplicate counter on stack
        JmpIfZero("loop_end")                 // Exit if counter is zero
        Dec                                   // Decrement the counter
        LoadVar("template")                   // Load template
        Stdout                                // Print to stdout
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        Pop                                   // Clean up counter from stack
        */

        assert_eq!(code.len(), 13);
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
        assert_eq!(code[7], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[8], Instruction::Stdout);
        assert_eq!(code[9], Instruction::Sleep(1000));
        assert_eq!(code[10], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[11], Instruction::Label("end_test".to_string()));
        assert_eq!(code[12], Instruction::Pop);
    }

    #[test]
    fn test_counted_loop_with_vars() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Amount(10),
                template: "User %s logged in".to_string(),
                vars: vec!["John".to_string()],
                severity: Severity::Info,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Push(10)                              // Initial counter value
        Label("loop_start")                   // Loop start
        Dup                                   // Duplicate counter on stack
        JmpIfZero("loop_end")                 // Exit if counter is zero
        Dec                                   // Decrement the counter
        LoadVar("template")                   // Load template
        LoadVar("var_0")                      // Load variable
        Printf                                // Join the strings
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
            Instruction::StoreVar("template".to_string(), "User %s logged in".to_string())
        );
        assert_eq!(
            code[2],
            Instruction::StoreVar("var_0".to_string(), "John".to_string())
        );
        assert_eq!(code[3], Instruction::Push(StackValue::Int(10)));
        assert_eq!(code[4], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[5], Instruction::Dup);
        assert_eq!(code[6], Instruction::JmpIfZero("end_test".to_string()));
        assert_eq!(code[7], Instruction::Dec);
        assert_eq!(code[8], Instruction::LoadVar("var_0".to_string()));
        assert_eq!(code[9], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[10], Instruction::Printf);
        assert_eq!(code[11], Instruction::Stdout);
        assert_eq!(code[12], Instruction::Sleep(1000));
        assert_eq!(code[13], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[14], Instruction::Label("end_test".to_string()));
        assert_eq!(code[15], Instruction::Pop);
    }

    #[test]
    fn test_generate_infinite_loop_with_single_var() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Const("Infinite".to_string()),
                template: "User %s logged in".to_string(),
                vars: vec!["John".to_string()],
                severity: Severity::Info,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User %s logged in") // Store template
        StoreVar("var_0", "John")               // Store variable
        Label("loop_start")                   // Loop start
        LoadVar("var_0")                      // Load variable
        LoadVar("template")                   // Load template
        Printf                                // Join the strings
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
            Instruction::StoreVar("template".to_string(), "User %s logged in".to_string())
        );
        assert_eq!(
            code[2],
            Instruction::StoreVar("var_0".to_string(), "John".to_string())
        );
        assert_eq!(code[3], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[4], Instruction::LoadVar("var_0".to_string()));
        assert_eq!(code[5], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[6], Instruction::Printf);
        assert_eq!(code[7], Instruction::Stdout);
        assert_eq!(code[8], Instruction::Sleep(1000));
        assert_eq!(code[9], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[10], Instruction::Label("end_test".to_string()));
    }

    #[test]
    fn test_generate_infinite_loop() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Const("Infinite".to_string()),
                template: "User logged in".to_string(),
                vars: vec![],
                severity: Severity::Info,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Label("loop_start")                   // Loop start
        LoadVar("template")                   // Load template
        Stdout                                // Print to stdout
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        */

        assert_eq!(code.len(), 8);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(
            code[1],
            Instruction::StoreVar("template".to_string(), "User logged in".to_string())
        );
        assert_eq!(code[2], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[3], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[4], Instruction::Stdout);
        assert_eq!(code[5], Instruction::Sleep(1000));
        assert_eq!(code[6], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[7], Instruction::Label("end_test".to_string()));
    }

    #[test]
    fn test_print_stderr() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Const("Infinite".to_string()),
                template: "User logged in".to_string(),
                vars: vec![],
                severity: Severity::Error,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Label("loop_start")                   // Loop start
        LoadVar("name")                       // Load the name (was "test")
        Push(" ")                             // Push separator
        LoadVar("template")                   // Load template
        StrJoin                               // Join the strings
        StdErr                                // Print to stderr
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        */

        assert_eq!(code.len(), 8);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(
            code[1],
            Instruction::StoreVar("template".to_string(), "User logged in".to_string())
        );
        assert_eq!(code[2], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[3], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[4], Instruction::Stderr);
        assert_eq!(code[5], Instruction::Sleep(1000));
        assert_eq!(code[6], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[7], Instruction::Label("end_test".to_string()));
    }

    #[test]
    fn test_generate_services() {
        let config = Config {
            logs: vec![],
            services: vec![Service {
                name: "test".to_string(),
                methods: vec![Method {
                    name: "charge".to_string(),
                    stdout: Some("Charging".to_string()),
                    sleep_ms: Some(500),
                    calls: Some(vec![Call {
                        name: "checkout".to_string(),
                        method: "process".to_string(),
                    }]),
                }],
            }],
        };

        let generator = ServiceByteCodeGenerator::new(&config.services[0]);
        let code = generator.process_service().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        Jump("main")
        ---
        Label("charge")
        Push("Charging")
        Stdout
        Sleep(500)
        Label("end_charge")
        Push("checkout")
        Push("process")
        RemoteCall
        Jump("main")
        ---
        Label("main")
        Jump("charge")
        Jump("main")
        Label("end_main")
        */
        assert_eq!(code.len(), 14);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(code[1], Instruction::Jump("main".to_string()));
        //--
        assert_eq!(code[2], Instruction::Label("charge".to_string()));
        assert_eq!(
            code[3],
            Instruction::Push(StackValue::String("Charging".to_string()))
        );
        assert_eq!(code[4], Instruction::Stdout);
        assert_eq!(code[5], Instruction::Sleep(500));
        assert_eq!(
            code[6],
            Instruction::Push(StackValue::String("checkout".to_string()))
        );
        assert_eq!(
            code[7],
            Instruction::Push(StackValue::String("process".to_string()))
        );
        assert_eq!(code[8], Instruction::RemoteCall);
        assert_eq!(code[9], Instruction::Jump("main".to_string()));
        assert_eq!(code[10], Instruction::Label("end_charge".to_string()));
        //--
        assert_eq!(code[11], Instruction::Label("main".to_string()));
        assert_eq!(code[12], Instruction::Jump("charge".to_string()));
        assert_eq!(code[13], Instruction::Label("end_main".to_string()));
    }
}
