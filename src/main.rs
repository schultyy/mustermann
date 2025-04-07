use std::fs;

use clap::Parser;
use code_gen::{instruction::Instruction, CodeGenerator};
use futures::future::join_all;
use printer::AnnotatedInstruction;
use runtime_error::RuntimeError;
use tokio::sync::mpsc;
use tracing::error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod code_gen;
mod config;
mod metadata_map;
mod otel;
mod parser;
mod printer;
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
    let args = Args::parse();
    let mut logger_provider = None;

    if let Some(otel_endpoint) = args.otel_endpoint.clone() {
        logger_provider = Some(otel::setup_otlp(&otel_endpoint, &args.service_name)?);
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    if args.print_code {
        print_code(&args)?;
    } else {
        execute_code(&args).await?;
    }

    if let Some(logger_provider) = logger_provider {
        logger_provider.shutdown()?;
    }
    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}

fn print_code(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = args.file_path.clone();
    let file_content = fs::read_to_string(&file_path)?;
    let ast = parser::parse(&file_content)?;
    for service in ast.services {
        let codes = CodeGenerator::new(&service).process()?;
        let rows: Vec<AnnotatedInstruction> = codes.iter().map(|i| i.into()).collect::<Vec<_>>();
        let mut table = tabled::Table::new(rows);
        println!("{}", table.with(tabled::settings::Style::sharp()));
    }
    Ok(())
}

async fn execute_code(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = args.file_path.clone();
    let file_content = fs::read_to_string(&file_path)?;
    let ast = parser::parse(&file_content)?;
    let mut handles: Vec<tokio::task::JoinHandle<Result<(), vm::VMError>>> = Vec::new();
    let mut coordinator = vm_coordinator::ServiceCoordinator::new();
    for service in ast.services {
        let service_code = CodeGenerator::new(&service).process()?;
        let service_handles = execute_service(
            &service.name,
            service_code,
            &mut coordinator,
            &args
                .otel_endpoint
                .clone()
                .unwrap_or("http://localhost:4317".to_string()),
        )
        .await?;
        handles.extend(service_handles);
    }
    let coordinator_handle = tokio::spawn(async move {
        coordinator.run().await;
        Ok(())
    });
    handles.push(coordinator_handle);
    join_all(handles).await;
    Ok(())
}

async fn execute_service(
    service_name: &str,
    service_code: Vec<Instruction>,
    coordinator: &mut vm_coordinator::ServiceCoordinator,
    otel_endpoint: &str,
) -> Result<Vec<tokio::task::JoinHandle<Result<(), vm::VMError>>>, RuntimeError> {
    let (print_tx, mut print_rx) = mpsc::channel(1);
    let (remote_call_tx, mut remote_call_rx) = mpsc::channel(1);

    let tracer = vm::setup_tracer(&otel_endpoint, &service_name)
        .map_err(|e| RuntimeError::InitTraceError(e))?;

    let mut vm = vm::VM::new(service_code.clone(), &service_name, print_tx)
        .with_remote_call_tx(coordinator.get_main_tx().clone())
        .with_remote_call_rx(remote_call_rx)
        .with_tracer(tracer.clone());
    coordinator.add_service(
        service_name.to_string(),
        remote_call_tx.clone(),
        Some(tracer),
    );
    let mut handles = Vec::new();
    let app_name = service_name.to_string();
    let print_handle = tokio::spawn(async move {
        while let Some(message) = print_rx.recv().await {
            match message {
                vm::PrintMessage::Stdout(message) => {
                    tracing::info!(app_name = %app_name, "{}", message);
                }
                vm::PrintMessage::Stderr(message) => {
                    tracing::error!(app_name = %app_name, "{}", message);
                }
            }
        }
        Ok(())
    });
    handles.push(print_handle);
    handles.push(tokio::spawn(async move {
        match vm.run().await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Error: {}", e);
                Err(e)
            }
        }
    }));
    Ok(handles)
}
