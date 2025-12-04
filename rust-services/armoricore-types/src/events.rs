//! Event type definitions for the message bus

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schemas::*;
use crate::error::{ArmoricoreError, Result};

/// Event type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Media events
    #[serde(rename = "media.uploaded")]
    MediaUploaded,
    #[serde(rename = "media.ready")]
    MediaReady,
    
    // Notification events
    #[serde(rename = "notification.requested")]
    NotificationRequested,
    #[serde(rename = "notification.sent")]
    NotificationSent,
    #[serde(rename = "notification.failed")]
    NotificationFailed,
    
    // Transcription events
    #[serde(rename = "transcription.requested")]
    TranscriptionRequested,
    #[serde(rename = "transcription.complete")]
    TranscriptionComplete,
    
    // Chat events
    #[serde(rename = "chat.message")]
    ChatMessage,
    
    // Presence events
    #[serde(rename = "presence.update")]
    PresenceUpdate,
}

/// Base event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event type identifier
    pub event_type: EventType,
    
    /// Unique event identifier
    pub event_id: Uuid,
    
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Source service that published the event
    pub source: String,
    
    /// Event payload (type depends on event_type)
    pub payload: serde_json::Value,
}

impl Event {
    /// Create a new event
    pub fn new(
        event_type: EventType,
        source: impl Into<String>,
        payload: impl Serialize,
    ) -> Result<Self> {
        let payload_value = serde_json::to_value(payload)?;
        
        Ok(Self {
            event_type,
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: source.into(),
            payload: payload_value,
        })
    }

    /// Deserialize the payload into a specific type
    pub fn payload_as<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        serde_json::from_value(self.payload.clone())
            .map_err(|e| ArmoricoreError::InvalidPayload(e.to_string()))
    }

    /// Validate the event structure
    pub fn validate(&self) -> Result<()> {
        // Validate event type matches payload structure
        match self.event_type {
            EventType::MediaUploaded => {
                let _: MediaUploadedPayload = self.payload_as()?;
            }
            EventType::MediaReady => {
                let _: MediaReadyPayload = self.payload_as()?;
            }
            EventType::NotificationRequested => {
                let _: NotificationRequestedPayload = self.payload_as()?;
            }
            EventType::NotificationSent => {
                let _: NotificationSentPayload = self.payload_as()?;
            }
            EventType::NotificationFailed => {
                let _: NotificationFailedPayload = self.payload_as()?;
            }
            EventType::TranscriptionRequested => {
                let _: TranscriptionRequestedPayload = self.payload_as()?;
            }
            EventType::TranscriptionComplete => {
                let _: TranscriptionCompletePayload = self.payload_as()?;
            }
            EventType::ChatMessage => {
                let _: ChatMessagePayload = self.payload_as()?;
            }
            EventType::PresenceUpdate => {
                let _: PresenceUpdatePayload = self.payload_as()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::*;

    #[test]
    fn test_event_creation() {
        let payload = MediaUploadedPayload {
            media_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            file_path: "s3://bucket/key".to_string(),
            content_type: "video/mp4".to_string(),
            file_size: 1048576,
            metadata: Default::default(),
        };

        let event = Event::new(EventType::MediaUploaded, "test-service", payload).unwrap();
        assert_eq!(event.event_type, EventType::MediaUploaded);
        assert_eq!(event.source, "test-service");
    }

    #[test]
    fn test_event_serialization() {
        let payload = NotificationRequestedPayload {
            user_id: Uuid::new_v4(),
            notification_type: NotificationType::Push,
            title: "Test".to_string(),
            body: "Test body".to_string(),
            data: Default::default(),
        };

        let event = Event::new(EventType::NotificationRequested, "test", payload).unwrap();
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        
        assert_eq!(event.event_type, deserialized.event_type);
        assert_eq!(event.event_id, deserialized.event_id);
    }
}

