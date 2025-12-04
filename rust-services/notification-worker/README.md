# Notification Worker

The Notification Worker consumes `notification.requested` events from the message bus and sends push notifications or emails.

## Features

- ✅ Consumes `notification.requested` events from NATS
- ✅ Sends push notifications (mock implementation)
- ✅ Sends emails (mock implementation)
- ✅ Publishes `notification.sent` events on success
- ✅ Publishes `notification.failed` events on failure
- ✅ Graceful shutdown on Ctrl+C
- ✅ Structured logging

## Usage

### Prerequisites

- NATS server running (default: `nats://localhost:4222`)
- Environment variables configured (see `.env.example`)

### Running

```bash
# From the rust-services directory
cargo run --bin notification-worker
```

### Configuration

Set the following environment variables:

```bash
MESSAGE_BUS_URL=nats://localhost:4222
MESSAGE_BUS_STREAM_NAME=armoricore-events
LOG_LEVEL=info
```

### Testing

You can test the worker by publishing a `notification.requested` event to NATS. The worker will:

1. Receive the event
2. Process the notification (mock)
3. Publish either `notification.sent` or `notification.failed`

#### Example Event

```json
{
  "event_type": "notification.requested",
  "event_id": "uuid",
  "timestamp": "2025-01-01T00:00:00Z",
  "source": "php-backend",
  "payload": {
    "user_id": "uuid",
    "notification_type": "push",
    "title": "New message",
    "body": "You have a new message",
    "data": {
      "message_id": "uuid"
    }
  }
}
```

## Architecture

```
┌──────────────┐
│ Message Bus  │
│  (NATS)      │
└──────┬───────┘
       │
       │ notification.requested
       ▼
┌──────────────────┐
│ Notification     │
│ Worker           │
├──────────────────┤
│ 1. Consume event │
│ 2. Send notif    │
│ 3. Publish result│
└──────────────────┘
       │
       ├─► notification.sent (success)
       └─► notification.failed (error)
```

## Future Enhancements

- [ ] Real FCM integration for Android push notifications
- [ ] Real APNS integration for iOS push notifications
- [ ] SMTP/SendGrid/SES integration for emails
- [ ] Retry logic with exponential backoff
- [ ] Dead letter queue for failed notifications
- [ ] Metrics and monitoring
- [ ] Rate limiting
- [ ] User device token management

