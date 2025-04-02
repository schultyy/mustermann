use ::futures::future::join_all;
use clap::Parser;
use code_gen::{log_byte_code::LogByteCodeGenerator, service_byte_code::ServiceByteCodeGenerator};
use runtime_error::RuntimeError;
use tokio::{sync::mpsc, task::JoinHandle};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use vm::VMError;

mod code_gen;
mod config;
mod metadata_map;
mod otel;
mod runtime_error;
mod vm;
mod vm_coordinator;

/// CLI tool for pattern matching
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug mode
    #[arg(short, long)]
    print_code: bool,
    /// The path to the config file
    file_path: String,
    otel_endpoint: Option<String>,
    /// The name of the service to be used in the logs. Defaults to "mustermann"
    #[arg(short, long, default_value = "mustermann")]
    service_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();
    if let Some(otel_endpoint) = args.otel_endpoint.clone() {
        println!("Setting up otel: {}", otel_endpoint);
        otel::setup_otlp(&otel_endpoint, &args.service_name)?;
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
    let file_path = args.file_path.clone();
    let config = config::Config::from_file(&file_path)?;
    if args.print_code {
        print_code(&config);
    } else {
        let config_clone = config.clone();
        execute_services(&args, config_clone).await?;
        let config_clone = config.clone();
        execute_logs(config_clone).await?;
    }
    Ok(())
}

fn print_code(config: &config::Config) {
    for log in &config.logs {
        let code = LogByteCodeGenerator::new(log).process_task().unwrap();
        println!(
            "{}",
            code.iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        );
    }
    for service in &config.services {
        let code = ServiceByteCodeGenerator::new(service)
            .process_service()
            .unwrap();
        println!("Service: {}", service.name);
        println!("--------------------------------");
        println!(
            "{}",
            code.iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        );
        println!("--------------------------------");
    }
}

async fn execute_services(args: &Args, config: config::Config) -> Result<(), RuntimeError> {
    let mut coordinator = vm_coordinator::ServiceCoordinator::new();
    let mut handles: Vec<JoinHandle<Result<(), VMError>>> = Vec::new();
    let otel_endpoint = args
        .otel_endpoint
        .clone()
        .unwrap_or("http://localhost:4317".to_string());
    for service in config.services {
        let (tx, rx) = mpsc::channel(1000);
        let tracer = vm::setup_tracer(&otel_endpoint, &service.name)
            .map_err(|e| RuntimeError::InitTraceError(e))?;

        coordinator.add_service(service.name.clone(), tx.clone(), tracer.clone());
        let coordinator_tx = coordinator.get_main_tx();
        let byte_code = ServiceByteCodeGenerator::new(&service).process_service()?;
        let mut vm = vm::VM::with_tracer(byte_code, Some(coordinator_tx), Some(rx), Some(tracer))
            .map_err(|e| RuntimeError::InitTraceError(e))?;
        handles.push(tokio::spawn(async move {
            return vm.run().await;
        }));
    }

    let coordinator_handle: JoinHandle<Result<(), VMError>> = tokio::spawn(async move {
        coordinator.run().await;
        Ok(())
    });
    handles.push(coordinator_handle);
    join_all(handles).await;
    Ok(())
}

async fn execute_logs(config: config::Config) -> Result<(), RuntimeError> {
    for task in config.logs {
        execute_config_task(&task).await?;
    }
    Ok(())
}

async fn execute_config_task(task: &config::Task) -> Result<(), RuntimeError> {
    let byte_code = LogByteCodeGenerator::new(task).process_task()?;
    let mut vm = vm::VM::with_tracer(byte_code, None, None, None)
        .map_err(|e| RuntimeError::InitTraceError(e))?;
    vm.run().await?;
    Ok(())
}
