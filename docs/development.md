# Development Guide

## Local Development Setup

### Prerequisites

- Rust (latest stable)
- Elixir 1.14+ and Erlang/OTP 25+
- Phoenix Framework 1.7+
- NATS Server or RabbitMQ
- FFmpeg (for media processing)
- Docker (optional)

### Initial Setup

1. **Clone Repository**
   ```bash
   git clone <repository-url>
   cd Armoricore
   ```

2. **Start Message Bus**
   ```bash
   # NATS
   nats-server -js
   
   # Or RabbitMQ
   docker run -d --name rabbitmq -p 5672:5672 -p 15672:15672 rabbitmq:3-management
   ```

3. **Setup Rust Services**
   ```bash
   cd rust-services
   cp .env.example .env
   # Edit .env with your configuration
   cargo build
   ```

4. **Setup Elixir Phoenix**
   ```bash
   cd elixir-realtime
   mix deps.get
   mix compile
   cp config/dev.exs.example config/dev.exs
   # Edit config/dev.exs with your configuration
   ```

### Running Services

**Terminal 1 - Rust Media Processor:**
```bash
cd rust-services
cargo run --bin media-processor
```

**Terminal 2 - Rust Notification Worker:**
```bash
cd rust-services
cargo run --bin notification-worker
```

**Terminal 3 - Elixir Phoenix:**
```bash
cd elixir-realtime
mix phx.server
```

### Testing

**Rust Tests:**
```bash
cd rust-services
cargo test
```

**Elixir Tests:**
```bash
cd elixir-realtime
mix test
```

### Development Workflow

1. Make changes to code
2. Run tests
3. Check linter/formatter
4. Test locally with message bus
5. Commit changes

### Debugging

**Rust:**
- Use `tracing` for structured logging
- Enable debug logs: `RUST_LOG=debug cargo run`

**Elixir:**
- Use `IO.inspect` for debugging
- Phoenix LiveDashboard: `http://localhost:4000/dashboard`
- IEx console: `iex -S mix phx.server`

### Code Style

**Rust:**
- Use `rustfmt` for formatting
- Use `clippy` for linting

**Elixir:**
- Use `mix format` for formatting
- Use `credo` for linting


