# Armoricore v0.9.0 - Quick Start Guide

## Prerequisites

Before starting, ensure you have:

1. **Rust** - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **Elixir** - `brew install elixir` (macOS) or use asdf
3. **FFmpeg** - `brew install ffmpeg` (macOS) or `apt install ffmpeg` (Linux)
4. **NATS Server** - `brew install nats-server` (macOS) or download from [nats.io](https://nats.io/download/)
5. **PostgreSQL** - `brew install postgresql` (macOS) or `apt install postgresql` (Linux)

## Quick Setup (5 minutes)

### 1. Clone and Navigate

```bash
git clone https://github.com/fpinochet/Armoricore.git
cd Armoricore
```

### 2. Configure Credentials

**⚠️ IMPORTANT:** You must configure database and object storage credentials before running.

**Quick Setup:**
```bash
# Set database URL (required for Elixir)
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/armoricore_realtime_dev"

# Set object storage credentials (required for media processing)
export OBJECT_STORAGE_ENDPOINT="https://your-bucket.akamai.com"
export OBJECT_STORAGE_ACCESS_KEY="your-access-key"
export OBJECT_STORAGE_SECRET_KEY="your-secret-key"
export OBJECT_STORAGE_BUCKET="your-bucket-name"
```

**Or create `.env` files:**
```bash
# Create .env files (gitignored)
cat > rust-services/.env << EOF
MESSAGE_BUS_URL=nats://localhost:4222
OBJECT_STORAGE_ENDPOINT=https://your-bucket.akamai.com
OBJECT_STORAGE_ACCESS_KEY=your-access-key
OBJECT_STORAGE_SECRET_KEY=your-secret-key
OBJECT_STORAGE_BUCKET=your-bucket-name
EOF

cat > elixir_realtime/.env << EOF
MESSAGE_BUS_URL=nats://localhost:4222
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/armoricore_realtime_dev
EOF
```

**For detailed configuration instructions, see [CONFIGURATION.md](./CONFIGURATION.md).**

### 3. Setup Database (Elixir only)

```bash
cd elixir_realtime
mix deps.get
mix ecto.create
mix ecto.migrate
cd ..
```

### 4. Start All Services

```bash
# Automated startup (recommended)
./start_all.sh start

# Check status
./start_all.sh status
```

### 5. Verify Services

```bash
# Check NATS
curl http://localhost:8222/healthz

# Check Phoenix
curl http://localhost:4000/api/health

# View logs
tail -f logs/*.log
```

---

## Manual Startup (Alternative)

If you prefer manual control:

### Terminal 1: NATS Server
```bash
nats-server -js -p 4222 -m 8222
```

### Terminal 2: Media Processor
```bash
cd rust-services
cargo run --release --bin media-processor
```

### Terminal 3: Notification Worker
```bash
cd rust-services
cargo run --release --bin notification-worker
```

### Terminal 4: Elixir Phoenix
```bash
cd elixir_realtime
mix phx.server
```

---

## Testing

### Test Scripts

**End-to-End Test:**
```bash
# Test complete media upload workflow (user creation → processing → storage)
./test_end_to_end.sh
```

**System Health Check:**
```bash
# Comprehensive system health checks (services, endpoints, database)
./test_system.sh
```

**Create Test Data:**
```bash
# Create test users and media records in database
cd elixir_realtime
mix run create_test_data.exs
```

### Manual Testing

**Test WebSocket Connection:**
```bash
# Install wscat if needed: npm install -g wscat
wscat -c ws://localhost:4000/socket
```

**Test Event Publishing:**
```bash
# Install NATS CLI: brew install nats-io/nats-tools/nats

# Publish test notification event
nats pub 'armoricore.notification_requested' '{
  "event_type": "notification.requested",
  "payload": {
    "notification_type": "push",
    "title": "Test",
    "body": "Hello from Armoricore"
  }
}'
```

---

## Troubleshooting

### Services Won't Start

1. **Check prerequisites:**
   ```bash
   cargo --version
   mix --version
   ffmpeg -version
   nats-server --version
   ```

2. **Check ports:**
   ```bash
   lsof -i :4222  # NATS
   lsof -i :4000  # Phoenix
   lsof -i :5432  # PostgreSQL
   ```

3. **Check logs:**
   ```bash
   tail -f logs/*.log
   ```

### Common Issues

**"NATS connection failed"**
- Ensure NATS is running: `nats-server -js`
- Check `MESSAGE_BUS_URL` in `.env`

**"PostgreSQL connection failed"**
- Start PostgreSQL: `brew services start postgresql`
- Create database: `cd elixir_realtime && mix ecto.create`

**"FFmpeg not found"**
- Install FFmpeg: `brew install ffmpeg`
- Verify: `ffmpeg -version`

---

## Next Steps

- Read [README_STARTUP.md](./README_STARTUP.md) for detailed startup guide
- Follow [CHECKLIST.md](./CHECKLIST.md) for comprehensive testing
- Review [CODEBASE_STATE.md](./CODEBASE_STATE.md) for feature overview

---

## Support

For issues or questions:
- Check logs in `./logs/` directory
- Review [Troubleshooting](#troubleshooting) section
- Open an issue on GitHub

