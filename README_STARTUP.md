# Armoricore Startup Guide

## Quick Start

### Option 1: Automated Startup Script (Recommended)

Use the provided startup script to start all services:

```bash
# Start all services
./start_all.sh start

# Check status
./start_all.sh status

# Stop all services
./start_all.sh stop

# Restart all services
./start_all.sh restart
```

The script will:
1. ✅ Check prerequisites (Rust, Elixir, FFmpeg, NATS, PostgreSQL)
2. ✅ Start NATS server (if not running)
3. ✅ Build Rust services
4. ✅ Start all Rust services (media-processor, notification-worker, ai-workers)
5. ✅ Start Elixir Phoenix server
6. ✅ Verify all services are running

**Logs:** All service logs are written to `./logs/` directory  
**PIDs:** Service PIDs are stored in `./pids/` directory

---

### Option 2: Manual Startup

If you prefer to start services manually:

#### 1. Start Infrastructure Services

**NATS Server:**
```bash
nats-server -js -p 4222 -m 8222
```

**PostgreSQL:**
```bash
# Start PostgreSQL (varies by system)
sudo systemctl start postgresql  # Linux
# or
brew services start postgresql   # macOS
```

#### 2. Start Rust Services

```bash
cd rust-services

# Build all services
cargo build --release

# Start services in separate terminals
cargo run --release --bin media-processor
cargo run --release --bin notification-worker
cargo run --release --bin ai-workers  # Optional
```

#### 3. Start Elixir Phoenix

```bash
cd elixir_realtime

# Install dependencies (first time only)
mix deps.get

# Run migrations (first time only)
mix ecto.migrate

# Start Phoenix server
mix phx.server
```

---

## Service Dependencies

### Required Services

1. **NATS Server** (port 4222)
   - Message bus for all services
   - Must start first

2. **PostgreSQL** (port 5432)
   - Required for Elixir Phoenix
   - Optional for notification-worker (device tokens)

3. **FFmpeg**
   - Required for media processing
   - Must be installed and in PATH

### Application Services

1. **Media Processor** (Rust)
   - Processes media uploads
   - Requires: NATS, FFmpeg, Object Storage

2. **Notification Worker** (Rust)
   - Sends push notifications and emails
   - Requires: NATS, FCM/APNS/SMTP credentials

3. **AI Workers** (Rust, Optional)
   - Speech-to-text, translation
   - Requires: NATS, OpenAI/Anthropic API keys

4. **Elixir Phoenix** (port 4000)
   - WebSocket server for real-time communication
   - Requires: NATS, PostgreSQL

---

## Verification

### Check Service Status

```bash
# Using the script
./start_all.sh status

# Or manually check ports
lsof -i :4222  # NATS
lsof -i :5432  # PostgreSQL
lsof -i :4000  # Phoenix
```

### Check Logs

```bash
# View all logs
tail -f logs/*.log

# View specific service log
tail -f logs/media-processor.log
tail -f logs/elixir-phoenix.log
```

### Test Services

**Test NATS:**
```bash
curl http://localhost:8222/healthz
```

**Test Phoenix:**
```bash
curl http://localhost:4000
```

**Test WebSocket:**
```bash
# Connect to WebSocket endpoint
wscat -c ws://localhost:4000/socket
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

2. **Check ports are available:**
   ```bash
   lsof -i :4222  # NATS
   lsof -i :4000  # Phoenix
   ```

3. **Check environment variables:**
   ```bash
   # Rust services
   cat rust-services/.env
   
   # Elixir services
   cat elixir_realtime/.env
   ```

### Common Issues

**NATS connection failed:**
- Ensure NATS server is running: `nats-server -js`
- Check `MESSAGE_BUS_URL` environment variable

**PostgreSQL connection failed:**
- Ensure PostgreSQL is running
- Check `DATABASE_URL` environment variable
- Run migrations: `cd elixir_realtime && mix ecto.migrate`

**Media processing fails:**
- Ensure FFmpeg is installed: `ffmpeg -version`
- Check object storage credentials

**Phoenix won't start:**
- Check database connection
- Run migrations: `mix ecto.migrate`
- Check for port conflicts: `lsof -i :4000`

---

## Production Deployment

For production deployment, see:
- [Linux Server Installation Guide](./LINUX_SERVER_INSTALLATION.md)
- [Startup and Operation Guide](./STARTUP_AND_OPERATION.md)

Production services should be managed via systemd or a process manager like Supervisor.

