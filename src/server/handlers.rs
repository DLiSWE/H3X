// src/server/handlers.rs
use quinn::SendStream;
use bincode::decode_from_slice;
use bincode::config::standard;

use crate::protocol::payloads::{EventsBatchPayload, FetchEventsPayload};
use crate::state::queue::{EventQueue};
use crate::protocol::frame::H3XFrame;
use crate::protocol::types::FrameType;
use crate::protocol::payloads::{EventPayload, AckEventPayload, AuthPayload};
use crate::state::registry::{ClientMetadata, NamespaceRegistry};
use crate::utils::{deserialize_payload, serialize_payload};
use crate::client::send::ack_event;


pub async fn handle_auth(frame: H3XFrame, registry: NamespaceRegistry) {
    let auth: AuthPayload = deserialize_payload(&frame.payload);
    println!("🔐 AUTH: client_id={}, token={}, namespace={}", auth.client_id, auth.token, auth.namespace);

    registry.write().await.insert(auth.namespace.clone(), ClientMetadata {
        client_id: auth.client_id,
        token: auth.token,
    });

    println!("✅ Authenticated client in namespace: {}", auth.namespace);
}

pub async fn handle_ack_event(frame: H3XFrame, sled_db: &sled::Db) {
    let payload: AckEventPayload = deserialize_payload(&frame.payload);
    let tree = match sled_db.open_tree(&payload.namespace) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("❌ Failed to open sled tree for ack: {e}");
            return;
        }
    };

    let key = payload.event_id.as_bytes();
    match tree.remove(key) {
        Ok(_) => println!("✅ Acked and deleted event {}", payload.event_id),
        Err(e) => eprintln!("❌ Failed to delete acked event: {e}"),
    }
}

pub async fn handle_event(
    mut frame: H3XFrame,
    registry: NamespaceRegistry,
    queue: EventQueue,
) {
    let event: EventPayload = deserialize_payload(&frame.payload);
    let ns = event.namespace.clone();

    if let Some(_) = registry.read().await.get(&ns) {
        println!("📨 [{}] EVENT: {}", ns, event.message);

        if let Err(e) = queue.enqueue(&mut frame) {
            eprintln!("❌ Failed to persist event to queue: {e}");
        }
        // Use `metadata` later for dispatching, alerting, etc.
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
    let batch: EventsBatchPayload = deserialize_payload(&frame.payload);

    for event in batch.events {
        let ns = event.namespace.clone();
        if let Some(_) = registry.read().await.get(&ns) {
            println!("📦 BATCH EVENT [{}]: {}", ns, event.r#type);

            // Serialize the event back into a frame and enqueue it
            let mut frame = H3XFrame {
                stream_id: frame.stream_id,
                frame_type: FrameType::Event,
                payload: serialize_payload(&event),
            };

            if let Err(e) = queue.enqueue(&mut frame) {
                eprintln!("❌ Failed to enqueue event for {}: {}", ns, e);
                continue;
            }

            // ✅ Acknowledge the event
            if let Err(e) = ack_event(frame.stream_id, ns.clone(), event.id, send).await {
                eprintln!("❌ Failed to ack event {}: {}", event.id, e);
            }

        } else {
            eprintln!("❌ Unknown namespace in batch: {}", ns);
        }
    }
}

pub async fn handle_ping(frame: H3XFrame, send: &mut SendStream) {
    println!("🔄 Received PING");
    let pong = H3XFrame {
        stream_id: frame.stream_id,
        frame_type: FrameType::Pong,
        payload: b"PONG".to_vec(),
    };
    if let Err(e) = pong.write_to(send).await {
        eprintln!("❌ Failed to send PONG: {e}");
    }
}

pub async fn handle_fetch_events(frame: H3XFrame, send: &mut SendStream, sled_db: &sled::Db) {
    let payload: FetchEventsPayload = deserialize_payload(&frame.payload);
    let ns_tree = sled_db.open_tree(payload.namespace).unwrap();
    
    let mut events = vec![];
    
    for result in ns_tree.iter().take(payload.max) {
        if let Ok((_, value)) = result {
            if let Ok((evt, _)) = decode_from_slice::<EventPayload, _>(&value, standard()) {
                events.push(evt);
            }
        } else {
            eprintln!("❌ Failed to decode event from sled: {:?}", result);
        }
    }
    
    let batch = EventsBatchPayload { events };
    let response = H3XFrame {
        stream_id: frame.stream_id,
        frame_type: FrameType::EventsBatch,
        payload: serialize_payload(&batch),
    };
    
    if let Err(e) = response.write_to(send).await {
        eprintln!("❌ Failed to send EventsBatch: {e}");
    }
}

pub async fn handle_frame(
    frame: H3XFrame,
    send: &mut SendStream,
    registry: NamespaceRegistry,
    queue: EventQueue,
) {
    match frame.frame_type {
        FrameType::Ping => handle_ping(frame, send).await,
        FrameType::Auth => handle_auth(frame, registry).await,
        FrameType::Event => handle_event(frame, registry, queue).await,
        FrameType::FetchEvent => handle_fetch_events(frame, send, &queue.db).await,
        FrameType::EventsBatch => handle_events_batch(frame, send, registry, queue).await,
        FrameType::AckEvent => handle_ack_event(frame, &queue.db).await,
        _ => eprintln!("❌ Unsupported frame type: {:?}", frame.frame_type),
    }
}
