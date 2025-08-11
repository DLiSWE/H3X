// src/server/handlers.rs
use quinn::{RecvStream, SendStream};

use std::convert::TryFrom;

use prost::Message;

use crate::state::queue::EventQueue;
use crate::state::registry::NamespaceRegistry;
use crate::utils::validate_auth;

use crate::protocol::h3x::{
    frame, // oneof namespace
    AckEvent,
    Event,
    EventsBatch,
    FetchEvents,
    Frame as H3XFrame,
    FrameType,
    Ping,
    Pong,
};

const PROTO_VERSION: u32 = 1;

// --- Small helpers ----------------------------------------------------------

async fn write_frame(send: &mut SendStream, frame: &H3XFrame) -> Result<(), std::io::Error> {
    frame
        .write_to(send)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

fn decode_stored_frame(bytes: &[u8]) -> Option<H3XFrame> {
    // If you stored frames with `encode_length_delimited`, use the corresponding decode.
    // If you stored raw `encode`, `decode` is fine as well. Try both if you’re migrating.
    H3XFrame::decode(bytes)
        .ok()
        .or_else(|| H3XFrame::decode_length_delimited(bytes).ok())
}

// --- Handlers ---------------------------------------------------------------

pub async fn handle_auth(
    frame: H3XFrame,
    registry: NamespaceRegistry,
    send: &mut SendStream,
) {
    let Some(frame::Payload::Auth(auth)) = frame.payload else {
        eprintln!("❌ Auth frame missing payload");
        return;
    };

    let is_valid = validate_auth(&auth, &registry).await;

    if is_valid {
        println!("🔐 Authenticated client_id={} namespaces={:?}", auth.client_id, auth.namespaces);

        // Send AuthAck (no payload needed)
        let ack = H3XFrame {
            version: PROTO_VERSION,
            stream_id: frame.stream_id,
            r#type: FrameType::AuthAck as i32,
            payload: None,
        };

        if let Err(e) = write_frame(send, &ack).await {
            eprintln!("❌ Failed to send AuthAck: {e}");
        }
    } else {
        eprintln!("❌ Invalid auth for client_id={}", auth.client_id);

        // Send AuthError (or Nack) and close stream
        let nack = H3XFrame {
            version: PROTO_VERSION,
            stream_id: frame.stream_id,
            r#type: FrameType::AuthError as i32,
            payload: None,
        };
        if let Err(e) = write_frame(send, &nack).await {
            eprintln!("❌ Failed to send AuthError: {e}");
        }
        if let Err(e) = send.finish().await {
            eprintln!("❌ Failed to close stream after AuthError: {e}");
        }
    }
}

pub async fn handle_ack_event(frame: H3XFrame, sled_db: &sled::Db) {
    let Some(frame::Payload::AckEvent(ack)) = frame.payload else {
        eprintln!("❌ AckEvent frame missing payload");
        return;
    };

    let tree = match sled_db.open_tree(&ack.namespace) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("❌ Failed to open sled tree for ack: {e}");
            return;
        }
    };

    let key = ack.event_id.as_bytes();
    match tree.remove(key) {
        Ok(_) => println!("✅ Acked and deleted event {}", ack.event_id),
        Err(e) => eprintln!("❌ Failed to delete acked event: {e}"),
    }
}

pub async fn handle_event(
    frame: H3XFrame,
    registry: NamespaceRegistry,
    queue: EventQueue,
) {
    let Some(frame::Payload::Event(event)) = frame.payload else {
        eprintln!("❌ Event frame missing payload");
        return;
    };

    let ns = event.namespace.clone();

    if registry.read().await.contains_key(&ns) {
        println!("📨 [{}] EVENT: {}", ns, event.message);

        // Re‑wrap into a prost Frame so your queue can persist the full envelope
        let mut stored = H3XFrame {
            version: PROTO_VERSION,
            stream_id: frame.stream_id,
            r#type: FrameType::Event as i32,
            payload: Some(frame::Payload::Event(event)),
        };

        if let Err(e) = queue.enqueue(&mut stored) {
            eprintln!("❌ Failed to persist event to queue: {e}");
        }
    } else {
        eprintln!("❌ Unknown namespace: {}", ns);
    }
}

pub async fn handle_events_batch(
    frame: H3XFrame,
    send: &mut SendStream,
    registry: NamespaceRegistry,
    queue: EventQueue,
) {
    let Some(frame::Payload::EventsBatch(EventsBatch { events })) = frame.payload else {
        eprintln!("❌ EventsBatch frame missing payload");
        return;
    };

    for ev in events {
        let ns = ev.namespace.clone();

        if registry.read().await.contains_key(&ns) {
            println!("📦 BATCH EVENT [{}]: {}", ns, ev.r#type);

            // Persist each as its own Event frame
            let mut store_frame = H3XFrame {
                version: PROTO_VERSION,
                stream_id: frame.stream_id,
                r#type: FrameType::Event as i32,
                payload: Some(frame::Payload::Event(ev.clone())),
            };

            if let Err(e) = queue.enqueue(&mut store_frame) {
                eprintln!("❌ Failed to enqueue event for {}: {}", ns, e);
                continue;
            }

            // Acknowledge this event back to the client
            let ack = H3XFrame {
                version: PROTO_VERSION,
                stream_id: frame.stream_id,
                r#type: FrameType::AckEvent as i32,
                payload: Some(frame::Payload::AckEvent(AckEvent {
                    namespace: ns.clone(),
                    event_id: ev.id.clone(),
                })),
            };

            if let Err(e) = write_frame(send, &ack).await {
                eprintln!("❌ Failed to send AckEvent {}: {}", ev.id, e);
            }
        } else {
            eprintln!("❌ Unknown namespace in batch: {}", ns);
        }
    }
}

pub async fn handle_ping(frame: H3XFrame, send: &mut SendStream) {
    println!("🔄 Received PING");

    // If Ping message is present, echo fields; otherwise send a payload‑less Pong
    let pong_payload = match frame.payload {
        Some(frame::Payload::Ping(Ping { timestamp_ms, seq })) => {
            Some(frame::Payload::Pong(Pong {
                echo_timestamp_ms: timestamp_ms,
                server_time_ms: crate::utils::now_ms(),
                seq,
            }))
        }
        _ => None,
    };

    let pong = H3XFrame {
        version: PROTO_VERSION,
        stream_id: frame.stream_id,
        r#type: FrameType::Pong as i32,
        payload: pong_payload,
    };

    if let Err(e) = write_frame(send, &pong).await {
        eprintln!("❌ Failed to send PONG: {e}");
    }
}

pub async fn handle_fetch_events(
    frame: H3XFrame,
    send: &mut SendStream,
    recv: &mut RecvStream,
    sled_db: &sled::Db,
) {
    let Some(frame::Payload::FetchEvents(FetchEvents { namespace, max_events })) = frame.payload else {
        eprintln!("❌ FetchEvents frame missing payload");
        return;
    };

    let prefix = format!("{}:", namespace);
    let mut events: Vec<Event> = Vec::new();

    // 1) Scan persisted frames and collect Events up to max_events
    for result in sled_db.scan_prefix(prefix.as_bytes()).take(max_events as usize) {
        match result {
            Ok((_key, value)) => {
                if let Some(stored_frame) = decode_stored_frame(&value) {
                    // Ensure it's an Event and extract it
                    let Ok(ft) = FrameType::try_from(stored_frame.r#type) else {
                        eprintln!("⚠️ Unknown stored frame type: {}", stored_frame.r#type);
                        continue;
                    };
                    if ft != FrameType::Event {
                        eprintln!("⚠️ Skipped non-Event stored frame: {:?}", ft);
                        continue;
                    }
                    match stored_frame.payload {
                        Some(frame::Payload::Event(ev)) => {
                            events.push(ev);
                        }
                        _ => eprintln!("⚠️ Stored Event frame missing payload"),
                    }
                } else {
                    eprintln!("❌ Failed to decode stored frame (not prost?)");
                }
            }
            Err(e) => {
                eprintln!("❌ Sled DB scan error: {:?}", e);
            }
        }
    }

    // 2) Send EventsBatch to client
    let response = H3XFrame {
        version: PROTO_VERSION,
        stream_id: frame.stream_id,
        r#type: FrameType::EventsBatch as i32,
        payload: Some(frame::Payload::EventsBatch(EventsBatch { events })),
    };

    println!(
        "🚚 Sending EventsBatch with {} event(s)",
        match &response.payload {
            Some(frame::Payload::EventsBatch(b)) => b.events.len(),
            _ => 0,
        }
    );

    if let Err(e) = write_frame(send, &response).await {
        eprintln!("❌ Failed to send EventsBatch response: {e}");
        return;
    } else {
        println!("✅ EventsBatch response sent.");
    }

    // 3) Wait for acks and delete from sled
    println!("📨 Waiting for AckEvent frames...");

    loop {
        match H3XFrame::read_from(recv).await {
            Ok(Some(ack_frame)) => {
                let Ok(ft) = FrameType::try_from(ack_frame.r#type) else {
                    eprintln!("⚠️ Unknown frame type value: {}", ack_frame.r#type);
                    continue;
                };

                if ft == FrameType::AckEvent {
                    let Some(frame::Payload::AckEvent(AckEvent { namespace, event_id })) = ack_frame.payload else {
                        eprintln!("⚠️ AckEvent missing payload");
                        continue;
                    };

                    println!("✅ Received Ack for event {} in namespace {}", event_id, namespace);

                    let key = format!("{}:{}", namespace, event_id);
                    match sled_db.remove(key.as_bytes()) {
                        Ok(Some(_)) => println!("🧹 Removed acknowledged event from sled: {}", key),
                        Ok(None) => println!("⚠️ Event not found in sled: {}", key),
                        Err(e) => eprintln!("❌ Failed to remove event from sled: {:?}", e),
                    }
                } else {
                    eprintln!("⚠️ Unexpected frame after EventsBatch: {:?}", ft);
                }
            }
            Ok(None) => {
                println!("📴 Client closed stream after sending Acks.");
                break;
            }
            Err(e) => {
                eprintln!("❌ Error reading from stream: {:?}", e);
                break;
            }
        }
    }
}

pub async fn handle_frame(
    frame: H3XFrame,
    send: &mut SendStream,
    recv: &mut RecvStream,
    registry: NamespaceRegistry,
    queue: EventQueue,
) -> Result<(), String> {
    let ft = FrameType::try_from(frame.r#type).map_err(|_| "bad frame type")?;

    match ft {
        FrameType::Ping => handle_ping(frame, send).await,
        FrameType::Auth => handle_auth(frame, registry, send).await,
        FrameType::Event => handle_event(frame, registry, queue).await,
        FrameType::FetchEvents => handle_fetch_events(frame, send, recv, &queue.db).await,
        FrameType::EventsBatch => handle_events_batch(frame, send, registry, queue).await,
        FrameType::AckEvent => handle_ack_event(frame, &queue.db).await,
        FrameType::Ack => println!("✅ ACK received on stream {}", frame.stream_id),
        other => eprintln!("❌ Unsupported frame type: {:?}", other),
    }

    Ok(())
}
