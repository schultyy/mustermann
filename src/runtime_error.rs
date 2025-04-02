use tokio::task::JoinError;

use crate::code_gen::error::ByteCodeError;
use crate::config;
use crate::vm;

#[derive(Debug)]
pub enum RuntimeError {
    VMError(vm::VMError),
    ConfigError(config::ConfigError),
    ByteCodeError(ByteCodeError),
    ServiceError(JoinError),
    InitTraceError(opentelemetry::trace::TraceError),
}

impl std::error::Error for RuntimeError {}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::VMError(e) => write!(f, "VM error: {}", e),
            RuntimeError::ConfigError(e) => write!(f, "Config error: {}", e),
            RuntimeError::ByteCodeError(e) => write!(f, "Byte code error: {}", e),
            RuntimeError::ServiceError(e) => write!(f, "Service error: {}", e),
            RuntimeError::InitTraceError(e) => write!(f, "Init trace error: {}", e),
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

impl From<config::ConfigError> for RuntimeError {
    fn from(e: config::ConfigError) -> Self {
        RuntimeError::ConfigError(e)
    }
}

impl From<ByteCodeError> for RuntimeError {
    fn from(e: ByteCodeError) -> Self {
        RuntimeError::ByteCodeError(e)
    }
}
