//! Notification Worker - Event Processing
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


use crate::database::DeviceTokenDb;
use crate::dead_letter_queue::DeadLetterQueue;
use crate::rate_limiter::RateLimiter;
use crate::retry::{is_retryable_error, RetryConfig};
use crate::sender::NotificationSender;
use armoricore_keys::init_key_store;
use armoricore_types::{
    schemas::{
        NotificationFailedPayload, NotificationRequestedPayload, NotificationSentPayload,
        NotificationType,
    },
    Event, EventType,
};
use message_bus_client::traits::MessageBusClient;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Notification Worker that processes notification requests
pub struct NotificationWorker {
    message_bus: Arc<dyn MessageBusClient>,
    sender: NotificationSender,
    retry_config: RetryConfig,
    rate_limiter: Option<Arc<RateLimiter>>,
    dead_letter_queue: DeadLetterQueue,
}

impl NotificationWorker {
    /// Create a new notification worker
    pub async fn new(message_bus: Arc<dyn MessageBusClient>) -> anyhow::Result<Self> {
        // Initialize device token database (optional)
        let device_token_db = match DeviceTokenDb::new().await {
            Ok(db) => {
                if db.is_available() {
                    Some(std::sync::Arc::new(db))
                } else {
                    None
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to initialize device token database, will use event payloads");
                None
            }
        };

        // Initialize key store (optional - falls back to environment variables)
        let key_store = init_key_store(None).await.ok();
        
        // Create sender with key store if available, otherwise use environment variables
        let mut sender = if let Some(ref ks) = key_store {
            NotificationSender::with_key_store(ks).await?
        } else {
            warn!("Key store not available, using environment variables");
            NotificationSender::new()
        };
        
        if let Some(db) = device_token_db {
            sender = sender.with_device_token_db(db);
        }

        // Load retry configuration from environment
        let retry_config = RetryConfig {
            max_retries: std::env::var("NOTIFICATION_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            initial_delay_secs: std::env::var("NOTIFICATION_RETRY_INITIAL_DELAY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            max_delay_secs: std::env::var("NOTIFICATION_RETRY_MAX_DELAY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            multiplier: std::env::var("NOTIFICATION_RETRY_MULTIPLIER")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2.0),
        };

        // Initialize rate limiter (optional)
        let rate_limiter = if let (Ok(requests_per_sec), Ok(burst)) = (
            std::env::var("NOTIFICATION_RATE_LIMIT_RPS"),
            std::env::var("NOTIFICATION_RATE_LIMIT_BURST"),
        ) {
            if let (Ok(rps), Ok(burst)) = (requests_per_sec.parse::<u32>(), burst.parse::<u32>()) {
                info!(
                    requests_per_second = rps,
                    burst_capacity = burst,
                    "Rate limiting enabled"
                );
                Some(Arc::new(RateLimiter::new(burst, rps, Duration::from_secs(1))))
            } else {
                warn!("Invalid rate limit configuration, disabling rate limiting");
                None
            }
        } else {
            None
        };

        // Initialize dead letter queue
        let dead_letter_queue = DeadLetterQueue::new(message_bus.clone());

        Ok(Self {
            message_bus,
            sender,
            retry_config,
            rate_limiter,
            dead_letter_queue,
        })
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

        // Apply rate limiting if configured
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }

        // Send the notification with retry logic
        #[allow(unused_assignments)]
        let mut last_error = None;
        let mut retry_count = 0;

        let send_result = loop {
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
                Ok(_) => break Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    retry_count += 1;

                    // Check if error is retryable
                    let is_retryable = last_error.as_ref()
                        .map(|e| is_retryable_error(e))
                        .unwrap_or(false);

                    if !is_retryable || retry_count > self.retry_config.max_retries {
                        break Err(last_error.ok_or_else(|| {
                            anyhow::anyhow!("Retry logic error: no errors recorded but operation failed")
                        })?);
                    }

                    // Calculate delay for this retry attempt
                    let delay = self.retry_config.delay_for_attempt(retry_count);
                    warn!(
                        notification_id = %notification_id,
                        attempt = retry_count,
                        max_retries = self.retry_config.max_retries,
                        delay_secs = delay.as_secs(),
                        error = %last_error.as_ref()
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "Unknown error".to_string()),
                        "Notification send failed, retrying with exponential backoff"
                    );

                    tokio::time::sleep(delay).await;
                }
            }
        };

        match send_result {
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
                // Check if error is retryable
                let is_retryable = is_retryable_error(&e);

                if !is_retryable {
                    // Permanent error - send to dead letter queue immediately
                    warn!(
                        notification_id = %notification_id,
                        error = %e,
                        "Permanent error, sending to dead letter queue"
                    );

                    self.dead_letter_queue
                        .send_to_dlq(event, notification_id, payload.user_id, &e.to_string(), 0)
                        .await?;
                } else {
                    // Transient error that failed after all retries - send to DLQ
                    warn!(
                        notification_id = %notification_id,
                        error = %e,
                        retries = retry_count - 1,
                        "Failed after all retries, sending to dead letter queue"
                    );

                    self.dead_letter_queue
                        .send_to_dlq(
                            event,
                            notification_id,
                            payload.user_id,
                            &e.to_string(),
                            retry_count - 1,
                        )
                        .await?;
                }

                // Also publish notification.failed event for monitoring
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

