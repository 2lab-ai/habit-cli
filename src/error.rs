use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct CliError {
    pub message: String,
    pub exit_code: i32,
}

impl CliError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 2,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 3,
        }
    }

    pub fn ambiguous(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 4,
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 5,
        }
    }
}
