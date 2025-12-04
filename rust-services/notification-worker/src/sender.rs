//! Notification Sender
//!
//! Handles sending push notifications and emails.
//! Currently implements mock senders for development/testing.

use armoricore_types::schemas::NotificationType;
use serde_json::Value;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

/// Notification sender that handles different notification types
pub struct NotificationSender {
    // Future: Add FCM client, APNS client, email client, etc.
}

impl NotificationSender {
    /// Create a new notification sender
    pub fn new() -> Self {
        Self {}
    }

    /// Send a notification
    pub async fn send_notification(
        &self,
        user_id: &Uuid,
        notification_type: &NotificationType,
        title: &str,
        body: &str,
        data: &Value,
    ) -> anyhow::Result<()> {
        info!(
            user_id = %user_id,
            notification_type = ?notification_type,
            title = title,
            "Sending notification"
        );

        match notification_type {
            NotificationType::Push => self.send_push_notification(user_id, title, body, data).await,
            NotificationType::Email => self.send_email(user_id, title, body, data).await,
        }
    }

    /// Send a push notification (mock implementation)
    async fn send_push_notification(
        &self,
        user_id: &Uuid,
        title: &str,
        body: &str,
        data: &Value,
    ) -> anyhow::Result<()> {
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Mock implementation - in production, this would:
        // 1. Look up user's device tokens from database
        // 2. Send to FCM (Android) or APNS (iOS)
        // 3. Handle errors and retries

        info!(
            user_id = %user_id,
            title = title,
            body = body,
            data = %data,
            "Mock push notification sent"
        );

        // Simulate occasional failures for testing
        if title.contains("FAIL") {
            return Err(anyhow::anyhow!("Simulated push notification failure"));
        }

        Ok(())
    }

    /// Send an email (mock implementation)
    async fn send_email(
        &self,
        user_id: &Uuid,
        title: &str,
        body: &str,
        data: &Value,
    ) -> anyhow::Result<()> {
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Mock implementation - in production, this would:
        // 1. Look up user's email address from database
        // 2. Send email via SMTP, SendGrid, SES, etc.
        // 3. Handle errors and retries

        info!(
            user_id = %user_id,
            title = title,
            body = body,
            data = %data,
            "Mock email sent"
        );

        // Simulate occasional failures for testing
        if title.contains("FAIL") {
            return Err(anyhow::anyhow!("Simulated email failure"));
        }

        Ok(())
    }
}

impl Default for NotificationSender {
    fn default() -> Self {
        Self::new()
    }
}

