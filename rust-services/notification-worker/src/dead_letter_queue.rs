//! Dead Letter Queue for failed notifications
// Copyright 2025 Francisco F. Pinochet
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.


use armoricore_types::{Event, EventType};
use message_bus_client::traits::MessageBusClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Dead letter queue event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterPayload {
    pub original_event_id: Uuid,
    pub original_event_type: String,
    pub notification_id: Uuid,
    pub user_id: Uuid,
    pub failure_reason: String,
    pub retry_count: u32,
    pub failed_at: chrono::DateTime<chrono::Utc>,
    pub original_payload: serde_json::Value,
}

/// Dead letter queue handler
pub struct DeadLetterQueue {
    message_bus: Arc<dyn MessageBusClient>,
    topic: String,
}

impl DeadLetterQueue {
    /// Create a new dead letter queue
    pub fn new(message_bus: Arc<dyn MessageBusClient>) -> Self {
        Self {
            message_bus,
            topic: "notification.dead_letter".to_string(),
        }
    }

    /// Create with custom topic
    pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = topic.into();
        self
    }

    /// Send a failed notification to the dead letter queue
    pub async fn send_to_dlq(
        &self,
        original_event: &Event,
        notification_id: Uuid,
        user_id: Uuid,
        failure_reason: &str,
        retry_count: u32,
    ) -> anyhow::Result<()> {
        let payload = DeadLetterPayload {
            original_event_id: original_event.event_id,
            original_event_type: format!("{:?}", original_event.event_type),
            notification_id,
            user_id,
            failure_reason: failure_reason.to_string(),
            retry_count,
            failed_at: chrono::Utc::now(),
            original_payload: original_event.payload.clone(),
        };

        let event = Event::new(
            EventType::NotificationFailed, // Reuse existing event type
            "notification-worker",
            payload,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create DLQ event: {}", e))?;

        self.message_bus
            .publish(&event)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish to dead letter queue: {}", e))?;

        error!(
            notification_id = %notification_id,
            user_id = %user_id,
            retry_count = retry_count,
            reason = failure_reason,
            "Notification sent to dead letter queue"
        );

        info!(
            notification_id = %notification_id,
            topic = self.topic,
            "Dead letter queue event published"
        );

        Ok(())
    }
}

