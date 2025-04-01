use std::collections::HashMap;

use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ServiceMessage {
    Call { to: String, function: String },
}

pub struct ServiceCoordinator {
    services: HashMap<String, mpsc::Sender<String>>,
    main_tx: mpsc::Sender<ServiceMessage>,
    main_rx: mpsc::Receiver<ServiceMessage>,
    remote_call_counter: usize,
}

impl ServiceCoordinator {
    async fn handle_remote_call(&self, msg: ServiceMessage) {
        match msg {
            ServiceMessage::Call { to, function } => {
                if let Some(service_tx) = self.services.get(&to) {
                    service_tx
                        .send(function)
                        .await
                        .unwrap_or_else(|_| println!("Error sending message"));
                } else {
                    tracing::error!("Service not found: {}", to);
                    tracing::error!("Services: {:?}", self.services);
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

    pub fn add_service(&mut self, name: String, tx: mpsc::Sender<String>) {
        self.services.insert(name, tx);
    }
}
