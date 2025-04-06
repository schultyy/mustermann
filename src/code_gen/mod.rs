use instruction::{Instruction, StackValue};

use crate::parser::{Method, Program, Service, Statement};

pub mod error;
pub mod instruction;
pub mod log_byte_code;
pub mod service_byte_code;

#[derive(Debug, Clone)]
pub enum CodeGenError {
    InvalidStatement(String),
}

impl std::fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodeGenError::InvalidStatement(msg) => write!(f, "Invalid statement: {}", msg),
        }
    }
}

impl std::error::Error for CodeGenError {}

struct CodeGenerator<'a> {
    ast: &'a Program,
}

impl<'a> CodeGenerator<'a> {
    fn new(ast: &'a Program) -> Self {
        Self { ast }
    }

    /// Process the AST into a list of instructions
    /// Returns a list of instruction lists. One for each service.
    fn process(&self) -> Result<Vec<Vec<Instruction>>, CodeGenError> {
        let mut instructions = vec![];
        for service in &self.ast.services {
            instructions.push(self.process_service(service)?);
        }
        Ok(instructions)
    }

    fn process_service(&self, service: &'a Service) -> Result<Vec<Instruction>, CodeGenError> {
        let mut instructions = Vec::new();
        instructions.push(Instruction::Label(format!("start_{}", service.name)));
        for method in &service.methods {
            instructions.extend(self.process_method(method));
        }
        instructions.push(Instruction::Label(format!("start_{}_main", service.name)));
        if let Some(loop_def) = service.loops.first() {
            if let Some(statements) = loop_def.statements.first() {
                instructions.push(Instruction::Label("start_loop".to_string()));
                match statements {
                    Statement::Call { service, method } => {
                        if let Some(_service) = service {
                            return Err(CodeGenError::InvalidStatement(format!(
                                "Expected Local Call - Got {}",
                                statements.to_string()
                            )));
                        }
                        instructions.push(Instruction::Call(format!("start_{}", method)));
                    }
                    _ => {
                        return Err(CodeGenError::InvalidStatement(format!(
                            "Expected Call - Got {}",
                            statements.to_string()
                        )));
                    }
                }
                instructions.push(Instruction::Jump(format!("start_loop")));
                instructions.push(Instruction::Label("end_loop".to_string()));
            }
        } else {
            instructions.push(Instruction::Nop);
            instructions.push(Instruction::Jump(format!("start_{}_main", service.name)));
        }
        instructions.push(Instruction::Label(format!("end_{}_main", service.name)));
        instructions.push(Instruction::Label(format!("end_{}", service.name)));
        Ok(instructions)
    }

    fn process_method(&self, method: &'a Method) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        instructions.push(Instruction::Label(format!("start_{}", method.name)));
        for statement in &method.statements {
            match statement {
                Statement::Print { message, args } => {
                    instructions.push(Instruction::Push(StackValue::String(message.clone())));
                    instructions.push(Instruction::Stdout);
                }
                Statement::Sleep { duration } => {
                    instructions.push(Instruction::Sleep(duration.as_millis() as u64));
                }
                Statement::Call { service, method } => {
                    // instructions.push(Instruction::Call(service.clone(), method.clone()));
                }
            }
        }
        instructions.push(Instruction::Ret);
        instructions.push(Instruction::Label(format!("end_{}", method.name)));
        instructions
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        code_gen::{
            instruction::{Instruction, StackValue},
            CodeGenerator,
        },
        parser,
    };

    fn service() -> String {
        "
        service frontend {
            method main_page {
                print \"Main page\"
            }
        }
        "
        .to_string()
    }

    fn service_with_sleep() -> String {
        "
        service frontend {
            method main_page {
                print \"Main page\"
                sleep 1000ms
            }
        }
        "
        .to_string()
    }

    fn service_with_main() -> String {
        "
        service frontend {
            method main_page {
                print \"Main page\"
                sleep 1000ms
            }

            loop {
                call main_page
            }
        }
        "
        .to_string()
    }

    #[test]
    fn test_log_byte_code() {
        let service = service();
        let ast = parser::parse(&service).unwrap();
        let codes = CodeGenerator::new(&ast).process().unwrap();
        let code = codes.first().unwrap();

        let expected = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Label("start_main_page".to_string()),
            Instruction::Push(StackValue::String("Main page".to_string())),
            Instruction::Stdout,
            Instruction::Ret,
            Instruction::Label("end_main_page".to_string()),
            Instruction::Label("start_frontend_main".to_string()),
            Instruction::Nop,
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("end_frontend_main".to_string()),
            Instruction::Label("end_frontend".to_string()),
        ];
        assert_eq!(code, &expected);
    }

    #[test]
    fn test_service_with_sleep() {
        let service = service_with_sleep();
        let ast = parser::parse(&service).unwrap();
        let codes = CodeGenerator::new(&ast).process().unwrap();
        let code = codes.first().unwrap();

        let expected = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Label("start_main_page".to_string()),
            Instruction::Push(StackValue::String("Main page".to_string())),
            Instruction::Stdout,
            Instruction::Sleep(1000),
            Instruction::Ret,
            Instruction::Label("end_main_page".to_string()),
            Instruction::Label("start_frontend_main".to_string()),
            Instruction::Nop,
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("end_frontend_main".to_string()),
            Instruction::Label("end_frontend".to_string()),
        ];
        assert_eq!(code, &expected);
    }

    #[test]
    fn test_service_with_main() {
        let service = service_with_main();
        let ast = parser::parse(&service).unwrap();
        let codes = CodeGenerator::new(&ast).process().unwrap();
        let code = codes.first().unwrap();
        let expected = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Label("start_main_page".to_string()),
            Instruction::Push(StackValue::String("Main page".to_string())),
            Instruction::Stdout,
            Instruction::Sleep(1000),
            Instruction::Ret,
            Instruction::Label("end_main_page".to_string()),
            Instruction::Label("start_frontend_main".to_string()),
            Instruction::Label("start_loop".to_string()),
            Instruction::Call("start_main_page".to_string()),
            Instruction::Jump("start_loop".to_string()),
            Instruction::Label("end_loop".to_string()),
            Instruction::Label("end_frontend_main".to_string()),
            Instruction::Label("end_frontend".to_string()),
        ];
        assert_eq!(code, &expected);
    }
}
