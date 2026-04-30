use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    User,
    Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub kind: ErrorKind,
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum VideoGenError {
    #[error("{message}")]
    User { message: String },
    #[error("{message}")]
    Runtime { message: String },
}

impl VideoGenError {
    pub fn user(message: impl Into<String>) -> Self {
        Self::User {
            message: message.into(),
        }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime {
            message: message.into(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::User { .. } => ErrorKind::User,
            Self::Runtime { .. } => ErrorKind::Runtime,
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self.kind() {
            ErrorKind::User => 2,
            ErrorKind::Runtime => 1,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::User { message } | Self::Runtime { message } => message,
        }
    }

    pub fn payload(&self) -> ErrorPayload {
        ErrorPayload {
            kind: self.kind(),
            code: self.exit_code(),
            message: self.message().to_string(),
        }
    }
}

impl From<std::io::Error> for VideoGenError {
    fn from(value: std::io::Error) -> Self {
        Self::runtime(value.to_string())
    }
}

impl From<serde_json::Error> for VideoGenError {
    fn from(value: serde_json::Error) -> Self {
        Self::runtime(value.to_string())
    }
}
