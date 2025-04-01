use crate::config::{Method, Service};

use super::{
    error::ByteCodeError,
    instruction::{Instruction, StackValue},
};

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
        if let Some(invoke) = &self.service.invoke {
            for method in invoke {
                code.push(Instruction::Jump(format!("{}", method)));
            }
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
