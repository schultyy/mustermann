use std::collections::HashMap;

use opentelemetry::metrics::Counter;
use opentelemetry::metrics::Gauge;
use opentelemetry::metrics::MeterProvider;
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::trace::{TraceContextExt, TracerProvider};
use opentelemetry::{global, KeyValue};
use opentelemetry::{
    trace::{SpanKind, Tracer},
    Context,
};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::metrics::Temporality;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tokio::sync::mpsc;
use tonic::metadata::{MetadataMap, MetadataValue};

use crate::code_gen::instruction::{
    Instruction, StackValue, CALL_CODE, CHECK_INTERRUPT_CODE, DEC_CODE, DUP_CODE, END_CONTEXT_CODE,
    JMP_IF_ZERO_CODE, JUMP_CODE, LABEL_CODE, LOAD_VAR_CODE, POP_CODE, PRINTF_CODE, PUSH_INT_CODE,
    PUSH_STRING_CODE, REMOTE_CALL_CODE, RET_CODE, SLEEP_CODE, START_CONTEXT_CODE, STDERR_CODE,
    STDOUT_CODE, STORE_VAR_CODE,
};
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
    InvalidInstruction(u8),
    MissingStackFrame,
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
            VMError::InvalidInstruction(instruction) => {
                write!(f, "Invalid instruction: {}", instruction)
            }
            VMError::MissingStackFrame => write!(f, "Missing stack frame"),
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

pub(crate) fn init_meter_provider(
    endpoint: Option<&str>,
    service_name: &str,
) -> Result<opentelemetry_sdk::metrics::SdkMeterProvider, opentelemetry_otlp::ExporterBuildError> {
    let provider = if let Some(endpoint) = endpoint {
        let exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_temporality(Temporality::Delta)
            .with_tonic()
            .with_endpoint(endpoint.to_string())
            .build()?;
        let resource = Resource::builder()
            .with_service_name(service_name.to_string())
            .build();

        SdkMeterProvider::builder()
            .with_periodic_exporter(exporter)
            .with_resource(resource)
            .build()
    } else {
        let exporter = opentelemetry_stdout::MetricExporter::default();

        let resource = Resource::builder()
            .with_service_name(service_name.to_string())
            .build();

        SdkMeterProvider::builder()
            .with_periodic_exporter(exporter)
            .with_resource(resource)
            .build()
    };

    Ok(provider)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintMessage {
    Stdout(String),
    Stderr(String),
}

///The length of the length byte array for a string
const LENGTH_OFFSET: usize = std::mem::size_of::<usize>();

pub struct VM {
    code: Vec<u8>,
    stack: Vec<Vec<StackValue>>,
    vars: HashMap<String, StackValue>,
    label_jump_map: HashMap<String, usize>,
    label_index_map: HashMap<usize, String>,
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
    meter_provider: SdkMeterProvider,
    otel_context: Option<opentelemetry::Context>,
}

///Generate the bytecode for a given set of instructions
/// Returns the bytecode and a map of label to jump position
/// This is used to optimize the code by precomputing the jump positions
fn generate_bytecode(
    instructions: Vec<Instruction>,
) -> (Vec<u8>, HashMap<String, usize>, HashMap<usize, String>) {
    let mut bytes = vec![];
    let mut label_jump_map = HashMap::new();
    let mut label_index_map = HashMap::new();
    for instruction in instructions {
        let instruction_bytes = instruction.to_bytes();
        bytes.extend(instruction_bytes);

        if let Instruction::Label(label) = instruction {
            //Store the position of the label + the length of the instruction + the length of the label
            label_jump_map.insert(label.clone(), bytes.len());
            label_index_map.insert(bytes.len(), label);
        }
    }
    (bytes, label_jump_map, label_index_map)
}

impl VM {
    pub fn new(
        code: Vec<Instruction>,
        service_name: &str,
        print_tx: mpsc::Sender<PrintMessage>,
    ) -> Self {
        let service_name = service_name.to_string();
        let (code, label_jump_map, label_index_map) = generate_bytecode(code);

        Self {
            code,
            label_jump_map,
            label_index_map,
            stack: vec![Vec::new()],
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
            meter_provider: init_meter_provider(None, &service_name).unwrap(),
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

    pub fn with_meter_provider(mut self, meter_provider: SdkMeterProvider) -> Self {
        self.meter_provider = meter_provider;
        self
    }

    fn build_counters(
        &self,
    ) -> Result<(Counter<u64>, Counter<u64>, Gauge<u64>, Gauge<u64>), VMError> {
        let remote_invocation_counter = self
            .meter_provider
            .meter("remote_invocation_counter")
            .u64_counter("remote_invocation_counter")
            .build()
            .to_owned();

        let local_invocation_counter = self
            .meter_provider
            .meter("local_invocation_counter")
            .u64_counter("local_invocation_counter")
            .build()
            .to_owned();

        let instruction_duration = self
            .meter_provider
            .meter("instruction_duration")
            .u64_gauge("instruction_duration")
            .with_unit("ms")
            .with_description("The duration of executing an instruction in milliseconds")
            .build()
            .to_owned();

        let remote_call_duration = self
            .meter_provider
            .meter("remote_call_duration")
            .u64_gauge("remote_call_duration")
            .with_unit("ms")
            .with_description("The duration of a remote call in milliseconds")
            .build()
            .to_owned();

        Ok((
            remote_invocation_counter,
            local_invocation_counter,
            instruction_duration,
            remote_call_duration,
        ))
    }

    pub async fn run(&mut self) -> Result<(), VMError> {
        let mut execution_counter = 0;
        let counters = self.build_counters()?;

        while self.ip < self.code.len() {
            if self.ip >= self.code.len() {
                return Err(VMError::IPOutOfBounds(self.ip, self.code.len()));
            }
            self.execute_instruction(counters.clone()).await?;
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
        self.stack.push(Vec::new());
        self.ip = *self
            .label_jump_map
            .get(&label)
            .ok_or(VMError::MissingLabel(label.clone()))?;
        Ok(())
    }

    #[inline]
    fn extract_length(&self) -> (usize, usize, usize) {
        let start = self.ip + 1;
        let end = start + LENGTH_OFFSET;
        let length_bytes: [u8; LENGTH_OFFSET] = self.code[start..end].try_into().unwrap();
        let length = usize::from_le_bytes(length_bytes.try_into().unwrap());
        (start, end, length)
    }

    fn current_stackframe(&mut self) -> Result<&mut Vec<StackValue>, VMError> {
        self.stack.last_mut().ok_or(VMError::MissingStackFrame)
    }

    async fn execute_instruction(
        &mut self,
        counters: (Counter<u64>, Counter<u64>, Gauge<u64>, Gauge<u64>),
    ) -> Result<(), VMError> {
        let instruction = self.code[self.ip];
        let (
            remote_invocation_counter,
            local_invocation_counter,
            instruction_duration,
            remote_call_duration,
        ) = counters;
        let start = std::time::Instant::now();
        match instruction {
            PUSH_STRING_CODE => {
                let (_start, end, str_len) = self.extract_length();
                let str = &self.code[end..end + str_len];
                let str = String::from_utf8(str.to_vec()).unwrap();
                self.current_stackframe()?.push(StackValue::String(str));
                self.ip = end + str_len;
            }
            PUSH_INT_CODE => {
                let (_start, end, int_len) = self.extract_length();
                let int = &self.code[end..end + int_len];
                let int = u64::from_le_bytes(int.try_into().unwrap());
                self.current_stackframe()?.push(StackValue::Int(int));
                self.ip = end + int_len;
            }
            POP_CODE => {
                self.stack.pop();
                self.ip += 1;
            }
            DEC_CODE => {
                let top = self
                    .current_stackframe()?
                    .pop()
                    .ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::Int(n) => self.current_stackframe()?.push(StackValue::Int(n - 1)),
                    _ => return Err(VMError::InvalidStackValue),
                }
                self.ip += 1;
            }
            JMP_IF_ZERO_CODE => {
                let (_start, end, jump_to_label_len) = self.extract_length();
                let jump_to_label_bytes = &self.code[end..end + jump_to_label_len];
                let jump_to_label = String::from_utf8(jump_to_label_bytes.to_vec()).unwrap();
                let top = self
                    .current_stackframe()?
                    .pop()
                    .ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::Int(n) => {
                        if n == 0 {
                            self.ip = self
                                .label_jump_map
                                .get(&jump_to_label)
                                .ok_or(VMError::MissingLabel(jump_to_label.clone()))?
                                .to_owned();
                        }
                    }
                    _ => return Err(VMError::InvalidStackValue),
                }
                self.ip += 1;
            }
            LABEL_CODE => {
                let (_start, end, label_len) = self.extract_length();
                self.ip = end + label_len;
            }
            STDOUT_CODE => {
                let str = self
                    .current_stackframe()?
                    .pop()
                    .ok_or(VMError::StackUnderflow)?;
                match str {
                    StackValue::String(s) => self
                        .print_tx
                        .send(PrintMessage::Stdout(s))
                        .await
                        .map_err(VMError::PrintError)?,
                    StackValue::Int(i) => self
                        .print_tx
                        .send(PrintMessage::Stdout(i.to_string()))
                        .await
                        .map_err(VMError::PrintError)?,
                }
                self.ip += 1;
            }
            STDERR_CODE => {
                let top = self
                    .current_stackframe()?
                    .pop()
                    .ok_or(VMError::StackUnderflow)?;
                match top {
                    StackValue::String(s) => {
                        self.print_tx
                            .send(PrintMessage::Stderr(s))
                            .await
                            .map_err(VMError::PrintError)?;
                    }
                    _ => return Err(VMError::InvalidStackValue),
                }
                self.ip += 1;
            }
            SLEEP_CODE => {
                let (_start, end, sleep_len) = self.extract_length();
                let sleep_bytes = &self.code[end..end + sleep_len];
                let sleep_ms = u64::from_le_bytes(sleep_bytes.try_into().unwrap());
                std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
                self.ip = end + sleep_len;
            }
            STORE_VAR_CODE => {
                let (_start, end, key_len) = self.extract_length();
                let key = &self.code[end..end + key_len];
                let key = String::from_utf8(key.to_vec()).unwrap();

                //We need to substract one here because extract_length adds +1 to compensate for the instruction byte
                self.ip = end + key_len - 1;

                let (_start, end, value_len) = self.extract_length();
                let value = &self.code[end..end + value_len];
                let value = String::from_utf8(value.to_vec()).unwrap();

                self.vars.insert(key, StackValue::String(value));
                self.ip = end + value_len;
            }
            LOAD_VAR_CODE => {
                let (_start, end, key_len) = self.extract_length();
                let key = &self.code[end..end + key_len];
                let key = String::from_utf8(key.to_vec()).unwrap();
                let value = self
                    .vars
                    .get(&key)
                    .ok_or(VMError::MissingVar(key.clone()))?
                    .clone();
                self.current_stackframe()?.push(value);
                self.ip = end + key_len;
            }
            DUP_CODE => {
                let top = self
                    .current_stackframe()?
                    .last()
                    .ok_or(VMError::StackUnderflow)?
                    .clone();
                self.current_stackframe()?.push(top);
                self.ip += 1;
            }
            JUMP_CODE => {
                let (_start, end, jump_to_label_len) = self.extract_length();
                let jump_to_label_bytes = &self.code[end..end + jump_to_label_len];
                let jump_to_label = String::from_utf8(jump_to_label_bytes.to_vec()).unwrap();
                self.ip = self
                    .label_jump_map
                    .get(&jump_to_label)
                    .ok_or(VMError::MissingLabel(jump_to_label.clone()))?
                    .to_owned();
            }
            PRINTF_CODE => {
                let var = self
                    .current_stackframe()?
                    .pop()
                    .ok_or(VMError::StackUnderflow)?;
                let template = self
                    .current_stackframe()?
                    .pop()
                    .ok_or(VMError::StackUnderflow)?;
                let template = match template {
                    StackValue::String(s) => s,
                    _ => return Err(VMError::InvalidStackValue),
                };

                if template.contains("%s") {
                    let var = match var {
                        StackValue::String(s) => s,
                        _ => return Err(VMError::InvalidStackValue),
                    };
                    let formatted = template.replace("%s", &var);
                    self.current_stackframe()?
                        .push(StackValue::String(formatted));
                } else if template.contains("%d") {
                    let var = match var {
                        StackValue::Int(i) => i,
                        _ => return Err(VMError::InvalidStackValue),
                    };
                    let formatted = template.replace("%d", &var.to_string());
                    self.current_stackframe()?
                        .push(StackValue::String(formatted));
                } else {
                    return Err(VMError::InvalidTemplate(template.clone()));
                }
                self.ip += 1;
            }
            REMOTE_CALL_CODE => {
                let start = std::time::Instant::now();
                let remote_call_tx = self
                    .remote_call_tx
                    .as_ref()
                    .ok_or(VMError::RemoteCallError(
                        "Remote call tx not set".to_string(),
                    ))?
                    .clone();

                let remote_method = self
                    .current_stackframe()?
                    .pop()
                    .ok_or(VMError::StackUnderflow)?
                    .clone();
                let remote_service = self
                    .current_stackframe()?
                    .pop()
                    .ok_or(VMError::StackUnderflow)?
                    .clone();
                let local_function_name = self
                    .find_current_function_name()
                    .ok_or(VMError::MissingFunctionName)?;
                let mut cx = None;

                if let Some(tracer_provider) = self.tracer.as_ref() {
                    if let Some(otel_cx) = self.otel_context.as_ref() {
                        let tracer = tracer_provider.tracer(self.service_name.clone());

                        let span = tracer
                            .span_builder(format!("{}/{}", self.service_name, local_function_name))
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

                remote_invocation_counter.add(
                    1,
                    &[
                        KeyValue::new("service", self.service_name.clone()),
                        KeyValue::new("method", remote_method.to_string().clone()),
                    ],
                );

                let duration = start.elapsed();
                let duration_ms = duration.as_millis() as u64;
                remote_call_duration.record(
                    duration_ms,
                    &[
                        KeyValue::new("service", self.service_name.clone()),
                        KeyValue::new("method", remote_method.to_string().clone()),
                    ],
                );
                if let Some(cx) = cx {
                    cx.span()
                        .set_attributes(vec![KeyValue::new("response", "OK")]);
                }
                self.ip += 1;
            }
            START_CONTEXT_CODE => {
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
                self.ip += 1;
            }
            END_CONTEXT_CODE => {
                match self.otel_context.as_mut() {
                    Some(_) => {
                        self.otel_context = None;
                    }
                    None => {
                        return Err(VMError::MissingSpan);
                    }
                }
                self.ip += 1;
            }
            CHECK_INTERRUPT_CODE => {
                self.handle_remote_call().await?;
            }
            CALL_CODE => {
                let (_start, end, label_len) = self.extract_length();
                let label = &self.code[end..end + label_len];
                let label = String::from_utf8(label.to_vec()).unwrap();
                self.handle_local_call(label.clone()).await?;
                local_invocation_counter
                    .add(1, &[KeyValue::new("method", label.to_string().clone())]);
            }
            RET_CODE => {
                self.ip = self.return_addresses.pop().unwrap();
                self.stack.pop();
            }
            _ => {
                return Err(VMError::InvalidInstruction(instruction));
            }
        }
        let duration = start.elapsed();
        let duration_ms = duration.as_millis() as u64;
        instruction_duration.record(
            duration_ms,
            &[KeyValue::new(
                "instruction",
                crate::code_gen::instruction::code_to_name(instruction),
            )],
        );
        Ok(())
    }

    fn find_current_function_name(&self) -> Option<String> {
        for i in (0..self.ip).rev() {
            if self.label_index_map.contains_key(&i) {
                return Some(self.label_index_map.get(&i).unwrap().clone());
            }
        }

        None
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
    async fn test_push_string() {
        let code = vec![
            Instruction::Push(StackValue::String("Hello, world!".to_string())),
            Instruction::Stdout,
        ];
        let (print_tx, mut print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(2);
        match vm.run().await {
            Ok(_) => {
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(
                    print_messages,
                    PrintMessage::Stdout("Hello, world!".to_string())
                );
            }
            Err(_e) => {
                assert!(false, "VM should have finished execution");
            }
        }
    }

    #[tokio::test]
    async fn test_push_int() {
        let code = vec![
            Instruction::Push(StackValue::Int(12345)),
            Instruction::Stdout,
        ];
        let (print_tx, mut print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(2);
        match vm.run().await {
            Ok(_) => {
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(print_messages, PrintMessage::Stdout("12345".to_string()));
            }
            Err(e) => {
                eprintln!("VM should have finished execution: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_jmp_if_zero() {
        let code = vec![
            Instruction::Push(StackValue::String("Unexpected Code Reached".to_string())),
            Instruction::Push(StackValue::Int(0)),
            Instruction::JmpIfZero("label".to_string()),
            Instruction::Stdout, //We're trying to skip this
            Instruction::Label("label".to_string()),
        ];
        let (print_tx, print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(4);
        match vm.run().await {
            Ok(_) => {
                assert_eq!(print_rx.len(), 0); //We should have skipped the stdout
            }
            Err(e) => {
                eprintln!("VM should have finished execution: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_sleep() {
        let sleep_duration = 100;
        let code = vec![Instruction::Sleep(sleep_duration)];
        let (print_tx, print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(1);
        let start = std::time::Instant::now();
        match vm.run().await {
            Ok(_) => {
                let elapsed = start.elapsed();
                assert_eq!(print_rx.len(), 0); //We should have skipped the stdout
                assert!(elapsed.as_millis() >= sleep_duration as u128);
                assert!(elapsed.as_millis() <= (sleep_duration + 100) as u128);
            }
            Err(e) => {
                eprintln!("VM should have finished execution: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_store_var() {
        let code = vec![
            Instruction::StoreVar("test".to_string(), "test".to_string()),
            Instruction::LoadVar("test".to_string()),
            Instruction::Stdout,
        ];
        let (print_tx, mut print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(3);
        match vm.run().await {
            Ok(_) => {
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(print_messages, PrintMessage::Stdout("test".to_string()));
            }
            Err(e) => {
                eprintln!("VM should have finished execution: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_dup() {
        let code = vec![
            Instruction::Push(StackValue::String("Hello, world!".to_string())),
            Instruction::Dup,
            Instruction::Stdout,
            Instruction::Stdout,
        ];
        let (print_tx, print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(5);
        match vm.run().await {
            Ok(_) => {
                assert_eq!(print_rx.len(), 2);
            }
            Err(e) => {
                eprintln!("VM should have finished execution: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_jump() {
        let code = vec![
            Instruction::Push(StackValue::String("Hello, world!".to_string())),
            Instruction::Jump("label".to_string()),
            Instruction::Stdout, //We're trying to skip this
            Instruction::Label("label".to_string()),
        ];
        let (print_tx, print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(3);
        match vm.run().await {
            Ok(_) => {
                assert_eq!(print_rx.len(), 0);
            }
            Err(e) => {
                eprintln!("VM should have finished execution: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_printf() {
        let code = vec![
            Instruction::Push(StackValue::String("Hello, %s!".to_string())),
            Instruction::Push(StackValue::String("world".to_string())),
            Instruction::Printf,
            Instruction::Stdout,
        ];
        let (print_tx, mut print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(4);
        match vm.run().await {
            Ok(_) => {
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(
                    print_messages,
                    PrintMessage::Stdout("Hello, world!".to_string())
                );
            }
            Err(e) => {
                eprintln!("VM should have finished execution: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_printf_with_int() {
        let code = vec![
            Instruction::Push(StackValue::String("Hello, %d!".to_string())),
            Instruction::Push(StackValue::Int(12345)),
            Instruction::Printf,
            Instruction::Stdout,
        ];
        let (print_tx, mut print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(4);
        match vm.run().await {
            Ok(_) => {
                let print_messages = print_rx.recv().await.unwrap();
                assert_eq!(
                    print_messages,
                    PrintMessage::Stdout("Hello, 12345!".to_string())
                );
            }
            Err(e) => {
                eprintln!("VM should have finished execution: {:?}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn test_printf_with_invalid_template() {
        let code = vec![
            Instruction::Push(StackValue::String("Hello, %!".to_string())),
            Instruction::Push(StackValue::Int(12345)),
            Instruction::Printf,
            Instruction::Stdout,
        ];
        let (print_tx, _print_rx) = mpsc::channel(10);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(4);
        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have reached max execution counter");
            }
            Err(e) => {
                assert_eq!(e, VMError::InvalidTemplate("Hello, %!".to_string()));
            }
        }
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

    #[tokio::test]
    async fn test_vm_creates_new_stackframe_on_call() {
        let code = vec![
            Instruction::Jump("main".to_string()),
            Instruction::Label("start_function".to_string()),
            Instruction::Stdout,
            Instruction::Ret,
            Instruction::Label("end_function".to_string()),
            Instruction::Label("main".to_string()),
            Instruction::Push(StackValue::String("world".to_string())),
            Instruction::Call("start_function".to_string()),
            Instruction::Stdout,
        ];

        let (print_tx, print_rx) = mpsc::channel(5);
        let mut vm = VM::new(code.clone(), "test", print_tx).with_max_execution_counter(15);

        match vm.run().await {
            Ok(_) => {
                assert!(false, "VM should have failed because of missing stackframe");
            }
            Err(e) => {
                assert_eq!(e, VMError::StackUnderflow);
                assert_eq!(print_rx.len(), 0);
            }
        }
    }
}
