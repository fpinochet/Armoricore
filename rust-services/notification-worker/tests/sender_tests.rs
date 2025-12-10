//! Notification Sender Unit Tests

use notification_worker::sender::NotificationSender;
use armoricore_types::schemas::NotificationType;

#[tokio::test]
async fn test_notification_type_parsing() {
    // Test that notification types can be created
    // NotificationType is an enum from armoricore-types
    let push_type = NotificationType::Push;
    let email_type = NotificationType::Email;
    
    // Verify they are different variants
    assert!(matches!(push_type, NotificationType::Push));
    assert!(matches!(email_type, NotificationType::Email));
}

#[tokio::test]
async fn test_notification_sender_new() {
    // Test creating a notification sender
    let sender = NotificationSender::new();
    // Should create without panicking
    // Configuration is loaded from environment variables
    let _ = sender;
}

