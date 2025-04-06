use instruction::{Instruction, StackValue};

use crate::parser::{Method, Program, Service, Statement};

pub mod error;
pub mod instruction;
pub mod log_byte_code;
pub mod service_byte_code;

struct CodeGenerator<'a> {
    ast: &'a Program,
}

impl<'a> CodeGenerator<'a> {
    fn new(ast: &'a Program) -> Self {
        Self { ast }
    }

    fn process(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        for service in &self.ast.services {
            instructions.extend(self.process_service(service));
        }
        instructions
    }

    fn process_service(&self, service: &'a Service) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        instructions.push(Instruction::Label(format!("start_{}", service.name)));
        for method in &service.methods {
            instructions.extend(self.process_method(method));
        }
        instructions.push(Instruction::Label(format!("start_{}_main", service.name)));
        instructions.push(Instruction::Jump(format!("start_{}_main", service.name)));
        instructions.push(Instruction::Label(format!("end_{}_main", service.name)));
        instructions.push(Instruction::Label(format!("end_{}", service.name)));
        instructions
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

    #[test]
    fn test_log_byte_code() {
        let service = service();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast).process();

        let expected = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Label("start_main_page".to_string()),
            Instruction::Push(StackValue::String("Main page".to_string())),
            Instruction::Stdout,
            Instruction::Label("end_main_page".to_string()),
            Instruction::Label("start_frontend_main".to_string()),
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
        let code = CodeGenerator::new(&ast).process();

        let expected = vec![
            Instruction::Label("start_frontend".to_string()),
            Instruction::Label("start_main_page".to_string()),
            Instruction::Push(StackValue::String("Main page".to_string())),
            Instruction::Stdout,
            Instruction::Sleep(1000),
            Instruction::Label("end_main_page".to_string()),
            Instruction::Label("start_frontend_main".to_string()),
            Instruction::Jump("start_frontend_main".to_string()),
            Instruction::Label("end_frontend_main".to_string()),
            Instruction::Label("end_frontend".to_string()),
        ];
        assert_eq!(code, expected);
    }
}
