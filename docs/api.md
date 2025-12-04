# API Documentation

## WebSocket API

### Connection

Connect to WebSocket endpoint:
```
wss://realtime.example.com/socket/websocket?token=<JWT_TOKEN>
```

### Channels

#### Chat Channel

**Join:**
```json
{
  "topic": "chat:room:123",
  "event": "phx_join",
  "payload": {},
  "ref": "1"
}
```

**Send Message:**
```json
{
  "topic": "chat:room:123",
  "event": "new_message",
  "payload": {
    "content": "Hello, world!",
    "user_id": "user-123"
  },
  "ref": "2"
}
```

**Receive Message:**
```json
{
  "topic": "chat:room:123",
  "event": "new_message",
  "payload": {
    "id": "msg-456",
    "content": "Hello, world!",
    "user_id": "user-123",
    "timestamp": "2024-01-01T00:00:00Z"
  },
  "ref": null
}
```

#### Live Comments Channel

**Join:**
```json
{
  "topic": "comments:stream:789",
  "event": "phx_join",
  "payload": {},
  "ref": "1"
}
```

**Send Comment:**
```json
{
  "topic": "comments:stream:789",
  "event": "new_comment",
  "payload": {
    "content": "Great stream!",
    "timestamp": 12345
  },
  "ref": "2"
}
```

#### Presence Channel

**Join:**
```json
{
  "topic": "presence:room:123",
  "event": "phx_join",
  "payload": {
    "user_id": "user-123",
    "status": "online"
  },
  "ref": "1"
}
```

**Presence Update:**
```json
{
  "topic": "presence:room:123",
  "event": "presence_diff",
  "payload": {
    "joins": {
      "user-456": {
        "metas": [{"status": "online"}]
      }
    },
    "leaves": {
      "user-123": {
        "metas": [{"status": "offline"}]
      }
    }
  },
  "ref": null
}
```

## Message Bus Events

### Published Events

#### `media.uploaded`
```json
{
  "event_type": "media.uploaded",
  "event_id": "uuid",
  "timestamp": "2024-01-01T00:00:00Z",
  "source": "php-backend",
  "payload": {
    "media_id": "uuid",
    "user_id": "uuid",
    "file_path": "s3://bucket/key",
    "content_type": "video/mp4",
    "file_size": 1048576,
    "metadata": {}
  }
}
```

#### `media.ready`
```json
{
  "event_type": "media.ready",
  "event_id": "uuid",
  "timestamp": "2024-01-01T00:00:00Z",
  "source": "rust-media-processor",
  "payload": {
    "media_id": "uuid",
    "playback_urls": {
      "hls": "https://cdn.example.com/media/123/playlist.m3u8",
      "dash": "https://cdn.example.com/media/123/manifest.mpd"
    },
    "thumbnail_urls": [
      "https://cdn.example.com/media/123/thumb_1.jpg"
    ],
    "duration": 3600,
    "resolutions": ["1080p", "720p", "480p"]
  }
}
```

#### `notification.requested`
```json
{
  "event_type": "notification.requested",
  "event_id": "uuid",
  "timestamp": "2024-01-01T00:00:00Z",
  "source": "php-backend",
  "payload": {
    "user_id": "uuid",
    "type": "push",
    "title": "New message",
    "body": "You have a new message",
    "data": {
      "message_id": "uuid"
    }
  }
}
```

### Consumed Events

Services consume events from the message bus based on their subscriptions.

