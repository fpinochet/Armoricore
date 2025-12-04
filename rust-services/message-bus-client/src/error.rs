//! Error types for message bus operations

use thiserror::Error;

/// Errors that can occur in message bus operations
#[derive(Error, Debug)]
pub enum MessageBusError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Publish error: {0}")]
    Publish(String),

    #[error("Subscribe error: {0}")]
    Subscribe(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("NATS error: {0}")]
    Nats(#[from] async_nats::Error),

    #[error("Timeout error")]
    Timeout,

    #[error("Invalid subject: {0}")]
    InvalidSubject(String),
}

pub type Result<T> = std::result::Result<T, MessageBusError>;

