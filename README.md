# Armoricore

**High-Performance Backend Platform for Media Processing and Real-Time Communication**

Armoricore is a distributed backend system that leverages **Rust** and **Elixir** to deliver scalable, efficient services for media processing, real-time communication, notifications, and AI workflows.

---

## 🎯 Overview

Armoricore combines the best of both worlds:

- **Rust** → CPU-intensive, performance-critical tasks (media transcoding, notifications, AI processing)
- **Elixir/Phoenix** → High-concurrency real-time WebSocket connections (chat, live comments, presence)

These components communicate asynchronously via a distributed message bus, enabling independent scaling and deployment.

---

## 🏗️ Architecture

### Component Responsibilities

#### Rust Services (Armoricore Core)

| Service | Responsibility |
|---------|---------------|
| **Media Processor** | Consumes media upload events, runs FFmpeg workflows for transcoding/segmentation, generates thumbnails, uploads to object storage |
| **Notification Worker** | Consumes notification events, sends push notifications and emails asynchronously |
| **AI/Captioning Workers** | Performs transcription and AI-based tasks (optional) |
| **Message Bus Client** | Publishes and consumes events from distributed message bus (NATS JetStream/RabbitMQ) |

#### Elixir Phoenix Realtime Server

| Feature | Responsibility |
|---------|---------------|
| **WebSocket Management** | Manages persistent WebSocket connections for chat, live comments, and presence tracking |
| **Authentication** | Validates JWT tokens issued by PHP backend |
| **PubSub Broadcasting** | Broadcasts messages and presence state changes using Phoenix PubSub |
| **Message Bus Integration** | Subscribes to message bus for cross-service communication |

---

## 🔄 Communication Flow

### Event-Driven Architecture

Both Rust and Elixir services connect to a **shared message bus** (NATS JetStream or RabbitMQ):

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   PHP API   │────────▶│ Message Bus  │◀────────│   Elixir    │
│  (Backend)  │         │ (NATS/Rabbit)│         │  (Phoenix)  │
└─────────────┘         └──────────────┘         └─────────────┘
                              ▲
                              │
                        ┌─────┴─────┐
                        │   Rust    │
                        │ Services  │
                        └───────────┘
```

### Event Types

- **Rust → Elixir**: `media.ready`, `notification.sent`, `transcription.complete`
- **Elixir → Rust**: `chat.message`, `presence.update` (for moderation workflows)
- **PHP → Rust/Elixir**: `media.uploaded`, `notification.requested`

---

## 📊 Data Flow Examples

### Media Upload to Playback

```
1. User uploads video
   ↓
2. PHP backend issues presigned URL
   ↓
3. Upload completes → PHP publishes `media.uploaded` event
   ↓
4. Rust Media Processor consumes event
   ↓
5. Transcodes video, generates thumbnails, uploads to object storage
   ↓
6. Rust publishes `media.ready` event
   ↓
7. Elixir subscribes to `media.ready` → notifies connected users via WebSocket
   ↓
8. Akamai CDN serves processed video segments
```

### Notification Flow

```
1. Event triggers notification request
   ↓
2. `notification.requested` event published to message bus
   ↓
3. Rust Notification Worker consumes event
   ↓
4. Sends push notification or email asynchronously
   ↓
5. Rust publishes `notification.sent` event
   ↓
6. Elixir may broadcast notification status to realtime clients
```

---

## 🔐 Authentication Flow

1. **PHP Backend** issues JWT tokens on user login
2. **Elixir Phoenix** validates JWT on WebSocket connection for realtime authentication
3. **Rust Services** receive JWT or event metadata for authorization in workflows

---

## 🚀 Getting Started

### Prerequisites

- **Rust** (latest stable version)
- **Elixir** (1.14+) and **Erlang/OTP** (25+)
- **Phoenix Framework** (1.7+)
- **NATS Server** or **RabbitMQ** (for message bus)
- **FFmpeg** (for media processing)
- **Docker** (optional, for containerized deployment)

### Installation

#### 1. Clone the Repository

```bash
git clone <repository-url>
cd Armoricore
```

#### 2. Setup Rust Services

```bash
cd rust-services
cargo build --release
```

#### 3. Setup Elixir Phoenix Server

```bash
cd elixir-realtime
mix deps.get
mix compile
```

#### 4. Configure Message Bus

Set up NATS JetStream or RabbitMQ and configure connection strings in:
- `rust-services/.env`
- `elixir-realtime/config/dev.exs` (or `prod.exs`)

#### 5. Run Services

**Rust Services:**
```bash
cd rust-services
cargo run --bin media-processor
cargo run --bin notification-worker
```

**Elixir Phoenix:**
```bash
cd elixir-realtime
mix phx.server
```

---

## 📁 Project Structure

```
Armoricore/
├── README.md
├── ARCHITECTURE.md
├── rust-services/
│   ├── Cargo.toml
│   ├── media-processor/
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── notification-worker/
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── ai-workers/
│   │   ├── src/
│   │   └── Cargo.toml
│   └── message-bus-client/
│       ├── src/
│       └── Cargo.toml
├── elixir-realtime/
│   ├── mix.exs
│   ├── config/
│   ├── lib/
│   │   ├── armoricore_realtime/
│   │   │   ├── application.ex
│   │   │   ├── endpoint.ex
│   │   │   └── channels/
│   │   └── armoricore_realtime_web/
│   └── priv/
└── docs/
    ├── deployment.md
    ├── api.md
    └── development.md
```

---

## 🧪 Development

### Running Tests

**Rust:**
```bash
cd rust-services
cargo test
```

**Elixir:**
```bash
cd elixir-realtime
mix test
```

### Local Development Setup

1. Start NATS server: `nats-server -js`
2. Start Rust services in separate terminals
3. Start Elixir Phoenix: `mix phx.server`
4. Connect WebSocket clients to `ws://localhost:4000/socket`

---

## 📈 Scalability

### Horizontal Scaling

- **Rust Workers**: Scale independently based on queue depth
- **Elixir Nodes**: Add Phoenix nodes behind a load balancer; Phoenix PubSub handles distribution
- **Message Bus**: NATS JetStream supports clustering for high availability

### Monitoring

- **Rust**: Metrics via Prometheus exporters
- **Elixir**: Phoenix LiveDashboard for real-time metrics
- **Message Bus**: NATS monitoring endpoints or RabbitMQ management UI

---

## 🔧 Configuration

### Environment Variables

**Rust Services:**
```bash
MESSAGE_BUS_URL=nats://localhost:4222
OBJECT_STORAGE_ENDPOINT=s3://...
OBJECT_STORAGE_ACCESS_KEY=...
OBJECT_STORAGE_SECRET_KEY=...
```

**Elixir:**
```elixir
# config/prod.exs
config :armoricore_realtime, ArmoricoreRealtime.Endpoint,
  http: [port: 4000],
  url: [host: "realtime.example.com"]

config :armoricore_realtime, :message_bus,
  url: "nats://nats-server:4222"
```

---

## 🛡️ Security

- **JWT Validation**: All WebSocket connections require valid JWT tokens
- **Event Authorization**: Rust services validate event metadata
- **TLS/SSL**: All external communications use TLS
- **Secrets Management**: Use environment variables or secret management services

---

## 📚 Documentation

- [Architecture Plan](./ARCHITECTURE.md) - Detailed system architecture
- [API Documentation](./docs/api.md) - API endpoints and WebSocket protocols
- [Deployment Guide](./docs/deployment.md) - Production deployment instructions
- [Development Guide](./docs/development.md) - Development setup and workflows

---

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## 📄 License

Copyright 2025 Francisco F. Pinochet

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

---

## 🙏 Acknowledgments

- Rust community for excellent FFmpeg and async libraries
- Phoenix Framework team for robust real-time capabilities
- NATS and RabbitMQ communities for reliable message bus solutions

---

## 📞 Support

For questions and support, please open an issue or contact the developer. 

