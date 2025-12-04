//! Notification Worker - Event Processing

use crate::sender::NotificationSender;
use armoricore_types::{
    schemas::{
        NotificationFailedPayload, NotificationRequestedPayload, NotificationSentPayload,
        NotificationType,
    },
    Event, EventType,
};
use message_bus_client::traits::MessageBusClient;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Notification Worker that processes notification requests
pub struct NotificationWorker {
    message_bus: Arc<dyn MessageBusClient>,
    sender: NotificationSender,
}

impl NotificationWorker {
    /// Create a new notification worker
    pub fn new(message_bus: Arc<dyn MessageBusClient>) -> Self {
        Self {
            message_bus,
            sender: NotificationSender::new(),
        }
    }

    /// Run the worker - consume events and process them
    pub async fn run(&self) -> anyhow::Result<()> {
        info!("Subscribing to notification.requested events");

        let mut event_stream = self.message_bus.subscribe("notification.requested");

        info!("Waiting for notification requests...");

        while let Some(event_result) = event_stream.next().await {
            match event_result {
                Ok(event) => {
                    info!(
                        event_id = %event.event_id,
                        event_type = ?event.event_type,
                        "Received notification request"
                    );

                    if let Err(e) = self.process_notification_request(&event).await {
                        error!(
                            event_id = %event.event_id,
                            error = %e,
                            "Failed to process notification request"
                        );
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error receiving event from message bus");
                    // Continue processing - don't crash on individual event errors
                }
            }
        }

        warn!("Event stream ended");
        Ok(())
    }

    /// Process a single notification request
    async fn process_notification_request(&self, event: &Event) -> anyhow::Result<()> {
        // Deserialize the payload
        let payload: NotificationRequestedPayload = event
            .payload_as()
            .map_err(|e| anyhow::anyhow!("Invalid payload: {}", e))?;

        let notification_id = Uuid::new_v4();

        info!(
            notification_id = %notification_id,
            user_id = %payload.user_id,
            notification_type = ?payload.notification_type,
            "Processing notification request"
        );

        // Send the notification
        match self
            .sender
            .send_notification(
                &payload.user_id,
                &payload.notification_type,
                &payload.title,
                &payload.body,
                &payload.data,
            )
            .await
        {
            Ok(_) => {
                // Publish notification.sent event
                self.publish_notification_sent(
                    payload.user_id,
                    notification_id,
                    payload.notification_type,
                )
                .await?;

                info!(
                    notification_id = %notification_id,
                    "Notification sent successfully"
                );
            }
            Err(e) => {
                // Publish notification.failed event
                self.publish_notification_failed(
                    payload.user_id,
                    notification_id,
                    payload.notification_type,
                    &e.to_string(),
                )
                .await?;

                error!(
                    notification_id = %notification_id,
                    error = %e,
                    "Failed to send notification"
                );
            }
        }

        Ok(())
    }

    /// Publish a notification.sent event
    async fn publish_notification_sent(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
        notification_type: NotificationType,
    ) -> anyhow::Result<()> {
        let payload = NotificationSentPayload {
            user_id,
            notification_id,
            notification_type,
            sent_at: chrono::Utc::now(),
        };

        let event = Event::new(EventType::NotificationSent, "notification-worker", payload)
            .map_err(|e| anyhow::anyhow!("Failed to create event: {}", e))?;

        self.message_bus
            .publish(&event)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish event: {}", e))?;

        Ok(())
    }

    /// Publish a notification.failed event
    async fn publish_notification_failed(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
        notification_type: NotificationType,
        error: &str,
    ) -> anyhow::Result<()> {
        let payload = NotificationFailedPayload {
            user_id,
            notification_id,
            notification_type,
            error: error.to_string(),
            failed_at: chrono::Utc::now(),
        };

        let event = Event::new(EventType::NotificationFailed, "notification-worker", payload)
            .map_err(|e| anyhow::anyhow!("Failed to create event: {}", e))?;

        self.message_bus
            .publish(&event)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish event: {}", e))?;

        Ok(())
    }
}

