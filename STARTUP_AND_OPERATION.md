# System Startup & Operation Flow

## Overview

This document explains how Armoricore starts up and operates on a Linux server, detailing the initialization sequence, service dependencies, and runtime behavior.

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Linux Server                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐   ┌──────────────┐    │
│  │    NATS      │    │ PostgreSQL  │    │   FFmpeg     │    │
│  │  (Port 4222) │    │ (Port 5432) │    │  (CLI Tool)  │    │
│  └──────┬───────┘    └──────┬──────┘    └──────────────┘    │
│         │                    │                              │
│         │                    │                              │
│  ┌──────▼────────────────────▼──────────────────────────┐   │
│  │              Application Services                    │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │                                                      │   │
│  │  ┌──────────────────┐  ┌──────────────────┐          │   │
│  │  │ Media Processor  │  │ Notification     │          │   │
│  │  │   (Rust)        │  │ Worker (Rust)    │           │   │
│  │  └────────┬─────────┘  └────────┬─────────┘          │   │
│  │           │                      │                   │   │
│  │           └──────────┬───────────┘                   │   │
│  │                      │                               │   │
│  │              ┌────────▼────────┐                     │   │
│  │              │  Elixir Phoenix │                     │   │
│  │              │  (Port 4000)    │                     │   │
│  │              └─────────────────┘                     │   │
│  │                                                      │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Startup Sequence

### Phase 1: Infrastructure Services (Systemd)

```
┌─────────────────────────────────────────────────────────┐
│ 1. NATS Server Starts                                   │
│    - Initializes JetStream                              │
│    - Listens on port 4222 (clients)                     │
│    - Listens on port 8222 (monitoring)                  │
│    - Ready for connections                              │
└─────────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────────┐
│ 2. PostgreSQL Starts                                    │
│    - Database server initializes                        │
│    - Connection pool ready                              │
│    - Database: armoricore_realtime                      │
└─────────────────────────────────────────────────────────┘
```

### Phase 2: Rust Services Start

#### Media Processor Startup

```
1. Binary executes: /opt/armoricore/rust-services/target/release/media-processor
   ↓
2. Initialize logging (tracing)
   ↓
3. Load configuration from environment variables
   ↓
4. Initialize key store (optional, for object storage credentials)
   ↓
5. Connect to NATS message bus
   - Creates NATS client
   - Connects to MESSAGE_BUS_URL
   - Subscribes to stream: armoricore-events
   ↓
6. Initialize object storage client (S3-compatible)
   - Uses key store or environment variables
   ↓
7. Create MediaWorker instance
   ↓
8. Start event processing loop
   - Subscribes to "media.uploaded" events
   - Ready to process media files
   ↓
9. Service running and waiting for events
```

#### Notification Worker Startup

```
1. Binary executes: /opt/armoricore/rust-services/target/release/notification-worker
   ↓
2. Initialize logging (tracing)
   ↓
3. Load configuration from environment variables
   ↓
4. Connect to NATS message bus
   - Creates NATS client
   - Connects to MESSAGE_BUS_URL
   - Subscribes to stream: armoricore-events
   ↓
5. Initialize device token database (optional)
   - Connects to PostgreSQL if DATABASE_URL is set
   - Runs migrations (creates device_tokens table)
   - Falls back to event payloads if database unavailable
   ↓
6. Initialize key store (optional, for FCM/APNS/SMTP credentials)
   ↓
7. Create NotificationSender
   - Loads FCM, APNS, SMTP credentials
   - Configures retry logic
   - Configures rate limiter
   - Sets up dead letter queue
   ↓
8. Create NotificationWorker instance
   ↓
9. Start event processing loop
   - Subscribes to "notification.requested" events
   - Ready to send notifications
   ↓
10. Service running and waiting for events
```

### Phase 3: Elixir Phoenix Starts

```
1. Binary executes: /opt/armoricore/elixir_realtime/bin/armoricore_realtime start
   OR: mix phx.server (development)
   ↓
2. OTP Application starts
   ↓
3. Supervisor tree initializes (one_for_one strategy)
   ↓
4. Children start in order:
   
   a) ArmoricoreRealtimeWeb.Telemetry
      - Sets up telemetry events
      - Configures metrics collection
   
   b) DNSCluster (optional)
      - Cluster discovery for multi-node setup
   
   c) ArmoricoreRealtime.Repo
      - Connects to PostgreSQL
      - Connection pool initialized
      - Database migrations run (if needed)
   
   d) Phoenix.PubSub
      - PubSub server started
      - Name: ArmoricoreRealtime.PubSub
      - Handles real-time message broadcasting
   
   e) ArmoricoreRealtime.KeyManager
      - GenServer started
      - Loads encrypted keys from priv/keys
      - Manages JWT secrets, API keys, etc.
   
   f) ArmoricoreRealtime.MessageBus
      - Connects to NATS
      - Subscribes to events:
        * media.ready
        * notification.sent
        * transcription.complete
      - Broadcasts events to Phoenix PubSub
   
   g) ArmoricoreRealtimeWeb.Endpoint
      - HTTP server starts (Bandit adapter)
      - Listens on port 4000
      - WebSocket endpoint ready
      - Routes configured
   ↓
5. Application fully started
   - All services connected
   - Ready to accept connections
```

---

## Runtime Operation

### Event-Driven Communication Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    NATS Message Bus                         │
│              (Central Communication Hub)                    │
└───────────────┬─────────────────────────────────────────────┘
                │
    ┌───────────┼───────────┐
    │           │           │
    ▼           ▼           ▼
┌─────────┐ ┌─────────┐ ┌─────────┐
│  Media  │ │Notification│ Elixir │
│Processor│ │  Worker   │ Phoenix │
└─────────┘ └─────────┘ └─────────┘
```

### Media Processing Flow

```
External System
    │
    │ Publishes "media.uploaded" event
    ▼
┌─────────────────────────────────────┐
│ NATS Message Bus                    │
│ Topic: media.uploaded               │
└──────────────┬──────────────────────┘
               │
               │ Event consumed
               ▼
┌─────────────────────────────────────┐
│ Media Processor (Rust)              │
│                                     │
│ 1. Receives event                   │
│ 2. Extracts media URL               │
│ 3. Downloads file (S3/HTTP)         │
│ 4. Processes with FFmpeg:           │
│    - Transcodes to multiple bitrates│
│    - Generates HLS segments         │
│    - Creates thumbnails             │
│    - Extracts metadata              │
│ 5. Uploads to object storage        │
│ 6. Publishes "media.ready" event    │
└──────────────┬──────────────────────┘
               │
               │ Publishes "media.ready"
               ▼
┌─────────────────────────────────────┐
│ NATS Message Bus                    │
│ Topic: media.ready                  │
└──────────────┬──────────────────────┘
               │
               │ Event consumed
               ▼
┌─────────────────────────────────────┐
│ Elixir Phoenix                      │
│                                     │
│ 1. MessageBus receives event        │
│ 2. Broadcasts to Phoenix PubSub     │
│ 3. Channels receive broadcast       │
│ 4. WebSocket clients notified       │
└─────────────────────────────────────┘
```

### Notification Flow

```
External System
    │
    │ Publishes "notification.requested" event
    ▼
┌─────────────────────────────────────┐
│ NATS Message Bus                    │
│ Topic: notification.requested       │
└──────────────┬──────────────────────┘
               │
               │ Event consumed
               ▼
┌─────────────────────────────────────┐
│ Notification Worker (Rust)          │
│                                     │
│ 1. Receives event                   │
│ 2. Retrieves device tokens:         │
│    - From database (if available)   │
│    - Or from event payload          │
│ 3. Applies rate limiting            │
│ 4. Sends notification:              │
│    - FCM (Android)                  │
│    - APNS (iOS)                     │
│    - SMTP (Email)                   │
│ 5. Retry logic (if failed)          │
│ 6. Publishes "notification.sent"    │
└──────────────┬──────────────────────┘
               │
               │ Publishes "notification.sent"
               ▼
┌─────────────────────────────────────┐
│ NATS Message Bus                    │
│ Topic: notification.sent            │
└──────────────┬──────────────────────┘
               │
               │ Event consumed
               ▼
┌─────────────────────────────────────┐
│ Elixir Phoenix                      │
│                                     │
│ 1. MessageBus receives event        │
│ 2. May broadcast to clients         │
└─────────────────────────────────────┘
```

### Real-Time Communication Flow

```
WebSocket Client
    │
    │ Connects to ws://server:4000/socket
    ▼
┌─────────────────────────────────────┐
│ Elixir Phoenix Endpoint             │
│                                     │
│ 1. Validates JWT token              │
│ 2. Creates socket connection        │
│ 3. Assigns user_id                  │
└──────────────┬──────────────────────┘
               │
               │ User joins channel
               ▼
┌─────────────────────────────────────┐
│ Phoenix Channel (Chat/Presence)     │
│                                     │
│ 1. User joins room                  │
│ 2. Presence tracked                 │
│ 3. Ready to receive messages        │
└──────────────┬──────────────────────┘
               │
               │ Message sent
               ▼
┌─────────────────────────────────────┐
│ Phoenix PubSub                      │
│                                     │
│ 1. Broadcasts to all subscribers    │
│ 2. All connected clients receive    │
└──────────────┬──────────────────────┘
               │
               │ (Optional) Publish to NATS
               ▼
┌─────────────────────────────────────┐
│ NATS Message Bus                    │
│ Topic: chat.message                 │
│                                     │
│ Other services can react to events  │
└─────────────────────────────────────┘
```

---

## Service Dependencies

### Dependency Graph

```
NATS Server
    │
    ├──► Media Processor (requires NATS)
    │
    ├──► Notification Worker (requires NATS)
    │
    └──► Elixir Phoenix (requires NATS)

PostgreSQL
    │
    ├──► Notification Worker (optional, for device tokens)
    │
    └──► Elixir Phoenix (required, for users, audit, analytics)

FFmpeg
    │
    └──► Media Processor (required, for media processing)
```

### Startup Order

1. **NATS Server** - Must start first (core dependency)
2. **PostgreSQL** - Must start before services that use it
3. **Rust Services** - Can start in parallel after NATS
4. **Elixir Phoenix** - Can start after NATS and PostgreSQL

---

## Health Checks

### Service Health Status

Each service exposes health information:

- **NATS**: `http://localhost:8222/healthz`
- **PostgreSQL**: `psql -U armoricore -d armoricore_realtime -c "SELECT 1;"`
- **Elixir Phoenix**: `GET http://localhost:4000/health`
- **Rust Services**: Check systemd status

### Monitoring Commands

```bash
# Check all services
sudo systemctl status nats postgresql armoricore-*

# Check NATS
nats server check

# Check database
psql -U armoricore -d armoricore_realtime -c "SELECT version();"

# Check Elixir Phoenix
curl http://localhost:4000/health

# View logs
sudo journalctl -u armoricore-* -f
```

---

## Error Handling & Recovery

### Service Failures

- **Systemd**: Automatically restarts failed services
- **Elixir OTP**: Supervisor trees restart crashed processes
- **Rust Services**: Graceful shutdown on errors, systemd restarts

### Connection Failures

- **NATS**: Services retry connection with exponential backoff
- **PostgreSQL**: Connection pool handles reconnections
- **Object Storage**: Retry logic in Rust services

### Event Processing Failures

- **Dead Letter Queue**: Failed events stored for analysis
- **Retry Logic**: Exponential backoff for transient errors
- **Rate Limiting**: Prevents overwhelming external services

---

## Resource Usage

### Typical Resource Consumption

- **NATS**: ~50MB RAM, minimal CPU
- **PostgreSQL**: ~200MB RAM, varies with data
- **Media Processor**: ~100-500MB RAM, high CPU during processing
- **Notification Worker**: ~50-100MB RAM, low CPU
- **Elixir Phoenix**: ~200-500MB RAM, moderate CPU

### Scaling Considerations

- **Rust Workers**: Scale horizontally (multiple instances)
- **Elixir Phoenix**: Scale horizontally (multiple nodes)
- **NATS**: Supports clustering for HA
- **PostgreSQL**: Use read replicas for scaling

---

## Summary

Armoricore operates as a **distributed, event-driven system**:

1. **Infrastructure** (NATS, PostgreSQL) starts first
2. **Rust Services** connect to NATS and process events
3. **Elixir Phoenix** connects to both NATS and PostgreSQL, handles real-time
4. **All services** communicate via NATS message bus
5. **Systemd** manages service lifecycle (start, stop, restart)
6. **OTP Supervision** handles Elixir process failures
7. **Event-driven** architecture enables loose coupling and scalability

The system is designed for **horizontal scaling**, with each service able to run multiple instances independently.

