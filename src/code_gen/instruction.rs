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
    CheckInterrupt,
    /// Calls a local function, indicated by a label
    Call(String),
    /// Return from a local function
    Ret,
}

pub const PUSH_STRING_CODE: u8 = 0x01;
pub const PUSH_INT_CODE: u8 = 0x02;
pub const POP_CODE: u8 = 0x03;
pub const DEC_CODE: u8 = 0x04;
pub const JMP_IF_ZERO_CODE: u8 = 0x05;
pub const LABEL_CODE: u8 = 0x06;
pub const STDOUT_CODE: u8 = 0x07;
pub const STDERR_CODE: u8 = 0x08;
pub const SLEEP_CODE: u8 = 0x09;
pub const STORE_VAR_CODE: u8 = 0x0a;
pub const LOAD_VAR_CODE: u8 = 0x0b;
pub const DUP_CODE: u8 = 0x0c;
pub const JUMP_CODE: u8 = 0x0d;
pub const PRINTF_CODE: u8 = 0x0e;
pub const REMOTE_CALL_CODE: u8 = 0x0f;
pub const START_CONTEXT_CODE: u8 = 0x10;
pub const END_CONTEXT_CODE: u8 = 0x11;
pub const CHECK_INTERRUPT_CODE: u8 = 0x12;
pub const CALL_CODE: u8 = 0x13;
pub const RET_CODE: u8 = 0x14;

pub fn code_to_name(code: u8) -> String {
    match code {
        PUSH_STRING_CODE => "PushString".to_string(),
        PUSH_INT_CODE => "PushInt".to_string(),
        POP_CODE => "Pop".to_string(),
        DEC_CODE => "Dec".to_string(),
        JMP_IF_ZERO_CODE => "JmpIfZero".to_string(),
        LABEL_CODE => "Label".to_string(),
        STDOUT_CODE => "Stdout".to_string(),
        STDERR_CODE => "Stderr".to_string(),
        SLEEP_CODE => "Sleep".to_string(),
        STORE_VAR_CODE => "StoreVar".to_string(),
        LOAD_VAR_CODE => "LoadVar".to_string(),
        DUP_CODE => "Dup".to_string(),
        JUMP_CODE => "Jump".to_string(),
        PRINTF_CODE => "Printf".to_string(),
        REMOTE_CALL_CODE => "RemoteCall".to_string(),
        START_CONTEXT_CODE => "StartContext".to_string(),
        END_CONTEXT_CODE => "EndContext".to_string(),
        CHECK_INTERRUPT_CODE => "CheckInterrupt".to_string(),
        CALL_CODE => "Call".to_string(),
        RET_CODE => "Ret".to_string(),
        _ => "Unknown".to_string(),
    }
}

impl Instruction {
    pub fn code(&self) -> u8 {
        match self {
            Instruction::Push(StackValue::String(_)) => PUSH_STRING_CODE,
            Instruction::Push(StackValue::Int(_)) => PUSH_INT_CODE,
            Instruction::Pop => POP_CODE,
            Instruction::Dec => DEC_CODE,
            Instruction::JmpIfZero(_) => JMP_IF_ZERO_CODE,
            Instruction::Label(_) => LABEL_CODE,
            Instruction::Stdout => STDOUT_CODE,
            Instruction::Stderr => STDERR_CODE,
            Instruction::Sleep(_) => SLEEP_CODE,
            Instruction::StoreVar(_, _) => STORE_VAR_CODE,
            Instruction::LoadVar(_) => LOAD_VAR_CODE,
            Instruction::Dup => DUP_CODE,
            Instruction::Jump(_) => JUMP_CODE,
            Instruction::Printf => PRINTF_CODE,
            Instruction::RemoteCall => REMOTE_CALL_CODE,
            Instruction::StartContext => START_CONTEXT_CODE,
            Instruction::EndContext => END_CONTEXT_CODE,
            Instruction::CheckInterrupt => CHECK_INTERRUPT_CODE,
            Instruction::Call(_) => CALL_CODE,
            Instruction::Ret => RET_CODE,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        match self {
            Instruction::Push(stack_value) => match stack_value {
                StackValue::String(s) => {
                    bytes.push(self.code());
                    let str_len = s.len();
                    bytes.extend_from_slice(&str_len.to_le_bytes());
                    bytes.extend_from_slice(s.as_bytes());
                }
                StackValue::Int(n) => {
                    bytes.push(self.code());
                    let n_bytes = n.to_le_bytes();
                    bytes.extend_from_slice(&n_bytes.len().to_le_bytes());
                    bytes.extend_from_slice(&n_bytes);
                }
            },
            Instruction::Pop => {
                bytes.push(self.code());
            }
            Instruction::Dec => {
                bytes.push(self.code());
            }
            Instruction::JmpIfZero(label) => {
                bytes.push(self.code());
                bytes.extend_from_slice(&label.len().to_le_bytes());
                bytes.extend_from_slice(label.as_bytes());
            }
            Instruction::Label(label) => {
                bytes.push(self.code());
                bytes.extend_from_slice(&label.len().to_le_bytes());
                bytes.extend_from_slice(label.as_bytes());
            }
            Instruction::Stdout => {
                bytes.push(self.code());
            }
            Instruction::Stderr => {
                bytes.push(self.code());
            }
            Instruction::Sleep(ms) => {
                bytes.push(self.code());
                let ms_bytes = ms.to_le_bytes();
                bytes.extend_from_slice(&ms_bytes.len().to_le_bytes());
                bytes.extend_from_slice(&ms_bytes);
            }
            Instruction::StoreVar(key, value) => {
                bytes.push(self.code());
                bytes.extend_from_slice(&key.len().to_le_bytes());
                bytes.extend_from_slice(key.as_bytes());
                bytes.extend_from_slice(&value.len().to_le_bytes());
                bytes.extend_from_slice(value.as_bytes());
            }
            Instruction::LoadVar(key) => {
                bytes.push(self.code());
                bytes.extend_from_slice(&key.len().to_le_bytes());
                bytes.extend_from_slice(key.as_bytes());
            }
            Instruction::Dup => {
                bytes.push(self.code());
            }
            Instruction::Jump(label) => {
                bytes.push(self.code());
                bytes.extend_from_slice(&label.len().to_le_bytes());
                bytes.extend_from_slice(label.as_bytes());
            }
            Instruction::Printf => {
                bytes.push(self.code());
            }
            Instruction::RemoteCall => {
                bytes.push(self.code());
            }
            Instruction::StartContext => {
                bytes.push(self.code());
            }
            Instruction::EndContext => {
                bytes.push(self.code());
            }
            Instruction::CheckInterrupt => {
                bytes.push(self.code());
            }
            Instruction::Call(label) => {
                bytes.push(self.code());
                bytes.extend_from_slice(&label.len().to_le_bytes());
                bytes.extend_from_slice(label.as_bytes());
            }
            Instruction::Ret => {
                bytes.push(self.code());
            }
        }
        bytes
    }
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
            Instruction::CheckInterrupt => write!(f, "CheckInterrupt"),
            Instruction::Call(label) => write!(f, "Call({})", label),
            Instruction::Ret => write!(f, "Ret"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_string_bytes() {
        let string_value = "Hello, world!".to_string();
        let string_len = string_value.len();
        let string_len_bytes = string_len.to_le_bytes();
        let instruction = Instruction::Push(StackValue::String(string_value.clone()));
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes[1..string_len_bytes.len() + 1], string_len_bytes);
        assert_eq!(
            &bytes[string_len_bytes.len() + 1..],
            string_value.as_bytes()
        );
        assert_eq!(bytes.len(), 1 + string_len_bytes.len() + string_value.len());
    }

    #[test]
    fn test_push_int_bytes() {
        let int_value: u64 = 4096;
        let int_value_bytes = int_value.to_le_bytes();
        let instruction = Instruction::Push(StackValue::Int(int_value));
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(
            bytes[1..int_value_bytes.len() + 1],
            int_value_bytes.len().to_le_bytes()
        );
        assert_eq!(&bytes[int_value_bytes.len() + 1..], &int_value_bytes);
        assert_eq!(
            bytes.len(),
            1 + int_value_bytes.len().to_le_bytes().len() + int_value_bytes.len()
        );
    }

    #[test]
    fn test_jmp_if_zero_bytes() {
        let label = "label".to_string();
        let label_bytes = label.as_bytes();
        let instruction = Instruction::JmpIfZero(label.clone());
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(
            bytes[1..label_bytes.len().to_le_bytes().len() + 1],
            label_bytes.len().to_le_bytes()
        );
        assert_eq!(
            &bytes[label_bytes.len().to_le_bytes().len() + 1..],
            label_bytes
        );
        assert_eq!(
            bytes.len(),
            1 + label_bytes.len().to_le_bytes().len() + label_bytes.len()
        );
    }

    #[test]
    fn test_label_bytes() {
        let label = "label".to_string();
        let label_bytes = label.as_bytes();
        let instruction = Instruction::Label(label.clone());
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(
            bytes[1..label_bytes.len().to_le_bytes().len() + 1],
            label_bytes.len().to_le_bytes()
        );
        assert_eq!(
            &bytes[label_bytes.len().to_le_bytes().len() + 1..],
            label_bytes
        );
        assert_eq!(
            bytes.len(),
            1 + label_bytes.len().to_le_bytes().len() + label_bytes.len()
        );
    }

    #[test]
    fn test_stdout_bytes() {
        let instruction = Instruction::Stdout;
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes.len(), 1);
    }

    #[test]
    fn test_stderr_bytes() {
        let instruction = Instruction::Stderr;
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes.len(), 1);
    }

    #[test]
    fn test_sleep_bytes() {
        let ms = 1000;
        let instruction = Instruction::Sleep(ms);
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(
            bytes[1..ms.to_le_bytes().len().to_le_bytes().len() + 1],
            ms.to_le_bytes().len().to_le_bytes()
        );
        assert_eq!(
            &bytes[ms.to_le_bytes().len().to_le_bytes().len() + 1..],
            &ms.to_le_bytes()
        );
        assert_eq!(
            bytes.len(),
            1 + ms.to_le_bytes().len().to_le_bytes().len() + ms.to_le_bytes().len()
        );
    }

    #[test]
    fn test_store_var_bytes() {
        let key = "key".to_string();
        let value = "value".to_string();

        let key_bytes = key.as_bytes();
        let value_bytes = value.as_bytes();

        let key_len = key_bytes.len();
        let value_len = value_bytes.len();

        let instruction = Instruction::StoreVar(key.clone(), value.clone());
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(
            bytes[1..key_len.to_le_bytes().len() + 1],
            key_len.to_le_bytes()
        );
        assert_eq!(
            &bytes[1 + key_len.to_le_bytes().len()
                ..1 + key_len.to_le_bytes().len() + key_bytes.len()],
            key_bytes
        );

        assert_eq!(
            bytes[1 + key_len.to_le_bytes().len() + key_bytes.len()
                ..1 + key_len.to_le_bytes().len()
                    + key_bytes.len()
                    + value_len.to_le_bytes().len()],
            value_len.to_le_bytes()
        );
        assert_eq!(
            &bytes[1
                + key_len.to_le_bytes().len()
                + key_bytes.len()
                + value_len.to_le_bytes().len()..],
            value_bytes
        );
    }

    #[test]
    fn test_load_var_bytes() {
        let key = "key".to_string();
        let key_bytes = key.as_bytes();
        let key_len = key_bytes.len();
        let instruction = Instruction::LoadVar(key.clone());
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(
            bytes[1..key_len.to_le_bytes().len() + 1],
            key_len.to_le_bytes()
        );
        assert_eq!(&bytes[1 + key_len.to_le_bytes().len()..], key_bytes);
        assert_eq!(
            bytes.len(),
            1 + key_len.to_le_bytes().len() + key_bytes.len()
        );
    }

    #[test]
    fn test_dup_bytes() {
        let instruction = Instruction::Dup;
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes.len(), 1);
    }

    #[test]
    fn test_jump_bytes() {
        let label = "label".to_string();
        let label_bytes = label.as_bytes();
        let instruction = Instruction::Jump(label.clone());
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(
            bytes[1..label_bytes.len().to_le_bytes().len() + 1],
            label_bytes.len().to_le_bytes()
        );
        assert_eq!(
            &bytes[1 + label_bytes.len().to_le_bytes().len()..],
            label_bytes
        );
        assert_eq!(
            bytes.len(),
            1 + label_bytes.len().to_le_bytes().len() + label_bytes.len()
        );
    }

    #[test]
    fn test_printf_bytes() {
        let instruction = Instruction::Printf;
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes.len(), 1);
    }

    #[test]
    fn test_remote_call_bytes() {
        let instruction = Instruction::RemoteCall;
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes.len(), 1);
    }

    #[test]
    fn test_start_context_bytes() {
        let instruction = Instruction::StartContext;
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes.len(), 1);
    }

    #[test]
    fn test_end_context_bytes() {
        let instruction = Instruction::EndContext;
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes.len(), 1);
    }

    #[test]
    fn test_check_interrupt_bytes() {
        let instruction = Instruction::CheckInterrupt;
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes.len(), 1);
    }

    #[test]
    fn test_call_bytes() {
        let label = "label".to_string();
        let label_bytes = label.as_bytes();
        let instruction = Instruction::Call(label.clone());
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(
            bytes[1..label_bytes.len().to_le_bytes().len() + 1],
            label_bytes.len().to_le_bytes()
        );
    }

    #[test]
    fn test_ret_bytes() {
        let instruction = Instruction::Ret;
        let bytes = instruction.to_bytes();
        assert_eq!(bytes[0], instruction.code());
        assert_eq!(bytes.len(), 1);
    }
}
