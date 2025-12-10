# Notification Worker

The Notification Worker consumes `notification.requested` events from the message bus and sends push notifications or emails.

## Features

- ✅ Consumes `notification.requested` events from NATS
- ✅ Sends push notifications via FCM (Firebase Cloud Messaging)
- ✅ Sends emails via SMTP
- ✅ APNS support structure (requires JWT implementation)
- ✅ Publishes `notification.sent` events on success
- ✅ Publishes `notification.failed` events on failure
- ✅ Graceful shutdown on Ctrl+C
- ✅ Structured logging
- ✅ Configurable via environment variables

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
# Message Bus
MESSAGE_BUS_URL=nats://localhost:4222
MESSAGE_BUS_STREAM_NAME=armoricore-events

# FCM (Firebase Cloud Messaging) for Android Push
FCM_API_KEY=your-fcm-server-key

# APNS (Apple Push Notification Service) for iOS Push
APNS_KEY_ID=your-apns-key-id
APNS_TEAM_ID=your-apns-team-id
APNS_BUNDLE_ID=your-app-bundle-id
APNS_KEY_PATH=/path/to/apns-key.p8

# SMTP for Email
SMTP_RELAY=smtp.example.com:587
SMTP_USERNAME=your-smtp-username
SMTP_PASSWORD=your-smtp-password
SMTP_FROM=noreply@example.com

# Device Token Database (Optional - falls back to event payload if not configured)
DATABASE_URL=postgresql://user:password@localhost:5432/armoricore

# Retry Configuration (Optional)
NOTIFICATION_MAX_RETRIES=3              # Maximum retry attempts (default: 3)
NOTIFICATION_RETRY_INITIAL_DELAY=1       # Initial delay in seconds (default: 1)
NOTIFICATION_RETRY_MAX_DELAY=60          # Maximum delay in seconds (default: 60)
NOTIFICATION_RETRY_MULTIPLIER=2.0        # Exponential backoff multiplier (default: 2.0)

# Rate Limiting (Optional)
NOTIFICATION_RATE_LIMIT_RPS=100          # Requests per second
NOTIFICATION_RATE_LIMIT_BURST=200        # Burst capacity

LOG_LEVEL=info
```

**Note:** At least one notification service (FCM, APNS, or SMTP) must be configured.

### Device Token Database (Optional)

The notification worker supports storing device tokens in a PostgreSQL database for better scalability. If `DATABASE_URL` is not configured, the worker will fall back to reading device tokens from the event payload.

**Benefits:**
- Supports multiple devices per user
- Automatic token management
- Platform-specific filtering (iOS/Android)
- Better scalability than event payloads

**Database Schema:**
The worker automatically creates the `device_tokens` table with:
- `user_id` (UUID) - User identifier
- `device_token` (TEXT) - Device token string
- `platform` (VARCHAR) - 'ios' or 'android'
- `created_at`, `updated_at` (TIMESTAMPTZ) - Timestamps

**Usage:**
1. Set `DATABASE_URL` environment variable
2. The worker will automatically create tables and indexes
3. Device tokens can be stored via API or event payloads
4. Tokens are automatically retrieved when sending notifications

### APNS Configuration

For iOS push notifications, you need:
1. **APNS Key File (.p8)**: Download from Apple Developer Portal
2. **Key ID**: Found in the key name (e.g., `ABC123DEFG`)
3. **Team ID**: Found in your Apple Developer account
4. **Bundle ID**: Your app's bundle identifier

**Environment Variables:**
```bash
APNS_KEY_ID=your-apns-key-id          # Required
APNS_TEAM_ID=your-team-id            # Required
APNS_BUNDLE_ID=com.yourapp.bundle    # Required
APNS_KEY_PATH=/path/to/AuthKey.p8    # Required (path to .p8 file)
APNS_USE_SANDBOX=false                # Optional (default: false, use true for development)
```

**Features:**
- ✅ **JWT Token Caching**: Tokens are cached for up to 1 hour (APNS token lifetime)
- ✅ **Automatic Refresh**: Tokens are automatically refreshed when less than 5 minutes remain
- ✅ **Key Caching**: APNS key file is cached in memory to reduce I/O
- ✅ **Production/Sandbox**: Supports both production and sandbox endpoints
- ✅ **ES256 Signing**: Uses ECDSA P-256 with SHA-256 algorithm
- ✅ **Error Handling**: Comprehensive error messages for debugging

**How It Works:**
1. Loads the .p8 key file on initialization (cached in memory)
2. Generates JWT tokens with ES256 algorithm when needed
3. Caches tokens for up to 1 hour (APNS token validity period)
4. Automatically refreshes tokens before expiration (5-minute buffer)
5. Uses production endpoint by default, sandbox for development/testing

**Token Structure:**
- **Header**: `{"alg": "ES256", "kid": "<key_id>", "typ": "JWT"}`
- **Claims**: `{"iss": "<team_id>", "iat": <unix_timestamp>}`
- **Validity**: 1 hour from issuance (managed automatically)

### Retry Logic

The notification worker implements automatic retry with exponential backoff for transient failures:

- **Configurable retries**: Set `NOTIFICATION_MAX_RETRIES` (default: 3)
- **Exponential backoff**: Delays increase exponentially (1s, 2s, 4s, ...)
- **Max delay cap**: Prevents excessive delays (default: 60s)
- **Smart error detection**: Automatically detects retryable vs permanent errors
- **Retryable errors**: Network issues, timeouts, rate limits (429, 503, 502)
- **Permanent errors**: Authentication failures (401, 403), invalid data

### Dead Letter Queue

Failed notifications are automatically sent to a dead letter queue:

- **Topic**: `notification.dead_letter` (configurable)
- **Triggers**: 
  - Permanent errors (sent immediately)
  - Transient errors that fail after all retries
- **Payload includes**:
  - Original event information
  - Failure reason
  - Retry count
  - Timestamp

### Rate Limiting

Optional rate limiting prevents overwhelming notification services:

- **Token bucket algorithm**: Smooth rate limiting with burst capacity
- **Configurable**: Set `NOTIFICATION_RATE_LIMIT_RPS` and `NOTIFICATION_RATE_LIMIT_BURST`
- **Automatic refill**: Tokens refill automatically over time
- **Blocking**: Worker waits for tokens if rate limit exceeded

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

## Implementation Status

- [x] FCM integration for Android push notifications ✅
- [x] APNS integration for iOS push notifications ✅ (JWT authentication)
- [x] SMTP integration for emails ✅
- [x] Device token database integration ✅ (PostgreSQL, optional)
- [x] Retry logic with exponential backoff ✅
- [x] Dead letter queue for failed notifications ✅
- [x] Rate limiting ✅
- [ ] Metrics and monitoring

