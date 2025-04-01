use clap::Parser;
use code_gen::{log_byte_code::LogByteCodeGenerator, service_byte_code::ServiceByteCodeGenerator};
use runtime_error::RuntimeError;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod code_gen;
mod config;
mod otel;
mod runtime_error;
mod vm;

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
    if let Some(otel_endpoint) = args.otel_endpoint {
        otel::setup_otlp_logger(&otel_endpoint, &args.service_name)?;
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
    let config = config::Config::from_file(&args.file_path)?;
    if args.print_code {
        print_code(&config);
    } else {
        let config_clone = config.clone();
        execute_services(config_clone).await?;
        let config_clone = config.clone();
        execute_logs(config_clone).await;
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

async fn execute_services(config: config::Config) -> Result<(), RuntimeError> {
    let mut handles = Vec::new();
    for service in config.services {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move { execute_config_service(service_clone) });
        handles.push(handle);
    }
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                tracing::error!("Error executing service: {}", e);
            }
            Err(e) => {
                tracing::error!("Error executing service: {}", e);
            }
        }
    }
    Ok(())
}

fn execute_config_service(service: config::Service) -> Result<(), RuntimeError> {
    let byte_code = ServiceByteCodeGenerator::new(&service).process_service()?;
    let mut vm = vm::VM::new(byte_code, Box::new(on_stdout), Box::new(on_stderr));
    println!("Running service: {}", service.name);
    vm.run()?;
    Ok(())
}

async fn execute_logs(config: config::Config) {
    let mut handles = Vec::new();
    for task in config.logs {
        let handle = tokio::spawn(async move { execute_config_task(&task) });
        handles.push(handle);
    }
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                tracing::error!("Error executing task: {}", e);
            }
            Err(e) => {
                tracing::error!("Error executing task: {}", e);
            }
        }
    }
}

fn execute_config_task(task: &config::Task) -> Result<(), RuntimeError> {
    let byte_code = LogByteCodeGenerator::new(task).process_task()?;
    let mut vm = vm::VM::new(byte_code, Box::new(on_stdout), Box::new(on_stderr));
    vm.run()?;
    Ok(())
}

fn on_stdout(name: &str, message: &str) -> () {
    tracing::info!(app_name = name, message);
}

fn on_stderr(name: &str, message: &str) -> () {
    tracing::error!(app_name = name, message);
}
