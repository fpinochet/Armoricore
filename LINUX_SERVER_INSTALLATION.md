# Linux Server Installation & Startup Guide

## Overview

This document provides comprehensive instructions for installing and running Armoricore on a Linux server. The system consists of:

- **Rust Services**: Media processor, notification worker, AI workers
- **Elixir Phoenix**: Real-time WebSocket server
- **NATS JetStream**: Message bus
- **PostgreSQL**: Database
- **FFmpeg**: Media processing

---

## System Requirements

### Minimum Requirements

- **OS**: Ubuntu 20.04+ / Debian 11+ / CentOS 8+ / RHEL 8+
- **CPU**: 4 cores (8+ recommended)
- **RAM**: 8GB (16GB+ recommended)
- **Storage**: 50GB+ (SSD recommended)
- **Network**: Stable internet connection

### Recommended Production Setup

- **OS**: Ubuntu 22.04 LTS
- **CPU**: 8+ cores
- **RAM**: 32GB+
- **Storage**: 200GB+ SSD
- **Network**: 1Gbps+

---

## Prerequisites Installation

### 1. System Updates

```bash
sudo apt update && sudo apt upgrade -y
```

### 2. Install Build Tools

```bash
sudo apt install -y \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    ca-certificates \
    gnupg \
    lsb-release
```

### 3. Install Rust

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### 4. Install Elixir & Erlang/OTP

```bash
# Install Erlang/OTP 25+
sudo apt install -y erlang-base erlang-dev erlang-parsetools

# Install Elixir 1.15+
sudo apt install -y elixir

# Or use asdf version manager (recommended)
git clone https://github.com/asdf-vm/asdf.git ~/.asdf --branch v0.14.0
echo '. "$HOME/.asdf/asdf.sh"' >> ~/.bashrc
echo '. "$HOME/.asdf/completions/asdf.bash"' >> ~/.bashrc
source ~/.bashrc

# Install Erlang
asdf plugin add erlang
asdf install erlang 26.0
asdf global erlang 26.0

# Install Elixir
asdf plugin add elixir
asdf install elixir 1.16.0
asdf global elixir 1.16.0

# Verify
elixir --version
```

### 5. Install PostgreSQL

```bash
# Install PostgreSQL 14+
sudo apt install -y postgresql postgresql-contrib

# Start and enable PostgreSQL
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Create database and user
sudo -u postgres psql << EOF
CREATE DATABASE armoricore_realtime;
CREATE USER armoricore WITH PASSWORD 'your-secure-password';
GRANT ALL PRIVILEGES ON DATABASE armoricore_realtime TO armoricore;
\q
EOF
```

### 6. Install NATS Server

```bash
# Download NATS server
cd /tmp
wget https://github.com/nats-io/nats-server/releases/download/v2.10.7/nats-server-v2.10.7-linux-amd64.zip
unzip nats-server-v2.10.7-linux-amd64.zip
sudo mv nats-server-v2.10.7-linux-amd64/nats-server /usr/local/bin/
sudo chmod +x /usr/local/bin/nats-server

# Verify
nats-server --version
```

### 7. Install FFmpeg

```bash
# Install FFmpeg
sudo apt install -y ffmpeg

# Verify
ffmpeg -version
```

---

## Application Installation

### 1. Clone Repository

```bash
cd /opt
sudo git clone <repository-url> armoricore
sudo chown -R $USER:$USER /opt/armoricore
cd /opt/armoricore
```

### 2. Build Rust Services

```bash
cd /opt/armoricore/rust-services

# Build all services in release mode
cargo build --release

# Verify binaries are created
ls -lh target/release/
# Should see: media-processor, notification-worker, etc.
```

### 3. Setup Elixir Phoenix

```bash
cd /opt/armoricore/elixir_realtime

# Install dependencies
mix deps.get

# Compile
mix compile

# Create database and run migrations
mix ecto.create
mix ecto.migrate

# Build production release (optional, for releases)
MIX_ENV=prod mix release
```

---

## Configuration

### 1. Environment Variables

Create environment files for each service:

#### Rust Services (`/opt/armoricore/rust-services/.env`)

```bash
# Message Bus
MESSAGE_BUS_URL=nats://localhost:4222
MESSAGE_BUS_STREAM_NAME=armoricore-events

# Object Storage (Akamai S3-compatible)
OBJECT_STORAGE_ENDPOINT=https://your-bucket.akamai.com
OBJECT_STORAGE_ACCESS_KEY=your-access-key
OBJECT_STORAGE_SECRET_KEY=your-secret-key
OBJECT_STORAGE_BUCKET=your-bucket-name
OBJECT_STORAGE_REGION=akamai

# Media Processing
AUDIO_CODEC=aac
LOG_LEVEL=info

# Notification Worker
FCM_SERVER_KEY=your-fcm-key
APNS_KEY_ID=your-apns-key-id
APNS_TEAM_ID=your-team-id
APNS_BUNDLE_ID=your-bundle-id
APNS_KEY_PATH=/opt/armoricore/keys/AuthKey.p8
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=your-email
SMTP_PASSWORD=your-password

# Database (for device tokens)
DATABASE_URL=postgresql://armoricore:your-password@localhost:5432/armoricore_realtime
```

#### Elixir Phoenix (`/opt/armoricore/elixir_realtime/.env`)

```bash
# Message Bus
MESSAGE_BUS_URL=nats://localhost:4222

# Database
DATABASE_URL=postgresql://armoricore:your-password@localhost:5432/armoricore_realtime

# JWT
JWT_SECRET=your-jwt-secret-key

# Phoenix
SECRET_KEY_BASE=your-secret-key-base
PHX_HOST=realtime.example.com
PORT=4000

# Environment
MIX_ENV=prod
```

### 2. Key Management

```bash
# Create keys directory
mkdir -p /opt/armoricore/keys

# Store encryption keys securely
# Keys will be managed by armoricore-keys service
```

---

## Systemd Service Files

### 1. NATS Server Service

Create `/etc/systemd/system/nats.service`:

```ini
[Unit]
Description=NATS Server
After=network.target

[Service]
Type=simple
User=nats
Group=nats
ExecStart=/usr/local/bin/nats-server -js -m 8222
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### 2. Media Processor Service

Create `/etc/systemd/system/armoricore-media-processor.service`:

```ini
[Unit]
Description=Armoricore Media Processor
After=network.target nats.service
Requires=nats.service

[Service]
Type=simple
User=armoricore
Group=armoricore
WorkingDirectory=/opt/armoricore/rust-services
EnvironmentFile=/opt/armoricore/rust-services/.env
ExecStart=/opt/armoricore/rust-services/target/release/media-processor
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### 3. Notification Worker Service

Create `/etc/systemd/system/armoricore-notification-worker.service`:

```ini
[Unit]
Description=Armoricore Notification Worker
After=network.target nats.service postgresql.service
Requires=nats.service postgresql.service

[Service]
Type=simple
User=armoricore
Group=armoricore
WorkingDirectory=/opt/armoricore/rust-services
EnvironmentFile=/opt/armoricore/rust-services/.env
ExecStart=/opt/armoricore/rust-services/target/release/notification-worker
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### 4. Elixir Phoenix Service

Create `/etc/systemd/system/armoricore-realtime.service`:

```ini
[Unit]
Description=Armoricore Realtime Server
After=network.target nats.service postgresql.service
Requires=nats.service postgresql.service

[Service]
Type=simple
User=armoricore
Group=armoricore
WorkingDirectory=/opt/armoricore/elixir_realtime
EnvironmentFile=/opt/armoricore/elixir_realtime/.env
ExecStart=/opt/armoricore/elixir_realtime/bin/armoricore_realtime start
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### 5. Create Service User

```bash
sudo useradd -r -s /bin/false armoricore
sudo chown -R armoricore:armoricore /opt/armoricore
```

---

## Startup Sequence

### 1. Enable and Start Services

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable services to start on boot
sudo systemctl enable nats
sudo systemctl enable postgresql
sudo systemctl enable armoricore-media-processor
sudo systemctl enable armoricore-notification-worker
sudo systemctl enable armoricore-realtime

# Start services in order
sudo systemctl start nats
sudo systemctl start postgresql
sleep 5  # Wait for database to be ready
sudo systemctl start armoricore-media-processor
sudo systemctl start armoricore-notification-worker
sudo systemctl start armoricore-realtime
```

### 2. Verify Services

```bash
# Check service status
sudo systemctl status nats
sudo systemctl status postgresql
sudo systemctl status armoricore-media-processor
sudo systemctl status armoricore-notification-worker
sudo systemctl status armoricore-realtime

# Check logs
sudo journalctl -u nats -f
sudo journalctl -u armoricore-media-processor -f
sudo journalctl -u armoricore-notification-worker -f
sudo journalctl -u armoricore-realtime -f
```

---

## How the System Works

### Startup Process

1. **NATS Server Starts**
   - Listens on port 4222 (client connections)
   - Listens on port 8222 (monitoring)
   - Initializes JetStream for persistent messaging

2. **PostgreSQL Starts**
   - Database server initializes
   - Connection pool ready

3. **Rust Services Start**
   - **Media Processor**:
     - Connects to NATS
     - Subscribes to `media.uploaded` events
     - Initializes object storage client
     - Ready to process media files
   
   - **Notification Worker**:
     - Connects to NATS
     - Subscribes to `notification.requested` events
     - Connects to PostgreSQL (optional, for device tokens)
     - Initializes FCM, APNS, SMTP clients
     - Ready to send notifications

4. **Elixir Phoenix Starts**
   - Connects to PostgreSQL
   - Runs database migrations
   - Connects to NATS
   - Starts KeyManager GenServer
   - Starts MessageBus subscriber
   - Starts Phoenix PubSub
   - Starts WebSocket endpoint (port 4000)
   - Application ready

### Runtime Operation

#### Media Processing Flow

```
1. External system publishes "media.uploaded" event to NATS
   ↓
2. Media Processor consumes event
   ↓
3. Downloads media file from S3/HTTP
   ↓
4. Processes with FFmpeg:
   - Transcodes to multiple bitrates
   - Generates HLS segments
   - Creates thumbnails
   ↓
5. Uploads processed files to object storage
   ↓
6. Publishes "media.ready" event to NATS
   ↓
7. Elixir Phoenix receives event
   ↓
8. Broadcasts to connected WebSocket clients
```

#### Notification Flow

```
1. External system publishes "notification.requested" event
   ↓
2. Notification Worker consumes event
   ↓
3. Retrieves device tokens from database (or event payload)
   ↓
4. Sends push notification (FCM/APNS) or email (SMTP)
   ↓
5. Publishes "notification.sent" event
   ↓
6. Elixir Phoenix may broadcast status to clients
```

#### Real-Time Communication Flow

```
1. Client connects via WebSocket to Elixir Phoenix
   ↓
2. JWT token validated
   ↓
3. User joins Phoenix Channel (chat, presence, etc.)
   ↓
4. Messages broadcast via Phoenix PubSub
   ↓
5. Events published to NATS for cross-service communication
   ↓
6. Other services can react to real-time events
```

### Service Dependencies

```
┌─────────────┐
│   NATS      │ (Message Bus - Core Dependency)
└──────┬──────┘
       │
       ├──► Media Processor
       ├──► Notification Worker
       └──► Elixir Phoenix

┌─────────────┐
│ PostgreSQL  │ (Database)
└──────┬──────┘
       │
       ├──► Notification Worker (device tokens)
       └──► Elixir Phoenix (users, audit, analytics)

┌─────────────┐
│   FFmpeg    │ (Media Processing)
└──────┬──────┘
       │
       └──► Media Processor
```

---

## Health Checks

### Service Health Endpoints

- **Elixir Phoenix**: `GET http://localhost:4000/health`
- **NATS**: `GET http://localhost:8222/healthz`

### Manual Health Checks

```bash
# Check NATS
nats server check

# Check PostgreSQL
sudo -u postgres psql -c "SELECT version();"

# Check Rust services
sudo systemctl status armoricore-media-processor
sudo systemctl status armoricore-notification-worker

# Check Elixir Phoenix
curl http://localhost:4000/health
```

---

## Monitoring & Logging

### View Logs

```bash
# All services
sudo journalctl -u armoricore-* -f

# Specific service
sudo journalctl -u armoricore-media-processor -f --since "1 hour ago"

# NATS logs
sudo journalctl -u nats -f
```

### Log Locations

- **Systemd logs**: `journalctl`
- **Application logs**: Structured JSON logs (can be forwarded to log aggregation)

---

## Troubleshooting

### Service Won't Start

1. **Check logs**:
   ```bash
   sudo journalctl -u <service-name> -n 50
   ```

2. **Check dependencies**:
   ```bash
   sudo systemctl status nats
   sudo systemctl status postgresql
   ```

3. **Check permissions**:
   ```bash
   ls -la /opt/armoricore
   sudo chown -R armoricore:armoricore /opt/armoricore
   ```

4. **Check environment variables**:
   ```bash
   sudo systemctl show armoricore-media-processor | grep EnvironmentFile
   ```

### Connection Issues

1. **NATS connection**:
   ```bash
   nats server ping
   ```

2. **PostgreSQL connection**:
   ```bash
   psql -U armoricore -d armoricore_realtime -h localhost
   ```

3. **Port conflicts**:
   ```bash
   sudo netstat -tulpn | grep -E '4222|4000|5432'
   ```

---

## Production Considerations

### 1. Security

- Use strong passwords for database
- Store secrets in environment variables or secret management
- Enable firewall (UFW):
  ```bash
  sudo ufw allow 22/tcp
  sudo ufw allow 4000/tcp
  sudo ufw enable
  ```

### 2. Performance

- Use production builds: `cargo build --release`
- Use Elixir releases: `MIX_ENV=prod mix release`
- Configure connection pooling
- Use reverse proxy (Nginx) for Elixir Phoenix

### 3. High Availability

- Run multiple instances behind load balancer
- Use NATS clustering
- Use PostgreSQL replication
- Monitor service health

### 4. Backup

- Regular database backups
- Key management backups
- Configuration backups

---

## Summary

The Armoricore system runs as a distributed set of services:

1. **NATS** - Message bus (core communication)
2. **PostgreSQL** - Database (users, tokens, audit, analytics)
3. **Rust Services** - Media processing, notifications
4. **Elixir Phoenix** - Real-time WebSocket server

All services are managed via systemd, start automatically on boot, and restart on failure. The system is event-driven, with services communicating via NATS message bus.

