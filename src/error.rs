use std::fmt;

#[derive(Debug, Clone)]
pub struct CliError {
    pub message: String,
    pub exit_code: i32,
}

impl CliError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self { message: message.into(), exit_code: 2 }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self { message: message.into(), exit_code: 3 }
    }

    pub fn ambiguous(message: impl Into<String>) -> Self {
        Self { message: message.into(), exit_code: 4 }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self { message: message.into(), exit_code: 5 }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}
