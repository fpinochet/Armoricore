# Armoricore

**Version:** v0.9.0 (Pre-Release Milestone)  
**Repository:** [https://github.com/fpinochet/Armoricore](https://github.com/fpinochet/Armoricore)

> **Note:** This version (v0.9.0) is published in the personal repository. Future versions (v1.0.0+) will be published in the Fastcomcorp organization repository.

**High-Performance Backend Platform for Media Processing and Real-Time Communication**

Armoricore is a distributed backend system that leverages **Rust** and **Elixir** to deliver scalable, efficient services for media processing (HLS, MP4, adaptive streaming, **audio-only streaming with FLAC**), real-time communication with **ArcRTC** (optimized for native applications requiring ultra-low latency) and **WebRTC** (browser support), notifications, and AI workflows.

---

## 🎯 Overview

Armoricore is a distributed backend platform designed for modern media and real-time communication applications. It provides a complete solution for video streaming, real-time communication, notifications, and AI-powered workflows.

### Key Capabilities

- **Media Processing**: Transcode videos up to 8K resolution with hardware acceleration, generate HLS/MP4 streams, support audio-only streaming with FLAC, and process media at scale
- **Real-Time Communication**: WebSocket-based chat, presence tracking, and voice/video calls with both WebRTC (browser) and ArcRTC (native) protocol support
- **AI Integration**: Speech-to-text transcription, multi-language translation, and auto-captioning powered by OpenAI Whisper, GPT, and Anthropic Claude
- **Notifications**: Push notifications (FCM/APNS) and email delivery with retry logic and dead-letter queues
- **Scalability**: Event-driven architecture with horizontal scaling, distributed processing, and independent service deployment

### Technical Approach

Armoricore uses a **hybrid architecture** that combines the strengths of two languages:

- **Rust** → Handles CPU-intensive, performance-critical tasks (media transcoding, notifications, AI processing) with zero-cost abstractions and memory safety
- **Elixir/Phoenix** → Manages high-concurrency real-time WebSocket connections (chat, live comments, presence) with millions of concurrent connections

These components communicate asynchronously via a distributed message bus (NATS JetStream), enabling independent scaling, fault tolerance, and flexible deployment strategies.

---

## 🏗️ Architecture

### Component Responsibilities

#### Rust Services (Armoricore Core)

| Service | Responsibility |
|---------|---------------|
| **Media Processor** | Consumes media upload events, runs FFmpeg workflows for transcoding/segmentation (up to **8K**), generates HLS and MP4 formats (video and **audio-only**), generates thumbnails, uploads to object storage. Supports **FLAC** and other audio formats for internet radio streaming. |
| **Notification Worker** | Consumes notification events, sends push notifications and emails asynchronously |
| **Realtime Media Engine** | Real-time media transport engine with RTP/SRTP, audio/video encoding, adaptive bitrate, packet loss concealment, and network adaptation. Supports **ArcRTC** protocol for native applications with ultra-low latency requirements and **WebRTC** for browser-based applications. |
| **Realtime Media Engine gRPC** | gRPC server providing Elixir/Phoenix integration for the realtime media engine |
| **AI Workers** | Performs speech-to-text transcription (OpenAI Whisper), translation (OpenAI GPT/Anthropic Claude), captioning, and moderation (optional) |
| **Message Bus Client** | Publishes and consumes events from distributed message bus (NATS JetStream/RabbitMQ) |

#### Elixir Phoenix Realtime Server

| Feature | Responsibility |
|---------|---------------|
| **WebSocket Management** | Manages persistent WebSocket connections for chat, live comments, and presence tracking |
| **Authentication** | Validates JWT tokens issued by PHP backend |
| **PubSub Broadcasting** | Broadcasts messages and presence state changes using Phoenix PubSub |
| **Message Bus Integration** | Subscribes to message bus for cross-service communication |
| **Media Processing Orchestration** | Task pools, priority queues, GenStage pipelines, and distributed processing for high-volume media workloads |
| **Database Layer** | PostgreSQL integration for users, media, analytics, and authentication |

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

- **Rust → Elixir**: `media.ready`, `notification.sent`, `transcription.complete`, `translation.complete`, `captioning.complete`
- **Elixir → Rust**: `chat.message`, `presence.update` (for moderation workflows)
- **PHP → Rust/Elixir**: `media.uploaded`, `notification.requested`, `transcription.requested`, `translation.requested`

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
5. Transcodes video to HLS and MP4 formats, generates thumbnails, uploads to object storage
   ↓
6. Rust publishes `media.ready` event with playback URLs (HLS, MP4, DASH-ready)
   ↓
7. Elixir subscribes to `media.ready` → notifies connected users via WebSocket
   ↓
8. Akamai CDN serves processed video (HLS segments, MP4 files, thumbnails)
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

### Speech-to-Text & Translation Flow

```
1. Media file uploaded or text content available
   ↓
2. `transcription.requested` or `translation.requested` event published
   ↓
3. Rust AI Workers consume event
   ↓
4. For transcription:
   - Downloads audio/video file (HTTP/HTTPS or local)
   - Extracts audio from video using FFmpeg (if needed)
   - Calls OpenAI Whisper API for transcription
   - Returns transcription with timestamp segments
   ↓
5. For translation:
   - Calls OpenAI GPT or Anthropic Claude API
   - Translates text from source to target language
   - Returns translated text
   ↓
6. Rust publishes `transcription.complete` or `translation.complete` event
   ↓
7. Elixir may broadcast results to realtime clients
   ↓
8. Caption files (SRT/VTT) can be generated from transcriptions
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
git clone https://github.com/fpinochet/Armoricore.git
cd Armoricore
```

> **Note:** Future versions (v1.0.0+) will be available in the Fastcomcorp organization repository.

**Quick Start:** See [QUICK_START.md](./QUICK_START.md) for a 5-minute setup guide.

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

**Option 1: Automated Startup (Recommended)**

Use the provided startup script to start all services:

```bash
./start_all.sh start
```

This will automatically:
- Check prerequisites
- Start NATS server
- Build and start all Rust services
- Start Elixir Phoenix server
- Verify all services are running

See [Startup Guide](./README_STARTUP.md) for details.

**Option 2: Manual Startup**

**Rust Services:**
```bash
cd rust-services
cargo build --release
cargo run --release --bin media-processor
cargo run --release --bin notification-worker
cargo run --release --bin ai-workers  # Optional: For speech-to-text and translation
```

**Elixir Phoenix:**
```bash
cd elixir_realtime
mix deps.get
mix ecto.migrate  # First time only
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
│   ├── ai-connectors/
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

### Test Scripts

**End-to-End Test:**
```bash
# Test complete media upload workflow
./test_end_to_end.sh
```

**System Health Check:**
```bash
# Comprehensive system health checks
./test_system.sh
```

**Create Test Data:**
```bash
# Create test users and media records in database
cd elixir_realtime
mix run create_test_data.exs
```

**Hardware Acceleration Test:**
```bash
# Test hardware encoding (macOS VideoToolbox)
cd rust-services/media-processor
./test_hardware.sh
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

### Media Processing Features

- **HLS Generation**: Adaptive streaming with multiple bitrates (360p to **8K** for video, 64k-320k for audio-only)
- **MP4 Generation**: Progressive download support for all resolution variants (video and audio-only)
- **Video Codecs**: H.264, VP9, AV1, and **VVC (H.266)** with automatic codec selection
- **Hardware Acceleration**: Automatic GPU acceleration (NVENC, VideoToolbox, VAAPI) for 4-8x faster encoding
- **Parallel Processing**: Concurrent multi-resolution encoding for 3-5x faster processing
- **Audio-Only Streaming**: Full support for internet radio streaming with FLAC, MP3, AAC, Opus, and Vorbis codecs
- **Multiple Audio Codecs**: AAC, Opus, MP3, Vorbis, FLAC support with resolution-based selection
- **Upload Retry Logic**: Exponential backoff retry for failed uploads with configurable delays
- **Thumbnail Generation**: Automatic thumbnail extraction (video only)
- **Object Storage**: S3-compatible storage (Akamai Object Storage) with retry logic
- **Elixir-Level Optimizations**: Task pools, priority queues, GenStage pipelines, and distributed processing for high-volume workloads

### AI & ML Features

- **Speech-to-Text**: OpenAI Whisper API integration for audio/video transcription
  - Automatic audio extraction from video files
  - Timestamp segments for caption generation
  - Language detection and specification
- **Translation**: Multi-language text translation
  - OpenAI GPT-4o-mini support
  - Anthropic Claude support
  - Auto-detect source language or specify manually
- **Auto-Captioning**: Generate captions from transcriptions
  - SRT/VTT format support (via captioning workflow)
  - Timestamp-aligned captions
- **Content Moderation**: AI-powered content moderation (optional)

### Monitoring

- **Rust**: Comprehensive structured logging (JSON for production, console for development)
- **Elixir**: Phoenix LiveDashboard for real-time metrics and system monitoring
- **Message Bus**: NATS monitoring endpoints or RabbitMQ management UI
- **Log Aggregation**: Logs can be sent to ELK, Loki, CloudWatch, or any log aggregation system

---

## 🔧 Configuration

### Required Credentials

**⚠️ IMPORTANT:** Never commit real credentials to version control!

All credentials should be configured via environment variables or `.env` files (which are gitignored).

**See [CONFIGURATION.md](./CONFIGURATION.md) for complete setup instructions.**

### Quick Configuration

**Rust Services (`rust-services/.env`):**
```bash
# Message Bus
MESSAGE_BUS_URL=nats://localhost:4222

# Object Storage (Akamai S3-compatible) - REQUIRED
OBJECT_STORAGE_ENDPOINT=https://your-bucket.akamai.com
OBJECT_STORAGE_ACCESS_KEY=your-akamai-access-key
OBJECT_STORAGE_SECRET_KEY=your-akamai-secret-key
OBJECT_STORAGE_BUCKET=your-bucket-name
OBJECT_STORAGE_REGION=akamai

# AI Services (Optional)
OPENAI_API_KEY=sk-...          # For Whisper transcription and GPT translation
OPENAI_ORGANIZATION=org-...    # Optional: OpenAI organization ID
ANTHROPIC_API_KEY=sk-ant-...   # Optional: For Claude translation
```

**Elixir Phoenix (`elixir_realtime/.env`):**
```bash
# Message Bus
MESSAGE_BUS_URL=nats://localhost:4222

# Database - REQUIRED
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/armoricore_realtime_dev

# JWT
JWT_SECRET=your-jwt-secret-key-change-in-production

# Phoenix
SECRET_KEY_BASE=your-secret-key-base-generate-with-mix-phx-gen-secret
PHX_HOST=localhost
PORT=4000
```

### Setup Instructions

1. **PostgreSQL Database:**
   ```bash
   # Create database
   createdb armoricore_realtime_dev
   
   # Run migrations
   cd elixir_realtime
   mix ecto.create
   mix ecto.migrate
   ```

2. **Object Storage:**
   - Create account in Akamai/Linode Object Storage
   - Create bucket
   - Generate access keys
   - Set `OBJECT_STORAGE_*` environment variables

3. **Generate Secrets:**
   ```bash
   # Generate Phoenix secret key base
   cd elixir_realtime
   mix phx.gen.secret
   # Copy output to SECRET_KEY_BASE
   ```

**For detailed configuration instructions, see [CONFIGURATION.md](./CONFIGURATION.md).**

---

## 🛡️ Security

- **JWT Validation**: All WebSocket connections require valid JWT tokens
- **Event Authorization**: Rust services validate event metadata
- **TLS/SSL**: All external communications use TLS
- **Secrets Management**: Use environment variables or secret management services

---

## 🎬 ArcRTC Protocol

Armoricore includes **ArcRTC** (Armoricore Real-Time Communication), a protocol engineered for specific use cases  where native applications require:

- **Ultra-low latency** - Optimized for sub-50ms end-to-end latency requirements (native mobile/desktop apps)
- **Full packet control** - Direct packet-level optimization for specialized applications
- **Advanced codec support** - Support for codecs not yet available in browsers (VVC/H.266, SCIP, etc.)
- **Simplified signaling** - ArcSignaling protocol designed for native-to-native communication

**When to use ArcRTC:**
- Native mobile or desktop applications requiring ultra-low latency
- Applications needing full control over packet processing and routing
- Use cases requiring advanced codecs not supported by browsers
- Specialized real-time communication scenarios

**When to use WebRTC:**
- Browser-based applications (Chrome, Firefox, Safari, Edge)
- Maximum compatibility across platforms
- Standard compliance and ecosystem integration
- Applications where WebRTC's 100-200ms latency is acceptable

**Hybrid Approach:**
Armoricore supports both protocols and includes a protocol bridge for interoperability, allowing you to choose the best protocol for each use case.

**ArcRTC Documentation:**
- [ArcRTC Protocol Specification](./ARCRTC_PROTOCOL_SPECIFICATION.md) - Complete protocol specification
- [ArcRTC Quick Reference](./ARCRTC_QUICK_REFERENCE.md) - Quick reference guide

---

## 📚 Documentation

- [Configuration Guide](./CONFIGURATION.md) - **Complete credential setup instructions** ⭐
- [Quick Start Guide](./QUICK_START.md) - Get started in 5 minutes
- [Startup Guide](./README_STARTUP.md) - How to start all services (automated and manual)
- [Pre-Release Checklist](./CHECKLIST.md) - Comprehensive testing checklist
- [Architecture Plan](./ARCHITECTURE.md) - Detailed system architecture
- [Codebase State](./CODEBASE_STATE.md) - Current implementation status and features
- [Linux Server Installation](./LINUX_SERVER_INSTALLATION.md) - Production deployment guide
- [Startup and Operation](./STARTUP_AND_OPERATION.md) - System operation details
- [API Documentation](./docs/api.md) - API endpoints and WebSocket protocols
- [Deployment Guide](./docs/deployment.md) - Production deployment instructions
- [Development Guide](./docs/development.md) - Development setup and workflows
- [Media Processor README](./rust-services/media-processor/README.md) - Media processing capabilities and configuration

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

