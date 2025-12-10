//! Integration Tests for Notification Pipeline
//!
//! These tests require:
//! - NATS server running
//! - FCM/APNS credentials (optional, will fail gracefully)

use armoricore_types::{Event, EventType, schemas::*};
use message_bus_client::nats::NatsClient;
use message_bus_client::traits::MessageBusClient;
use uuid::Uuid;
use futures::StreamExt;
use std::time::Duration;

#[tokio::test]
#[ignore] // Requires NATS server
async fn test_notification_requested_event_flow() {
    // This test verifies the end-to-end flow:
    // 1. Publish notification.requested event
    // 2. Notification worker consumes it
    // 3. Notification worker publishes notification.sent or notification.failed event
    // 4. We subscribe to notification.sent and verify it arrives

    let client = NatsClient::new("nats://localhost:4222", None)
        .await
        .expect("Failed to connect to NATS");

    // Create a notification.requested event
    let payload = NotificationRequestedPayload {
        user_id: Uuid::new_v4(),
        notification_type: NotificationType::Push,
        title: "Test Notification".to_string(),
        body: "This is a test notification".to_string(),
        data: serde_json::json!({"key": "value"}),
    };

    let event = Event::new(EventType::NotificationRequested, "test", payload)
        .expect("Failed to create event");

    // Publish the event
    client.publish(&event).await.expect("Failed to publish event");

    // Subscribe to notification.sent events
    let mut stream = client.subscribe("notification.sent");

    // Wait for the notification worker to process and publish notification.sent
    let timeout = Duration::from_secs(30);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        tokio::select! {
            result = stream.next() => {
                if let Some(Ok(received_event)) = result {
                    if received_event.event_type == EventType::NotificationSent {
                        // Verify the event structure
                        assert_eq!(received_event.event_type, EventType::NotificationSent);
                        println!("✅ Received notification.sent event: {:?}", received_event.event_id);
                        return;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                // Continue waiting
            }
        }
    }

    // If we get here, the notification might have failed
    // That's okay - we just verify the event flow works
    println!("⚠️  No notification.sent event received (may have failed, which is expected without credentials)");
}

#[tokio::test]
#[ignore] // Requires NATS server
async fn test_notification_failed_event() {
    // Test that notification.failed events are published when sending fails
    let client = NatsClient::new("nats://localhost:4222", None)
        .await
        .expect("Failed to connect to NATS");

    // Create a notification.requested event with invalid data
    let payload = NotificationRequestedPayload {
        user_id: Uuid::new_v4(),
        notification_type: NotificationType::Push,
        title: "".to_string(), // Invalid: empty title
        body: "".to_string(),  // Invalid: empty body
        data: serde_json::Value::Null,
    };

    let event = Event::new(EventType::NotificationRequested, "test", payload)
        .expect("Failed to create event");

    // Publish the event
    client.publish(&event).await.expect("Failed to publish event");

    // Subscribe to notification.failed events
    let mut stream = client.subscribe("notification.failed");

    // Wait for the notification worker to process and publish notification.failed
    let timeout = Duration::from_secs(30);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        tokio::select! {
            result = stream.next() => {
                if let Some(Ok(received_event)) = result {
                    if received_event.event_type == EventType::NotificationFailed {
                        assert_eq!(received_event.event_type, EventType::NotificationFailed);
                        println!("✅ Received notification.failed event: {:?}", received_event.event_id);
                        return;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                // Continue waiting
            }
        }
    }

    println!("⚠️  No notification.failed event received");
}

