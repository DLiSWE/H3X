# H3X — Reliable, Low-Latency Event Protocol over QUIC (Rust)

H3X is a lightweight QUIC-powered protocol for structured events with authentication, durable queues, and at-least-once delivery. Built with quinn, tokio, prost, and sled.

> Status: Alpha. Core loop (Auth → FetchEvents → EventsBatch → AckEvent) works. In progress: batching, reconnect/backoff, stream-ID routing, JS/TS SDK.

## Features
- QUIC streams (TLS by default), multiplexed I/O
- Length-prefixed Protobuf frames (versionable)
- sled-backed queue for durable replay
- Namespaced auth: `client:{namespace}` + token registry
- Explicit reliability via `AckEvent`
- Stream-ID routing scaffold (handlers per stream)

## Quick Start

### Prereqs
- Rust (stable), OpenSSL
- Windows: `rustup component add rust-src` (if IDE/builds complain)

## Protocol

### Frame format
+------------+-----------------+-------------------+
| frame_type | payload_length | payload_bytes... |
| u8 | u32 LE | [length] |
+------------+-----------------+-------------------+


### Core frames (Protobuf payloads)
- **Auth**: `{ client_id, token, namespaces[] }`
- **FetchEvents**: `{ namespaces[], limit? }`
- **EventsBatch**: `{ events[] }`
- **AckEvent**: `{ event_ids[] }`
- *(Planned)* SendEvent, RateLimitNotice, Ping, Pong

### Handshake
1. Client → **Auth**
2. Server → validate via `H3X_REGISTRY`
3. Client → **FetchEvents**
4. Server → **EventsBatch**
5. Client → **AckEvent** for delivered IDs

## Event Model (Protobuf)

message EventPayload {
  string id = 1;                   // UUID v4
  string namespace = 2;            // e.g., "env_namespace"
  string type = 3;                 // e.g., "Error"
  string message = 4;              // short description
  bytes  data = 5;                 // JSON or arbitrary bytes
  int64  timestamp = 6;            // Unix seconds (UTC)
  map<string, string> metadata = 7;// severity, service, env, ...
}

## Project Layout
.
├─ Cargo.toml
├─ build.rs              # prost-build (generates src/protocol/*)
├─ proto/                # .proto sources
├─ src/
│  ├─ server/            # QUIC server, handlers, queue, registry
│  ├─ client/            # run_client, builder, reconnect (WIP)
│  ├─ protocol/          # prost-generated Rust
│  ├─ state/             # server state (registry, queue)
│  └─ utils/             # framing (u8 + u32 LE + bytes)
├─ data/                 # sled database (runtime)
└─ .env.example


## Troubleshooting

**Auth fails / stream closes**
- Ensure `H3X_REGISTRY` has matching `client:{namespace}={token}`.

**Always 0 events**
- Check `H3X_DATA_DIR` and keys like `env_namespace:{uuid}` exist in sled.

**ApplicationClosed / BI stream error**
- Often benign if the client exits after acks during dev; keep client running.


---

### Chunk I — Roadmap & License
```markdown
## Roadmap
- [x] Auth, FetchEvents, EventsBatch, AckEvent
- [x] sled-backed queue
- [x] Stream-ID routing scaffold
- [ ] Client reconnect + backoff
- [ ] Server batching & per-namespace rate limiting
- [ ] JS/TS SDK
- [ ] CLI: `h3x inject`, `h3x tail`, `h3x bench`
- [ ] Metrics (Prometheus)

## License
MIT
