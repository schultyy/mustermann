#[derive(Debug, Clone)]
pub enum CodeGenError {
    InvalidStatement(String),
}

impl std::fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodeGenError::InvalidStatement(msg) => write!(f, "Invalid statement: {}", msg),
        }
    }
}

impl std::error::Error for CodeGenError {}
