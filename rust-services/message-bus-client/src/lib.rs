//! Message Bus Client Library
//!
//! Provides a unified interface for publishing and consuming events
//! from the message bus (NATS JetStream or RabbitMQ).

pub mod nats;
pub mod error;
pub mod traits;

pub use error::*;
pub use traits::*;
pub use nats::*;

