use std::collections::HashMap;

use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::trace::{TraceContextExt, TracerProvider};
use opentelemetry::{global, KeyValue};
use opentelemetry::{
    trace::{SpanKind, Tracer},
    Context,
};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tokio::sync::mpsc;
use tonic::metadata::{MetadataMap, MetadataValue};

use crate::code_gen::instruction::{Instruction, StackValue};
use crate::vm_coordinator::ServiceMessage;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMError {
    StackUnderflow,
    InvalidStackValue,
    MissingVar(String),
    RemoteCallError(String),
    MissingLabel(String),
    MissingSpan,
    PrintError(mpsc::error::SendError<PrintMessage>),
    MaxExecutionCounterReached,
    InvalidTemplate(String),
    IPOutOfBounds(usize, usize),
    MissingFunctionName,
    MissingContext,
}

impl std::error::Error for VMError {}

impl std::fmt::Display for VMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VMError::StackUnderflow => write!(f, "Stack underflow"),
            VMError::InvalidStackValue => write!(f, "Invalid stack value"),
            VMError::MissingVar(var) => write!(f, "Missing variable: {}", var),
            VMError::RemoteCallError(msg) => write!(f, "Remote call error: {}", msg),
            VMError::MissingLabel(label) => write!(f, "Missing label: {}", label),
            VMError::MissingSpan => write!(f, "Missing span"),
            VMError::PrintError(err) => write!(f, "Print error: {}", err),
            VMError::MaxExecutionCounterReached => write!(f, "Max execution counter reached"),
            VMError::InvalidTemplate(template) => write!(f, "Invalid template: {}", template),
            VMError::IPOutOfBounds(ip, len) => {
                write!(
                    f,
                    "Instruction Pointer out of bounds: {} | No of instructions: {}",
                    ip, len
                )
            }
            VMError::MissingFunctionName => write!(f, "Missing function name"),
            VMError::MissingContext => write!(f, "Missing context"),
        }
    }
}

pub fn setup_tracer(
    endpoint: &str,
    service_name: &str,
) -> Result<SdkTracerProvider, opentelemetry_otlp::ExporterBuildError> {
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
            protocol: opentelemetry_otlp::Protocol::Grpc,
            timeout: Some(std::time::Duration::from_secs(3)),
        })
        .with_metadata(map)
        .build()?;

    let resource = Resource::builder()
        .with_attribute(KeyValue::new(SERVICE_NAME, service_name.to_string()))
        .build();
    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(otlp_exporter)
        .build();

    // Then pass it into provider builder
    global::set_text_map_propagator(TraceContextPropagator::new());
    Ok(provider)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintMessage {
    Stdout(String),
    Stderr(String),
}

pub struct VM {
    code: Vec<Instruction>,
    stack: Vec<StackValue>,
    vars: HashMap<String, StackValue>,
    ip: usize,
    print_tx: mpsc::Sender<PrintMessage>,
    max_execution_counter: Option<usize>,
    return_addresses: Vec<usize>,
    remote_call_tx: Option<mpsc::Sender<ServiceMessage>>,
    remote_call_rx: Option<mpsc::Receiver<String>>,
    remote_call_counter: usize,
    remote_call_limit: usize,
    service_name: String,
    tracer: Option<SdkTracerProvider>,
    otel_context: Option<opentelemetry::Context>,
}

impl VM {
    pub fn new(
        code: Vec<Instruction>,
        service_name: &str,
        print_tx: mpsc::Sender<PrintMessage>,
    ) -> Self {
        Self {
            code,
            stack: Vec::new(),
            vars: HashMap::new(),
            ip: 0,
            print_tx,
            max_execution_counter: None,
            return_addresses: Vec::new(),
            remote_call_tx: None,
            remote_call_rx: None,
            remote_call_counter: 0,
            remote_call_limit: 10000,
            service_name: service_name.to_string(),
            tracer: None,
            otel_context: None,
        }
    }

    pub fn with_max_execution_counter(mut self, max_execution_counter: usize) -> Self {
        self.max_execution_counter = Some(max_execution_counter);
        self
    }

    pub fn with_remote_call_tx(mut self, remote_call_tx: mpsc::Sender<ServiceMessage>) -> Self {
        self.remote_call_tx = Some(remote_call_tx);
        self
    }

    pub fn with_remote_call_rx(mut self, remote_call_rx: mpsc::Receiver<String>) -> Self {
        self.remote_call_rx = Some(remote_call_rx);
        self
    }

    pub fn with_custom_remote_call_limit(mut self, limit: usize) -> Self {
        self.remote_call_limit = limit;
        self
    }

    pub fn with_tracer(mut self, tracer: SdkTracerProvider) -> Self {
        self.tracer = Some(tracer);
        self
    }

    pub async fn run(&mut self) -> Result<(), VMError> {
        let mut execution_counter = 0;
        while self.ip < self.code.len() {
            self.ip += 1;
            if self.ip >= self.code.len() {
                return Err(VMError::IPOutOfBounds(self.ip, self.code.len()));
            }
            let instruction = self.code[self.ip].clone();
            self.execute_instruction(instruction).await?;
            execution_counter += 1;
            if let Some(max_execution_counter) = self.max_execution_counter {
                if execution_counter > max_execution_counter {
                    return Err(VMError::MaxExecutionCounterReached);
                }
            }
        }
        Ok(())
    }

    async fn handle_remote_call(&mut self) -> Result<(), VMError> {
        if let Some(remote_call_rx) = &mut self.remote_call_rx {
            self.remote_call_counter += 1;
            if self.remote_call_counter > self.remote_call_limit {
                if let Ok(msg) = remote_call_rx.try_recv() {
                    let label_name = format!("start_{}", msg);
                    self.handle_local_call(label_name).await?;
                }
                self.remote_call_counter = 0;
            }
        }
        Ok(())
    }

    async fn handle_local_call(&mut self, label: String) -> Result<(), VMError> {
        self.return_addresses.push(self.ip);
        let jump_to = self
            .code
            .iter()
            .position(|i| i == &Instruction::Label(label.clone()));

        if let Some(jump_to) = jump_to {
            self.ip = jump_to;
        } else {
            return Err(VMError::MissingLabel(label.clone()));
        }
        Ok(())
    }

    async fn execute_instruction(&mut self, instruction: Instruction) -> Result<(), VMError> {
        tracing::debug!("Executing instruction: {:?}", instruction);
        match instruction {
            Instruction::Push(stack_value) => {
                self.stack.push(stack_value);
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
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
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
            Instruction::Label(_) => { /* Labels are used for jumps and are not executed */ }
            Instruction::Stdout => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                tracing::debug!("Sending stdout: {:?}", top);
                match top {
                    StackValue::String(s) => {
                        self.print_tx
                            .send(PrintMessage::Stdout(s))
                            .await
                            .map_err(VMError::PrintError)?;
                    }
                    _ => return Err(VMError::InvalidStackValue),
                }
            }
            Instruction::Stderr => {
                let top = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::String(s) => {
                        self.print_tx
                            .send(PrintMessage::Stderr(s))
                            .await
                            .map_err(VMError::PrintError)?;
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
                    .unwrap();
            }
            Instruction::Printf => {
                let var = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                let var = match var {
                    StackValue::String(s) => s,
                    _ => return Err(VMError::InvalidStackValue),
                };
                let template = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                let template = match template {
                    StackValue::String(s) => s,
                    _ => return Err(VMError::InvalidStackValue),
                };

                if template.contains("%s") {
                    let formatted = template.replace("%s", &var);
                    self.stack.push(StackValue::String(formatted));
                } else {
                    return Err(VMError::InvalidTemplate(template.clone()));
                }
            }
            Instruction::RemoteCall => {
                if let Some(remote_call_tx) = self.remote_call_tx.as_ref() {
                    let remote_method = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                    let remote_service = self.stack.pop().ok_or(VMError::StackUnderflow)?;
                    let local_function_name = self
                        .find_current_function_name()
                        .ok_or(VMError::MissingFunctionName)?;
                    let mut cx = None;

                    if let Some(tracer_provider) = self.tracer.as_ref() {
                        if let Some(otel_cx) = self.otel_context.as_ref() {
                            let tracer = tracer_provider.tracer(self.service_name.clone());

                            let span = tracer
                                .span_builder(format!(
                                    "{}/{}",
                                    self.service_name, local_function_name
                                ))
                                .with_kind(SpanKind::Client)
                                .with_attributes(vec![KeyValue::new(
                                    SERVICE_NAME,
                                    self.service_name.clone(),
                                )])
                                .start(&tracer);

                            cx = Some(otel_cx.with_span(span));
                            let mut metadata = HashMap::new();
                            let propagator = TraceContextPropagator::new();
                            propagator.inject_context(&cx.clone().unwrap(), &mut metadata);
                        } else {
                            return Err(VMError::MissingContext);
                        }
                    }

                    remote_call_tx
                        .send(ServiceMessage::Call {
                            to: remote_service.to_string(),
                            function: remote_method.to_string(),
                            context: cx.clone().unwrap_or(opentelemetry::Context::current()),
                        })
                        .await
                        .map_err(|e| VMError::RemoteCallError(e.to_string()))?;

                    if let Some(cx) = cx {
                        cx.span()
                            .set_attributes(vec![KeyValue::new("response", "OK")]);
                    }
                } else {
                    return Err(VMError::RemoteCallError(
                        "Remote call tx not set".to_string(),
                    ));
                }
            }
            Instruction::StartContext => {
                if let Some(tracer_provider) = self.tracer.as_ref() {
                    let mut metadata = HashMap::new();
                    let tracer = tracer_provider.tracer(self.service_name.clone());
                    let span = tracer
                        .span_builder(format!("{}/{}", self.service_name, "start_context"))
                        .with_kind(SpanKind::Server)
                        .start(&tracer);
                    let cx = Context::current_with_span(span);
                    global::get_text_map_propagator(|propagator| {
                        propagator.inject_context(&cx, &mut metadata)
                    });
                    self.otel_context = Some(cx);
                }
            }
            Instruction::EndContext => match self.otel_context.as_mut() {
                Some(_) => {
                    self.otel_context = None;
                }
                None => {
                    return Err(VMError::MissingSpan);
                }
            },
            Instruction::CheckInterrupt => {
                self.handle_remote_call().await?;
            }
            Instruction::Call(label) => {
                self.handle_local_call(label).await?;
            }
            Instruction::Ret => {
                self.ip = self.return_addresses.pop().unwrap();
            }
        }
        Ok(())
    }

    fn find_current_function_name(&self) -> Option<String> {
        let mut function_name = None;
        for i in (0..self.ip).rev() {
            if matches!(self.code[i], Instruction::Label(_)) {
                match self.code[i].clone() {
                    Instruction::Label(label) => function_name = Some(label),
                    _ => {}
                }
                break;
            }
        }
        function_name
    }
}

#[cfg(test)]
mod tests {
    use crate::{code_gen::CodeGenerator, parser};

    use super::*;

    fn service() -> String {
        "
        service frontend {
            method main_page {
                print \"Main page\";
            }
        }
        "
        .to_string()
    }

    fn service_with_local_call() -> String {
        "
        service frontend {
            method main_page {
                print \"Main page\";
                sleep 1ms;
            }

            loop {
                call main_page;
            }
        }
        "
        .to_string()
    }

    fn service_with_print_template() -> String {
        "
        service frontend {
            method main_page {
                print \"Main page %s\" with [\"12345\", \"67890\"];
            }

            loop {
                call main_page;
            }
        }
        "
        .to_string()
    }

    fn service_with_stderr_template() -> String {
        "
        service frontend {
            method main_page {
                stderr \"Main page %s\" with [\"12345\", \"67890\"];
            }

            loop {
                call main_page;
            }
        }
        "
        .to_string()
    }

    fn service_with_broken_template() -> String {
        "
        service frontend {
            method main_page {
                print \"Main page\" with [\"12345\", \"67890\"];
            }

            loop {
                call main_page;
            }
        }
        "
        .to_string()
    }

    fn call_other_service() -> String {
        "
        service products {
            method get_products {
                print \"Fetching product orders %s\" with [\"12345\", \"67890\"];
                sleep 500ms;
            }
        }

        service frontend {
            method main_page {
                call products.get_products;
            }

            loop {
                call main_page;
            }
        }
        "
        .to_string()
    }

    #[tokio::test]
    async fn test_vm_run() {
        let service = service();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let (print_tx, print_rx) = mpsc::channel(10);
        let mut vm =
            VM::new(code.clone(), &ast.services[0].name, print_tx).with_max_execution_counter(10);
        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have reached max execution counter");
            }
            Err(e) => {
                assert!(print_rx.is_empty(), "Print messages should be empty");
                assert_eq!(e, VMError::MaxExecutionCounterReached);
            }
        }
    }

    #[tokio::test]
    async fn test_vm_with_local_call() {
        let service = service_with_local_call();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let (print_tx, mut print_rx) = mpsc::channel(10);
        let mut vm =
            VM::new(code.clone(), &ast.services[0].name, print_tx).with_max_execution_counter(30);
        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have reached max execution counter");
            }
            Err(e) => {
                assert_eq!(e, VMError::MaxExecutionCounterReached);
                assert_eq!(print_rx.len(), 5);
                for _ in 0..5 {
                    let print_messages = print_rx.recv().await.unwrap();
                    assert_eq!(
                        print_messages,
                        PrintMessage::Stdout("Main page".to_string())
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_vm_with_print_template() {
        let service = service_with_print_template();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let (print_tx, mut print_rx) = mpsc::channel(10);
        let mut vm =
            VM::new(code.clone(), &ast.services[0].name, print_tx).with_max_execution_counter(15);
        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have reached max execution counter");
            }
            Err(e) => {
                assert_eq!(e, VMError::MaxExecutionCounterReached);
                assert_eq!(print_rx.len(), 2);
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(
                    print_messages,
                    PrintMessage::Stdout("Main page 12345".to_string())
                );
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(
                    print_messages,
                    PrintMessage::Stdout("Main page 67890".to_string())
                );
            }
        }
    }

    #[tokio::test]
    async fn test_vm_with_stderr_template() {
        let service = service_with_stderr_template();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let (print_tx, mut print_rx) = mpsc::channel(10);
        let mut vm =
            VM::new(code.clone(), &ast.services[0].name, print_tx).with_max_execution_counter(15);
        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have reached max execution counter");
            }
            Err(e) => {
                assert_eq!(e, VMError::MaxExecutionCounterReached);
                assert_eq!(print_rx.len(), 2);
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(
                    print_messages,
                    PrintMessage::Stderr("Main page 12345".to_string())
                );
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(
                    print_messages,
                    PrintMessage::Stderr("Main page 67890".to_string())
                );
            }
        }
    }

    #[tokio::test]
    async fn test_vm_with_broken_template() {
        let service = service_with_broken_template();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let (print_tx, print_rx) = mpsc::channel(10);
        let mut vm =
            VM::new(code.clone(), &ast.services[0].name, print_tx).with_max_execution_counter(10);
        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have reached max execution counter");
            }
            Err(e) => {
                assert_eq!(e, VMError::InvalidTemplate("Main page".to_string()));
                assert_eq!(print_rx.len(), 0);
            }
        }
    }

    #[tokio::test]
    async fn test_vm_with_remote_call_tx() {
        let service = call_other_service();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[1]).process().unwrap();
        let (print_tx, _print_rx) = mpsc::channel(10);
        let mut vm =
            VM::new(code.clone(), &ast.services[1].name, print_tx).with_max_execution_counter(10);
        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have reached max execution counter");
            }
            Err(e) => {
                assert_eq!(
                    e,
                    VMError::RemoteCallError("Remote call tx not set".to_string())
                );
            }
        }
    }

    #[tokio::test]
    async fn test_vm_with_remote_call() {
        let service = call_other_service();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[1]).process().unwrap();

        let (print_tx, _print_rx) = mpsc::channel(5);
        let (remote_call_tx, mut remote_call_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), &ast.services[1].name, print_tx)
            .with_max_execution_counter(10)
            .with_remote_call_tx(remote_call_tx);

        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have reached max execution counter");
            }
            Err(e) => {
                assert_eq!(e, VMError::MaxExecutionCounterReached);
                assert_eq!(remote_call_rx.len(), 1);
                let remote_call_messages = remote_call_rx.recv().await.unwrap();
                match remote_call_messages {
                    ServiceMessage::Call {
                        to,
                        function,
                        context: _,
                    } => {
                        assert_eq!(to, "products".to_string());
                        assert_eq!(function, "get_products".to_string());
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_vm_with_remote_call_and_receiver() {
        let service = call_other_service();
        let ast = parser::parse(&service).unwrap();
        let code = CodeGenerator::new(&ast.services[0]).process().unwrap();

        let (print_tx, mut print_rx) = mpsc::channel(5);
        let (remote_call_tx, remote_call_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), &ast.services[0].name, print_tx)
            .with_max_execution_counter(15)
            .with_custom_remote_call_limit(1)
            .with_remote_call_rx(remote_call_rx);

        remote_call_tx
            .send("get_products".to_string())
            .await
            .unwrap();

        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have reached max execution counter");
            }
            Err(e) => {
                assert_eq!(e, VMError::MaxExecutionCounterReached);
                assert_eq!(print_rx.len(), 2);
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(
                    print_messages,
                    PrintMessage::Stdout("Fetching product orders 12345".to_string())
                );
            }
        }
    }
}
