//! Shared error types for SmaTelcom Rust layer.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SmaError {
    #[error("Ollama unreachable at {0}: {1}")]
    OllamaUnreachable(String, String),

    #[error("Ollama API error: {0}")]
    OllamaApi(String),

    #[error("Safety linter blocked command: {0}")]
    SafetyBlocked(String),

    #[error("Knowledge base error: {0}")]
    KnowledgeBase(String),

    #[error("Invalid intent: {0}")]
    InvalidIntent(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl Serialize for SmaError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type SmaResult<T> = Result<T, SmaError>;
