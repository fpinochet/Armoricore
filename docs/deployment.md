# Deployment Guide

## Production Deployment

### Prerequisites

- Docker and Docker Compose (or Kubernetes cluster)
- NATS JetStream or RabbitMQ cluster
- Object storage (S3-compatible) access
- Domain name and SSL certificates
- Monitoring and logging infrastructure

### Deployment Options

#### Option 1: Docker Compose (Development/Staging)

```yaml
# docker-compose.yml
version: '3.8'

services:
  nats:
    image: nats:latest
    ports:
      - "4222:4222"
      - "8222:8222"
    command: ["-js", "-m", "8222"]

  rust-media-processor:
    build: ./rust-services
    command: cargo run --bin media-processor
    environment:
      - MESSAGE_BUS_URL=nats://nats:4222
      - OBJECT_STORAGE_ENDPOINT=${OBJECT_STORAGE_ENDPOINT}
    depends_on:
      - nats

  rust-notification-worker:
    build: ./rust-services
    command: cargo run --bin notification-worker
    environment:
      - MESSAGE_BUS_URL=nats://nats:4222
    depends_on:
      - nats

  elixir-realtime:
    build: ./elixir-realtime
    command: mix phx.server
    ports:
      - "4000:4000"
    environment:
      - MESSAGE_BUS_URL=nats://nats:4222
    depends_on:
      - nats
```

#### Option 2: Kubernetes (Production)

See Kubernetes manifests in `k8s/` directory for production deployment.

### Environment Configuration

Set the following environment variables:

**Rust Services:**
```bash
MESSAGE_BUS_URL=nats://nats-server:4222
OBJECT_STORAGE_ENDPOINT=s3://your-bucket
OBJECT_STORAGE_ACCESS_KEY=your-key
OBJECT_STORAGE_SECRET_KEY=your-secret
LOG_LEVEL=info
```

**Elixir Phoenix:**
```bash
MESSAGE_BUS_URL=nats://nats-server:4222
SECRET_KEY_BASE=your-secret-key-base
DATABASE_URL=postgresql://user:pass@db:5432/armoricore
```

### Health Checks

All services expose health check endpoints:
- Rust: `GET /health`
- Elixir: `GET /health`

### Scaling

Scale services independently:
```bash
# Scale Rust workers
kubectl scale deployment rust-media-processor --replicas=5

# Scale Elixir nodes
kubectl scale deployment elixir-realtime --replicas=3
```


