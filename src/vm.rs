use std::collections::HashMap;

use opentelemetry::trace::FutureExt;
use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry::{
    trace::{SpanKind, TraceContextExt, Tracer},
    Context,
};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{Builder, RandomIdGenerator, Span, TracerProvider};
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tokio::sync::mpsc;
use tonic::metadata::{MetadataMap, MetadataValue};
use tracing::Instrument;

use crate::{
    code_gen::instruction::{Instruction, StackValue},
    vm_coordinator::ServiceMessage,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMError {
    StackUnderflow,
    InvalidStackValue,
    MissingAppName,
    MissingVar(String),
    RemoteCallError,
    MissingLabel(String),
    MissingSpan,
}

impl std::error::Error for VMError {}

impl std::fmt::Display for VMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VMError::StackUnderflow => write!(f, "Stack underflow"),
            VMError::InvalidStackValue => write!(f, "Invalid stack value"),
            VMError::MissingAppName => write!(f, "Missing app name"),
            VMError::MissingVar(var) => write!(f, "Missing variable: {}", var),
            VMError::RemoteCallError => write!(f, "Remote call error"),
            VMError::MissingLabel(label) => write!(f, "Missing label: {}", label),
            VMError::MissingSpan => write!(f, "Missing span"),
        }
    }
}

pub struct VM {
    code: Vec<Instruction>,
    stack: Vec<StackValue>,
    vars: HashMap<String, StackValue>,
    ip: usize,
    // on_stdout: Arc<Box<dyn Fn(&str, &str) -> ()>>, //name, message
    // on_stderr: Arc<Box<dyn Fn(&str, &str) -> ()>>, //name, message
    tx: Option<mpsc::Sender<ServiceMessage>>,
    rx: Option<mpsc::Receiver<String>>,
    message_check_counter: usize,
    tracer: Option<TracerProvider>,
    context: Option<Context>,
}

pub fn setup_tracer(
    endpoint: &str,
    service_name: &str,
) -> Result<TracerProvider, opentelemetry::trace::TraceError> {
    let mut map = MetadataMap::with_capacity(3);

    map.insert("x-application", service_name.parse().unwrap());
    map.insert_bin(
        "trace-proto-bin",
        MetadataValue::from_bytes(b"[binary data]"),
    );

    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_export_config(opentelemetry_otlp::ExportConfig {
            endpoint: Some(endpoint.to_string()),
            ..Default::default()
        })
        .with_timeout(std::time::Duration::from_secs(3))
        .with_metadata(map)
        .build()?;

    let provider = Builder::default()
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(64)
        .with_max_attributes_per_span(16)
        .with_resource(Resource::new(vec![KeyValue::new(
            SERVICE_NAME,
            service_name.to_string(),
        )]))
        .with_batch_exporter(otlp_exporter, opentelemetry_sdk::runtime::Tokio)
        .build();
    // Then pass it into provider builder
    global::set_text_map_propagator(TraceContextPropagator::new());
    Ok(provider)
}

impl VM {
    pub fn with_tracer(
        code: Vec<Instruction>,
        tx: Option<mpsc::Sender<ServiceMessage>>,
        rx: Option<mpsc::Receiver<String>>,
        tracer: Option<TracerProvider>,
    ) -> Result<Self, opentelemetry::trace::TraceError> {
        Ok(Self {
            code,
            stack: Vec::new(),
            vars: HashMap::new(),
            ip: 0,
            context: None,
            tx,
            rx,
            message_check_counter: 0,
            tracer,
        })
    }

    pub async fn run(&mut self) -> Result<(), VMError> {
        while self.ip < self.code.len() {
            let instruction = self.code[self.ip].clone();
            self.ip += 1;
            self.message_check_counter += 1;
            if self.message_check_counter > 10000 {
                if let Some(rx) = &mut self.rx {
                    if let Ok(msg) = rx.try_recv() {
                        self.handle_service_message(msg)?;
                    }
                }
                self.message_check_counter = 0;
            }
            self.execute_instruction(instruction).await?;
        }
        Ok(())
    }

    fn handle_service_message(&mut self, msg: String) -> Result<(), VMError> {
        self.ip = self
            .code
            .iter()
            .position(|i| i == &Instruction::Label(msg.clone()))
            .ok_or(VMError::MissingLabel(msg.clone()))?;
        Ok(())
    }

    async fn execute_instruction(&mut self, instruction: Instruction) -> Result<(), VMError> {
        match instruction {
            Instruction::Push(stack_value) => {
                self.stack.push(stack_value.to_owned());
            }
            Instruction::Pop => {
                self.stack.pop();
            }
            Instruction::Dec => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::Int(n) => self.stack.push(StackValue::Int(n - 1)),
                    _ => return Err(VMError::InvalidStackValue),
                }
            }
            Instruction::JmpIfZero(label) => {
                let top = self.stack.last().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::Int(0) => {
                        self.ip = self
                            .code
                            .iter()
                            .position(|i| i == &Instruction::Label(label.clone()))
                            .unwrap();
                    }
                    _ => {}
                }
            }
            Instruction::Label(_) => {
                // Labels are used for jumps and are not executed
            }
            Instruction::Stdout => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                let name = self.vars.get("name").ok_or(VMError::MissingAppName)?;
                match top {
                    StackValue::String(s) => {
                        tracing::info!("{}: {}", name, s);
                    }
                    StackValue::Int(n) => {
                        tracing::info!("{}: {}", name, n);
                    }
                }
            }
            Instruction::Stderr => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::String(s) => {
                        let name = self.vars.get("name").ok_or(VMError::MissingAppName)?;
                        tracing::error!("{}: {}", name, s);
                    }
                    _ => return Err(VMError::InvalidStackValue),
                }
            }
            Instruction::Sleep(ms) => {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
            Instruction::StoreVar(key, value) => {
                self.vars
                    .insert(key.clone(), StackValue::String(value.clone()));
            }
            Instruction::LoadVar(key) => {
                let value = self
                    .vars
                    .get(&key)
                    .ok_or(VMError::MissingVar(key.clone()))?;
                self.stack.push(value.clone());
            }
            Instruction::Dup => {
                let top = self.stack.last().ok_or(VMError::StackUnderflow)?;
                self.stack.push(top.clone());
            }
            Instruction::Jump(label) => {
                self.ip = self
                    .code
                    .iter()
                    .position(|i| i == &Instruction::Label(label.clone()))
                    .ok_or(VMError::MissingLabel(label.clone()))?;
            }
            Instruction::Printf => {
                let template = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                let template = match template {
                    StackValue::String(s) => s,
                    _ => return Err(VMError::InvalidStackValue),
                };
                let var = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                let var = match var {
                    StackValue::String(s) => s,
                    _ => return Err(VMError::InvalidStackValue),
                };

                let formatted = template.replace("%s", &var);
                self.stack.push(StackValue::String(formatted));
            }
            Instruction::RemoteCall => {
                if let Some(tx) = self.tx.as_ref() {
                    let method = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                    let service = self.stack.pop().ok_or(VMError::StackUnderflow)?;

                    //find the previous label in code based on current
                    let mut function_name = "default".into();
                    for i in (0..self.ip).rev() {
                        if matches!(self.code[i], Instruction::Label(_)) {
                            match self.code[i].clone() {
                                Instruction::Label(label) => function_name = label,
                                _ => {}
                            }
                            break;
                        }
                    }

                    let service_name = self.vars.get("name").unwrap();
                    let service_name = service_name.to_string();
                    if let Some(tracer_provider) = self.tracer.as_ref() {
                        if let Some(otel_cx) = self.context.as_ref() {
                            let tracer = tracer_provider.tracer(service_name.clone());
                            let _span = tracer
                                .span_builder(format!("{}/{}", service_name, function_name))
                                .with_kind(SpanKind::Server)
                                .with_context(otel_cx.clone());
                            // let cx = Context::current_with_span(span);
                            // global::get_text_map_propagator(|propagator| {
                            //     propagator.inject_context(&cx, &mut metadata)
                            // });
                        }
                    }

                    let service = match service {
                        StackValue::String(s) => s,
                        _ => return Err(VMError::InvalidStackValue),
                    };
                    let method = match method {
                        StackValue::String(s) => s,
                        _ => return Err(VMError::InvalidStackValue),
                    };

                    tx.send(ServiceMessage::Call {
                        to: service,
                        function: method,
                        context: self.context.clone().unwrap_or_else(|| Context::current()),
                    })
                    .await
                    .or(Err(VMError::RemoteCallError))?;

                    tracing::info!("Remote call initiated");
                }
            }
            Instruction::StartContext => {
                if let Some(tracer_provider) = self.tracer.as_ref() {
                    let service_name = self.vars.get("name").unwrap();
                    let service_name = service_name.to_string();
                    let mut metadata = HashMap::new();
                    let tracer = tracer_provider.tracer(service_name.clone());
                    let span = tracer
                        .span_builder(format!("{}/{}", service_name, "start_context"))
                        .with_kind(SpanKind::Server)
                        .start(&tracer);
                    let cx = Context::current_with_span(span);
                    global::get_text_map_propagator(|propagator| {
                        propagator.inject_context(&cx, &mut metadata)
                    });
                    self.context = Some(cx);
                }
            }
            Instruction::EndContext => match self.context.as_mut() {
                Some(_) => {
                    self.context = None;
                }
                None => {
                    return Err(VMError::MissingSpan);
                }
            },
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vm_run() {
        let mut vm = VM::with_tracer(
            vec![Instruction::StoreVar(
                "name".to_string(),
                "test".to_string(),
            )],
            None,
            None,
            None,
        )
        .unwrap();
        vm.run().await.unwrap();
    }
}
