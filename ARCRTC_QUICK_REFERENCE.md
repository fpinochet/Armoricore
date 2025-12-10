# ArcRTC Quick Reference

## What is ArcRTC?

**ArcRTC** (Armoricore Real-Time Communication) is a high-performance, low-latency media transport protocol designed to rival WebRTC with superior performance for native applications.

## Key Features

- ✅ **Ultra-low latency** - < 50ms end-to-end (vs WebRTC's 100-200ms)
- ✅ **Simplified signaling** - ArcSignaling protocol (vs WebRTC's complex SDP/ICE)
- ✅ **Full control** - Packet-level optimization
- ✅ **Any codec** - Not limited to browser-supported codecs
- ✅ **Native-first** - Optimized for mobile/desktop apps
- ✅ **WebRTC compatible** - Hybrid mode for interoperability

## Protocol Stack

```
Application Layer
    ↓
ArcSignaling (Connection/Stream Management)
    ↓
ArcMedia (ArcRTP/ArcSRTP)
    ↓
Transport (UDP/TCP/QUIC)
    ↓
Network (IP)
```

## ArcSignaling Messages

| Message | Purpose |
|---------|---------|
| `CONNECT` | Initiate connection |
| `CONNECT_ACK` | Accept connection |
| `STREAM_START` | Start media stream |
| `STREAM_STOP` | Stop media stream |
| `QUALITY_ADAPT` | Change quality |
| `HEARTBEAT` | Keep-alive & latency |
| `HEARTBEAT_ACK` | Heartbeat response |

## ArcRTP Enhancements

- **Quality Indicator** - 2-bit field for rapid quality switching
- **Priority Field** - 2-bit field for packet prioritization
- **Reduced Header** - Optional minimal header (4 bytes)
- **Latency Metadata** - Built-in send/receive time tracking

## Performance Targets

- **Latency:** < 50ms end-to-end
- **Jitter Buffer:** 5-30ms adaptive
- **Packet Processing:** < 1ms per packet
- **Throughput:** 10 Mbps per stream, 1000+ concurrent

## Codec Support

**Audio:** Opus (preferred), AAC, G.722, PCM  
**Video:** VP9 (preferred), H.264, AV1, H.265

## Security

- **Encryption:** AES-128-GCM (default), AES-256-GCM (optional)
- **Key Exchange:** ECDH P-256
- **Authentication:** HMAC-SHA256
- **Replay Protection:** 64-bit sequence window
- **Key Rotation:** Every 24 hours or 2^31 packets

## Hybrid Mode

ArcRTC works alongside WebRTC:

- **Native apps** → Use ArcRTC (better performance)
- **Browser apps** → Use WebRTC (native support)
- **Protocol Bridge** → Server-side translation for interoperability

## Quick Start

### 1. Connect

```rust
let mut arcrtc = ArcRtcClient::new();
arcrtc.connect(server_url, session_id).await?;
```

### 2. Start Stream

```rust
let stream_id = arcrtc.start_stream(StreamConfig {
    audio: AudioConfig {
        codec: "opus",
        bitrate: 128000,
    },
    video: VideoConfig {
        codec: "vp9",
        bitrate: 2000000,
        resolution: Resolution::HD1080,
        fps: 30,
    },
}).await?;
```

### 3. Send Media

```rust
let packet = ArcRtpPacket::new(stream_id, payload);
arcrtc.send_packet(packet).await?;
```

### 4. Receive Media

```rust
while let Some(packet) = arcrtc.receive_packet().await? {
    process_media(packet.payload);
}
```

## Comparison: ArcRTC vs WebRTC

| Feature | WebRTC | ArcRTC |
|---------|--------|--------|
| **Latency** | 100-200ms | < 50ms |
| **Signaling** | SDP/ICE (complex) | ArcSignaling (simple) |
| **Platform** | Browser-first | Native-first |
| **Control** | Limited | Full |
| **Codecs** | Browser-supported | Any |
| **Overhead** | Browser stack | Minimal |

## Documentation

- **Full Spec:** `ARCRTC_PROTOCOL_SPECIFICATION.md`
- **Implementation Plan:** `REALTIME_MEDIA_ENGINE_PLAN.md`
- **WebRTC Comparison:** See `WEBRTC_VS_CUSTOM_ANALYSIS.md` for detailed comparison

---

**Version:** 1.0  
**Status:** Specification (Implementation in progress)

