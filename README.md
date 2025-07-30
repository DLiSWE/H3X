# H3X — Custom Protocol over QUIC

**H3X** is a minimal, extensible protocol built on top of QUIC (via [`quinn`](https://github.com/quinn-rs/quinn)) for high-performance, low-latency, multiplexed messaging. It’s designed for projects where you need more control than HTTP but don’t want the full complexity of gRPC.

This is mostly an experimental playground to explore building structured, reliable communication over QUIC using Rust.

---

## 🔧 Features

- 🔄 **Framed messages** with type and payload (Ping, Auth, Event, etc.)
- ⚡ **Bi-directional streaming** via QUIC streams
- 🔐 **TLS encryption** with self-signed certs for local use
- 🧠 **Custom frame handlers** for logic like authentication and event routing
- 🗂️ **Namespace-aware auth** (e.g. for multi-tenant apps)
- 🔁 **Client auto-reconnect** with exponential backoff
- 🧼 **Graceful shutdown** on Ctrl+C

---

## 🧪 Example Use Case

You could use this as a lightweight foundation for:

- Internal observability pipelines
- Real-time telemetry or event ingestion
- A drop-in backend transport protocol for SDKs

---

## ▶️ Running Locally

### Start the server

```bash
cargo run -- server

```


### Start the client
```bash

cargo run -- client
```
