use tokio::task::JoinError;

use crate::vm;

#[derive(Debug)]
pub enum RuntimeError {
    VMError(vm::VMError),
    ServiceError(JoinError),
    InitTraceError(opentelemetry_otlp::ExporterBuildError),
    InitMeterError(opentelemetry_otlp::ExporterBuildError),
}

impl std::error::Error for RuntimeError {}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::VMError(e) => write!(f, "VM error: {}", e),
            RuntimeError::ServiceError(e) => write!(f, "Service error: {}", e),
            RuntimeError::InitTraceError(e) => write!(f, "Init trace error: {}", e),
            RuntimeError::InitMeterError(e) => write!(f, "Init meter error: {}", e),
        }
    }
}

impl From<JoinError> for RuntimeError {
    fn from(e: JoinError) -> Self {
        RuntimeError::ServiceError(e)
    }
}

impl From<vm::VMError> for RuntimeError {
    fn from(e: vm::VMError) -> Self {
        RuntimeError::VMError(e)
    }
}
