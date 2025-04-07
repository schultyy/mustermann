use instruction::{Instruction, StackValue};

use crate::parser::{Method, Program, Service, Statement};

pub mod error;
pub mod instruction;

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

#[derive(Debug, Clone, PartialEq)]
pub enum PrintType {
    Stdout,
    Stderr,
}

pub struct CodeGenerator<'a> {
    ast: &'a Service,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(ast: &'a Service) -> Self {
        Self { ast }
    }

    pub fn process(&self) -> Result<Vec<Instruction>, CodeGenError> {
        self.process_service(self.ast)
    }

    fn process_service(&self, service: &'a Service) -> Result<Vec<Instruction>, CodeGenError> {
        let mut instructions = Vec::new();
        instructions.push(Instruction::Label(format!("start_{}", service.name)));
        instructions.push(Instruction::Jump(format!("start_{}_main", service.name)));
        for method in &service.methods {
            instructions.extend(self.process_method(method)?);
        }
        instructions.push(Instruction::Label(format!("start_{}_main", service.name)));
        if let Some(loop_def) = service.loops.first() {
            self.process_loop(&mut instructions, &loop_def)?;
        } else {
            instructions.push(Instruction::CheckInterrupt);
            instructions.push(Instruction::Jump(format!("start_{}_main", service.name)));
        }
        instructions.push(Instruction::Label(format!("end_{}_main", service.name)));
        instructions.push(Instruction::Label(format!("end_{}", service.name)));
        Ok(instructions)
    }

    fn process_loop(
        &self,
        instructions: &mut Vec<Instruction>,
        loop_def: &crate::parser::Loop,
    ) -> Result<(), CodeGenError> {
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
        Ok(())
    }

    fn process_method(&self, method: &'a Method) -> Result<Vec<Instruction>, CodeGenError> {
        let mut instructions = Vec::new();
        instructions.push(Instruction::Label(format!("start_{}", method.name)));
        for statement in &method.statements {
            match statement {
                Statement::Stdout { message, args } => {
                    instructions.extend(self.process_print(message, args, PrintType::Stdout));
                }
                Statement::Sleep { duration } => {
                    instructions.push(Instruction::Sleep(duration.as_millis() as u64));
                }
                Statement::Call { service, method } => {
                    if let Some(service) = service {
                        instructions.push(Instruction::Push(StackValue::String(service.clone())));
                        instructions.push(Instruction::Push(StackValue::String(method.clone())));
                        instructions.push(Instruction::RemoteCall);
                    } else {
                        return Err(CodeGenError::InvalidStatement(format!(
                            "Expected Remote Call - Got {}",
                            statement.to_string()
                        )));
                    }
                }
                Statement::Stderr { message, args } => {
                    instructions.extend(self.process_print(message, args, PrintType::Stderr));
                }
            }
        }
        instructions.push(Instruction::Ret);
        instructions.push(Instruction::Label(format!("end_{}", method.name)));
        Ok(instructions)
    }

    fn process_print(
        &self,
        message: &str,
        args: &Option<Vec<String>>,
        print_type: PrintType,
    ) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        if let Some(args) = args {
            for arg in args {
                instructions.push(Instruction::Push(StackValue::String(message.to_string())));
                instructions.push(Instruction::Push(StackValue::String(arg.to_string())));
                instructions.push(Instruction::Printf);
                match print_type {
                    PrintType::Stdout => instructions.push(Instruction::Stdout),
                    PrintType::Stderr => instructions.push(Instruction::Stderr),
                }
            }
        } else {
            instructions.push(Instruction::Push(StackValue::String(message.to_string())));
            match print_type {
                PrintType::Stdout => instructions.push(Instruction::Stdout),
                PrintType::Stderr => instructions.push(Instruction::Stderr),
            }
        }
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

    fn service_with_template() -> String {
        "
        service products {
            method get_products {
                print \"Fetching product orders %s\" with [\"12345\", \"67890\"]
                sleep 500ms
            }
        }
        "
        .to_string()
    }

    fn service_with_stderr_template() -> String {
        "
        service products {
            method get_products {
                stderr \"Fetching product orders %s\" with [\"12345\", \"67890\"]
                sleep 500ms
            }
        }
        "
        .to_string()
    }

    fn service_with_template_and_empty_var_list() -> String {
        "
        service products {
            method get_products {
                print \"Fetching product orders %s\" with []
                sleep 500ms
            }
        }
        "
        .to_string()
    }

    fn service_with_stderr_template_and_empty_var_list() -> String {
        "
        service products {
            method get_products {
                stderr \"Fetching product orders %s\" with []
                sleep 500ms
            }
        }
        "
        .to_string()
    }

    fn call_other_service() -> String {
        "
        service products {
            method get_products {
                print \"Fetching product orders %s\" with [\"12345\", \"67890\"]
                sleep 500ms
            }
        }

        service frontend {
            method main_page {
                call products.get_products
            }

            loop {
                call main_page
            }
        }
        "
        .to_string()
    }

    fn call_other_service_without_loop() -> String {
        "
        service products {
            method get_products {
                print \"Fetching product orders %s\" with [\"12345\", \"67890\"]
                sleep 500ms
            }
        }

        service frontend {
            method main_page {
                call products.get_products
            }
        }
        "
        .to_string()
    }

    #[test]
    fn test_log_byte_code() {
        let service = service();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let expected = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("start_main_page".to_string()),
            Instruction::Push(StackValue::String("Main page".to_string())),
            Instruction::Stdout,
            Instruction::Ret,
            Instruction::Label("end_main_page".to_string()),
            Instruction::Label("start_frontend_main".to_string()),
            Instruction::CheckInterrupt,
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("end_frontend_main".to_string()),
            Instruction::Label("end_frontend".to_string()),
        ];
        assert_eq!(code, expected);
    }

    #[test]
    fn test_service_with_sleep() {
        let service = service_with_sleep();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let expected = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("start_main_page".to_string()),
            Instruction::Push(StackValue::String("Main page".to_string())),
            Instruction::Stdout,
            Instruction::Sleep(1000),
            Instruction::Ret,
            Instruction::Label("end_main_page".to_string()),
            Instruction::Label("start_frontend_main".to_string()),
            Instruction::CheckInterrupt,
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("end_frontend_main".to_string()),
            Instruction::Label("end_frontend".to_string()),
        ];
        assert_eq!(code, expected);
    }

    #[test]
    fn test_service_with_main() {
        let service = service_with_main();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();
        let expected = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Jump("start_frontend_main".to_string()),
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
        assert_eq!(code, expected);
    }

    #[test]
    fn test_service_with_template() {
        let service = service_with_template();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let expected = vec![
            Instruction::Label("start_products".to_string()),
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("start_get_products".to_string()),
            Instruction::Push(StackValue::String("Fetching product orders %s".to_string())),
            Instruction::Push(StackValue::String("12345".to_string())),
            Instruction::Printf,
            Instruction::Stdout,
            Instruction::Push(StackValue::String("Fetching product orders %s".to_string())),
            Instruction::Push(StackValue::String("67890".to_string())),
            Instruction::Printf,
            Instruction::Stdout,
            Instruction::Sleep(500),
            Instruction::Ret,
            Instruction::Label("end_get_products".to_string()),
            Instruction::Label("start_products_main".to_string()),
            Instruction::CheckInterrupt,
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("end_products_main".to_string()),
            Instruction::Label("end_products".to_string()),
        ];
        assert_eq!(code, expected);
    }

    #[test]
    fn test_service_with_template_and_empty_var_list() {
        let service = service_with_template_and_empty_var_list();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let expected = vec![
            Instruction::Label("start_products".to_string()),
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("start_get_products".to_string()),
            Instruction::Sleep(500),
            Instruction::Ret,
            Instruction::Label("end_get_products".to_string()),
            Instruction::Label("start_products_main".to_string()),
            Instruction::CheckInterrupt,
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("end_products_main".to_string()),
            Instruction::Label("end_products".to_string()),
        ];
        assert_eq!(code, expected);
    }

    #[test]
    fn test_service_with_stderr_template() {
        let service = service_with_stderr_template();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let expected = vec![
            Instruction::Label("start_products".to_string()),
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("start_get_products".to_string()),
            Instruction::Push(StackValue::String("Fetching product orders %s".to_string())),
            Instruction::Push(StackValue::String("12345".to_string())),
            Instruction::Printf,
            Instruction::Stderr,
            Instruction::Push(StackValue::String("Fetching product orders %s".to_string())),
            Instruction::Push(StackValue::String("67890".to_string())),
            Instruction::Printf,
            Instruction::Stderr,
            Instruction::Sleep(500),
            Instruction::Ret,
            Instruction::Label("end_get_products".to_string()),
            Instruction::Label("start_products_main".to_string()),
            Instruction::CheckInterrupt,
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("end_products_main".to_string()),
            Instruction::Label("end_products".to_string()),
        ];
        assert_eq!(code, expected);
    }

    #[test]
    fn test_service_with_stderr_template_and_empty_var_list() {
        let service = service_with_stderr_template_and_empty_var_list();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let expected = vec![
            Instruction::Label("start_products".to_string()),
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("start_get_products".to_string()),
            Instruction::Sleep(500),
            Instruction::Ret,
            Instruction::Label("end_get_products".to_string()),
            Instruction::Label("start_products_main".to_string()),
            Instruction::CheckInterrupt,
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("end_products_main".to_string()),
            Instruction::Label("end_products".to_string()),
        ];
        assert_eq!(code, expected);
    }

    #[test]
    fn test_call_other_service() {
        let service = call_other_service();
        let ast = parser::parse(&service).unwrap();
        let products_code = CodeGenerator::new(&ast.services[0]).process().unwrap();
        let frontend_code = CodeGenerator::new(&ast.services[1]).process().unwrap();

        let expected_products = vec![
            Instruction::Label("start_products".to_string()),
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("start_get_products".to_string()),
            Instruction::Push(StackValue::String("Fetching product orders %s".to_string())),
            Instruction::Push(StackValue::String("12345".to_string())),
            Instruction::Printf,
            Instruction::Stdout,
            Instruction::Push(StackValue::String("Fetching product orders %s".to_string())),
            Instruction::Push(StackValue::String("67890".to_string())),
            Instruction::Printf,
            Instruction::Stdout,
            Instruction::Sleep(500),
            Instruction::Ret,
            Instruction::Label("end_get_products".to_string()),
            Instruction::Label("start_products_main".to_string()),
            Instruction::CheckInterrupt,
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("end_products_main".to_string()),
            Instruction::Label("end_products".to_string()),
        ];
        assert_eq!(products_code, expected_products);

        let expected_frontend = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("start_main_page".to_string()),
            Instruction::Push(StackValue::String("products".to_string())),
            Instruction::Push(StackValue::String("get_products".to_string())),
            Instruction::RemoteCall,
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
        assert_eq!(frontend_code, expected_frontend);
    }

    #[test]
    fn test_call_other_service_without_loop() {
        let service = call_other_service_without_loop();
        let ast = parser::parse(&service).unwrap();
        let products_code = CodeGenerator::new(&ast.services[0]).process().unwrap();
        let frontend_code = CodeGenerator::new(&ast.services[1]).process().unwrap();

        let expected_products = vec![
            Instruction::Label("start_products".to_string()),
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("start_get_products".to_string()),
            Instruction::Push(StackValue::String("Fetching product orders %s".to_string())),
            Instruction::Push(StackValue::String("12345".to_string())),
            Instruction::Printf,
            Instruction::Stdout,
            Instruction::Push(StackValue::String("Fetching product orders %s".to_string())),
            Instruction::Push(StackValue::String("67890".to_string())),
            Instruction::Printf,
            Instruction::Stdout,
            Instruction::Sleep(500),
            Instruction::Ret,
            Instruction::Label("end_get_products".to_string()),
            Instruction::Label("start_products_main".to_string()),
            Instruction::CheckInterrupt,
            Instruction::Jump("start_products_main".to_string()),
            Instruction::Label("end_products_main".to_string()),
            Instruction::Label("end_products".to_string()),
        ];
        assert_eq!(products_code, expected_products);

        let expected_frontend = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("start_main_page".to_string()),
            Instruction::Push(StackValue::String("products".to_string())),
            Instruction::Push(StackValue::String("get_products".to_string())),
            Instruction::RemoteCall,
            Instruction::Ret,
            Instruction::Label("end_main_page".to_string()),
            Instruction::Label("start_frontend_main".to_string()),
            Instruction::CheckInterrupt,
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("end_frontend_main".to_string()),
            Instruction::Label("end_frontend".to_string()),
        ];
        assert_eq!(frontend_code, expected_frontend);
    }
}
