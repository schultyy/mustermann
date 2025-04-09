use std::collections::HashMap;

use opentelemetry::trace::{Span, SpanKind, Status, Tracer};
use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tokio::sync::mpsc;

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
    trace_provider: Option<SdkTracerProvider>,
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
                if let Some(service) = self.services.get(&to) {
                    let mut span = None;
                    if let Some(trace_provider) = &service.trace_provider {
                        let tracer = trace_provider.tracer(to.clone());
                        span = Some(
                            tracer
                                .span_builder(format!("{}/{}", to.clone(), function))
                                .with_kind(SpanKind::Server)
                                .with_attributes(vec![KeyValue::new(SERVICE_NAME, to.clone())])
                                .start_with_context(&tracer, &context),
                        );
                    }

                    service.sender.send(function).await.unwrap_or_else(|_| {
                        tracing::error!("Error sending message");
                        if let Some(span) = &mut span {
                            span.set_status(Status::error("Error sending message"));
                        }
                    });
                    if let Some(span) = span {
                        drop(span);
                    }
                } else {
                    tracing::error!("Service not found: {}", to);
                }
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
        tracer: Option<SdkTracerProvider>,
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
