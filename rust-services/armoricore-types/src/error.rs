//! Error types for Armoricore

use thiserror::Error;

/// Errors that can occur in Armoricore services
#[derive(Error, Debug)]
pub enum ArmoricoreError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid event type: {0}")]
    InvalidEventType(String),

    #[error("Invalid event payload: {0}")]
    InvalidPayload(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, ArmoricoreError>;

