//! MLVInspector backend error types.
//!
//! Pure Rust error type — no serialization needed for Dioxus.
//! Frontend receives errors via direct Rust types, not JSON over IPC.

use std::fmt;

/// Application error type.
#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    Io(String),
    Process(String),
    Parse(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(msg) => write!(f, "IO error: {msg}"),
            AppError::Process(msg) => write!(f, "ILInspector process error: {msg}"),
            AppError::Parse(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}
