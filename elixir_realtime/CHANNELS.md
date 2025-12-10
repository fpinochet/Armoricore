# Phoenix Channels - WebSocket Real-time Features

This document describes the WebSocket channels implemented for real-time communication.

## Overview

The application implements four main channel types:
1. **Chat Channel** - Real-time messaging
2. **Comments Channel** - Live streaming comments
3. **Presence Channel** - User presence tracking
4. **Signaling Channel** - WebRTC voice/video call signaling

## Authentication

All channels require JWT authentication. The client must provide a valid JWT token when connecting:

```javascript
const socket = new Socket("/socket", {
  params: { token: "your-jwt-token" }
});
```

The JWT must contain a `user_id` claim and be signed with the configured secret.

## Chat Channel

**Topic Pattern:** `chat:room:{room_id}`

### Joining

```javascript
const channel = socket.channel("chat:room:123", {});
channel.join()
  .receive("ok", resp => console.log("Joined", resp))
  .receive("error", resp => console.log("Failed", resp));
```

### Sending Messages

```javascript
channel.push("new_message", { content: "Hello, world!" })
  .receive("ok", msg => console.log("Message sent", msg))
  .receive("error", err => console.log("Error", err));
```

### Receiving Messages

```javascript
channel.on("new_message", msg => {
  console.log("New message:", msg);
});
```

### Message Format

```json
{
  "id": "uuid",
  "content": "Message text",
  "user_id": "user-uuid",
  "room_id": "room-id",
  "timestamp": "2025-01-01T00:00:00Z"
}
```

### Typing Indicators

The chat channel supports typing indicators to show when users are typing.

#### Starting Typing

```javascript
channel.push("typing_start", {})
  .receive("ok", resp => console.log("Typing started", resp));
```

#### Stopping Typing

```javascript
channel.push("typing_stop", {})
  .receive("ok", resp => console.log("Typing stopped", resp));
```

#### Receiving Typing Updates

```javascript
channel.on("user_typing", payload => {
  if (payload.is_typing) {
    console.log(`User ${payload.user_id} is typing...`);
  } else {
    console.log(`User ${payload.user_id} stopped typing`);
  }
});
```

**Typing Indicator Format:**

```json
{
  "user_id": "user-uuid",
  "room_id": "room-id",
  "is_typing": true
}
```

**Auto-Stop:** Typing indicators automatically stop after 3 seconds of inactivity if the client doesn't send `typing_stop`.

## Comments Channel

**Topic Pattern:** `comments:stream:{stream_id}`

### Joining

```javascript
const channel = socket.channel("comments:stream:456", {});
channel.join();
```

### Sending Comments

```javascript
channel.push("new_comment", { 
  content: "Great stream!",
  timestamp: Date.now() 
})
  .receive("ok", comment => console.log("Comment sent", comment))
  .receive("error", err => {
    if (err.reason === "rate_limit") {
      console.log("Too many comments, please slow down");
    }
  });
```

### Rate Limiting

Comments are rate-limited to 1 comment per second per user to prevent spam.

### Comment Format

```json
{
  "id": "uuid",
  "content": "Comment text",
  "user_id": "user-uuid",
  "stream_id": "stream-id",
  "timestamp": 1234567890
}
```

## Presence Channel

**Topic Pattern:** `presence:room:{room_id}`

### Joining

```javascript
const channel = socket.channel("presence:room:789", {
  user_id: "your-user-id",
  status: "online"
});
channel.join();
```

### Receiving Presence Updates

```javascript
// Initial presence state
channel.on("presence_state", presences => {
  console.log("Current users:", presences);
});

// Presence changes (user joined/left/updated)
channel.on("presence_diff", diff => {
  console.log("Presence changed:", diff);
});
```

### Updating Status

```javascript
channel.push("update_status", { status: "away" })
  .receive("ok", resp => console.log("Status updated", resp));
```

### Presence Format

```json
{
  "user-id-1": {
    "metas": [
      {
        "online_at": 1234567890,
        "status": "online"
      }
    ]
  }
}
```

## Signaling Channel

**Topic Pattern:** `signaling:call:{call_id}`

### Joining

```javascript
const callId = "call-uuid-123";
const signalingChannel = socket.channel(`signaling:call:${callId}`, {
  caller_id: "user-123",
  callee_id: "user-456"
});
signalingChannel.join();
```

### Initiating Call

```javascript
signalingChannel.push("call_initiate", {
  callee_id: "user-456",
  call_type: "video"  // or "voice"
});
```

### SDP Offer/Answer

```javascript
// Send offer
signalingChannel.push("call_offer", {
  sdp: offer.sdp,
  type: "offer"
});

// Receive answer
signalingChannel.on("call_answer", ({ sdp, type }) => {
  // Handle SDP answer
});
```

### ICE Candidates

```javascript
// Send ICE candidate
signalingChannel.push("ice_candidate", {
  candidate: candidate.candidate,
  sdp_mid: candidate.sdpMid,
  sdp_m_line_index: candidate.sdpMLineIndex
});

// Receive ICE candidate
signalingChannel.on("ice_candidate", ({ candidate, sdp_mid, sdp_m_line_index }) => {
  // Handle ICE candidate
});
```

### Ending Call

```javascript
signalingChannel.push("call_end", {
  reason: "user_hangup"
});
```

For WebRTC signaling implementation details, see the `SignalingChannel` module in `lib/armoricore_realtime_web/channels/signaling_channel.ex`.

## WebSocket Connection

### JavaScript Client Example

```javascript
import { Socket } from "phoenix";

const socket = new Socket("/socket", {
  params: { token: "your-jwt-token" }
});

socket.connect();

// Join channels
const chatChannel = socket.channel("chat:room:123", {});
chatChannel.join();

const commentsChannel = socket.channel("comments:stream:456", {});
commentsChannel.join();

const presenceChannel = socket.channel("presence:room:789", {
  user_id: "user-123",
  status: "online"
});
presenceChannel.join();

const signalingChannel = socket.channel("signaling:call:call-123", {
  caller_id: "user-123",
  callee_id: "user-456"
});
signalingChannel.join();
```

### Connection Events

```javascript
socket.onOpen(() => console.log("Connected"));
socket.onClose(() => console.log("Disconnected"));
socket.onError(() => console.log("Error"));
```

## Message Bus Integration

Channels publish events to the NATS message bus for:
- Moderation workflows
- Analytics
- Cross-service communication

Events are published via `ArmoricoreRealtimeWeb.ChannelHelpers`.

## Security Considerations

1. **JWT Validation**: All connections require valid JWT tokens
2. **User Verification**: Presence channel verifies user_id matches socket
3. **Rate Limiting**: Comments are rate-limited to prevent spam
4. **Room Authorization**: In production, implement room access checks

## Testing

### Manual Testing with Phoenix Client

```elixir
# In IEx console
alias ArmoricoreRealtimeWeb.UserSocket
alias Phoenix.Socket

# Create a test JWT (use your JWT module)
token = "your-test-jwt-token"

# Connect socket
{:ok, socket} = Phoenix.Socket.connect(UserSocket, %{"token" => token}, %{})
```

### Testing Channels

```elixir
# Join a channel
{:ok, _, socket} = Phoenix.Socket.join(socket, "chat:room:123")

# Send a message
ref = Phoenix.Channel.push(socket, "new_message", %{content: "Hello"})

# Receive response
receive do
  {^ref, {:ok, message}} -> IO.inspect(message)
end
```

## Production Considerations

1. **Scaling**: Use Redis PubSub adapter for multi-node deployments
2. **Monitoring**: Track channel joins/leaves and message rates
3. **Rate Limiting**: Implement stricter rate limits per user/IP
4. **Message Persistence**: Consider storing messages in database
5. **Moderation**: Integrate with moderation service via message bus

