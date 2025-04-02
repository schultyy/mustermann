use crate::config::{Count, Severity, Task};

use super::{
    error::ByteCodeError,
    instruction::{Instruction, StackValue},
};

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
