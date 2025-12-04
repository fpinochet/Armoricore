//! Traits for message bus operations

use async_trait::async_trait;
use armoricore_types::Event;
use std::pin::Pin;
use futures::Stream;

/// Trait for message bus clients
#[async_trait]
pub trait MessageBusClient: Send + Sync {
    /// Publish an event to the message bus
    async fn publish(&self, event: &Event) -> Result<(), crate::error::MessageBusError>;

    /// Subscribe to events of a specific type
    /// Returns a stream of events
    fn subscribe(
        &self,
        event_type: &str,
    ) -> Pin<Box<dyn Stream<Item = std::result::Result<Event, crate::error::MessageBusError>> + Send + '_>>;

    /// Check if the client is connected
    async fn is_connected(&self) -> bool;

    /// Get the client type name
    fn client_type(&self) -> &str;
}

