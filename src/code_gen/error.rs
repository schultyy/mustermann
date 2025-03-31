#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteCodeError {
    UnsupportedConst(String),
}

impl std::error::Error for ByteCodeError {}

impl std::fmt::Display for ByteCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ByteCodeError::UnsupportedConst(val) => write!(f, "Unsupported constant: {}", val),
        }
    }
}
