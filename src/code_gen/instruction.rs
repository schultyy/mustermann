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

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Push(value) => write!(f, "Push({})", value),
            Instruction::Pop => write!(f, "Pop"),
            Instruction::Dec => write!(f, "Dec"),
            Instruction::JmpIfZero(label) => write!(f, "JmpIfZero({})", label),
            Instruction::Label(label) => write!(f, "Label({})", label),
            Instruction::Stdout => write!(f, "Stdout"),
            Instruction::Stderr => write!(f, "Stderr"),
            Instruction::Sleep(ms) => write!(f, "Sleep({})", ms),
            Instruction::StoreVar(key, value) => write!(f, "StoreVar({} = {})", key, value),
            Instruction::LoadVar(key) => write!(f, "LoadVar({})", key),
            Instruction::Dup => write!(f, "Dup"),
            Instruction::Jump(label) => write!(f, "Jump({})", label),
            Instruction::Printf => write!(f, "Printf"),
            Instruction::RemoteCall => write!(f, "RemoteCall"),
        }
    }
}
