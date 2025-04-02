use std::collections::HashMap;

use opentelemetry::trace::{FutureExt, Span, SpanKind, Status, Tracer};
use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_sdk::trace;
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tokio::sync::mpsc;

// Create a HashMap adapter that implements opentelemetry's Extractor
struct MetadataExtractor<'a>(pub &'a HashMap<String, String>);

impl<'a> opentelemetry::propagation::Extractor for MetadataExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|v| v.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

#[derive(Debug, Clone)]
pub enum ServiceMessage {
    Call {
        to: String,
        function: String,
        context: opentelemetry::Context,
    },
}

struct Service {
    sender: mpsc::Sender<String>,
    trace_provider: trace::TracerProvider,
}

pub struct ServiceCoordinator {
    services: HashMap<String, Service>,
    main_tx: mpsc::Sender<ServiceMessage>,
    main_rx: mpsc::Receiver<ServiceMessage>,
    remote_call_counter: usize,
}

impl ServiceCoordinator {
    async fn handle_remote_call(&self, msg: ServiceMessage) {
        match msg {
            ServiceMessage::Call {
                to,
                function,
                context,
            } => {
                let tracer = global::tracer(to.clone());
                let mut span = tracer
                    .span_builder(format!("{}/{}", to.clone(), function))
                    .with_kind(SpanKind::Server)
                    .start_with_context(&tracer, &context);

                span.set_attribute(KeyValue::new(SERVICE_NAME, to.clone()));

                if let Some(service) = self.services.get(&to) {
                    let tracer = service.trace_provider.tracer(to.clone());
                    let mut span = tracer
                        .span_builder(format!("{}/{}", to.clone(), function))
                        .with_kind(SpanKind::Server)
                        .start_with_context(&tracer, &context);
                    span.set_attribute(KeyValue::new(SERVICE_NAME, to.clone()));

                    service
                        .sender
                        .send(function)
                        .await
                        .unwrap_or_else(|_| println!("Error sending message"));
                } else {
                    tracing::error!("Service not found: {}", to);
                    span.set_status(Status::error("Service not found"));
                }
                span.end();
            }
        }
    }
    pub async fn run(&mut self) {
        loop {
            self.remote_call_counter += 1;
            if self.remote_call_counter > 10000 {
                match self.main_rx.try_recv() {
                    Ok(msg) => {
                        self.handle_remote_call(msg).await;
                    }
                    Err(e) => {
                        tracing::debug!("Error: {}", e);
                    }
                }
                self.remote_call_counter = 0;
            }
        }
    }

    pub fn new() -> Self {
        let (main_tx, main_rx) = mpsc::channel(100);
        Self {
            services: HashMap::new(),
            main_tx,
            main_rx,
            remote_call_counter: 0,
        }
    }

    pub fn get_main_tx(&self) -> mpsc::Sender<ServiceMessage> {
        self.main_tx.clone()
    }

    pub fn add_service(
        &mut self,
        name: String,
        tx: mpsc::Sender<String>,
        tracer: trace::TracerProvider,
    ) {
        self.services.insert(
            name,
            Service {
                sender: tx,
                trace_provider: tracer,
            },
        );
    }
}
