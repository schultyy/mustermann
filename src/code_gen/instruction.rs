#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackValue {
    String(String),
    Int(u64),
}

impl std::fmt::Display for StackValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackValue::String(s) => write!(f, "{}", s),
            StackValue::Int(n) => write!(f, "{}", n),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Push(StackValue),
    Pop,
    Dec,
    JmpIfZero(String),
    Label(String),
    Stdout,
    Stderr,
    Sleep(u64),
    StoreVar(String, String),
    LoadVar(String),
    Dup,
    Jump(String),
    Printf,
    RemoteCall,
}
