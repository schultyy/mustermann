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
    /// Push a value onto the stack
    Push(StackValue),
    /// Pop a value from the stack
    Pop,
    /// Decrement the value on the top of the stack
    Dec,
    /// Jump to a label if the value on the top of the stack is zero
    /// Will not pop the value from the stack
    JmpIfZero(String),
    /// Label for a jump target
    Label(String),
    /// Print to stdout
    Stdout,
    /// Print to stderr
    Stderr,
    /// Sleep for a given number of milliseconds
    Sleep(u64),
    /// Store a variable
    StoreVar(String, String),
    /// Load a variable
    LoadVar(String),
    /// Duplicate the value on the top of the stack
    Dup,
    /// Jump to a label
    Jump(String),
    /// Takes the top value of the stack and prints it as a formatted string
    /// example:
    /// ```
    /// "Hello, %s!"
    /// ```
    /// will print "Hello, John!" if the name variable is "John"
    Printf,
    /// Remote call, expected stack layout:
    /// ```
    /// [service_name, method_name]
    /// ```
    RemoteCall,
    /// Start a new OpenTelemetry context
    StartContext,
    /// End a OpenTelemetry context
    EndContext,
    /// No operation
    Nop,
    /// Calls a local function, indicated by a label
    Call(String),
    /// Return from a local function
    Ret,
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
            Instruction::StartContext => write!(f, "StartContext"),
            Instruction::EndContext => write!(f, "EndContext"),
            Instruction::Nop => write!(f, "Nop"),
            Instruction::Call(label) => write!(f, "Call({})", label),
            Instruction::Ret => write!(f, "Ret"),
        }
    }
}
