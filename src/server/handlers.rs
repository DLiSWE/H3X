// src/server/handlers.rs
use quinn::SendStream;

use crate::protocol::frame::H3XFrame;
use crate::protocol::types::FrameType;
use crate::server::payloads::{AuthPayload, EventPayload};
use crate::server::state::{ClientMetadata, NamespaceRegistry};
use crate::utils::deserialize_payload;


pub async fn handle_auth(frame: H3XFrame, registry: NamespaceRegistry) {
    let auth: AuthPayload = deserialize_payload(&frame.payload);
    println!("🔐 AUTH: client_id={}, token={}, namespace={}", auth.client_id, auth.token, auth.namespace);

    registry.write().await.insert(auth.namespace.clone(), ClientMetadata {
        client_id: auth.client_id,
        token: auth.token,
    });
}

pub async fn handle_event(frame: H3XFrame, registry: NamespaceRegistry) {
    let event: EventPayload = deserialize_payload(&frame.payload);
    let ns = event.namespace.clone();

    if let Some(metadata) = registry.read().await.get(&ns) {
        println!("📨 [{}] EVENT: {}", ns, event.message);
        // TODO: idk we can do something with the metadata here
    } else {
        eprintln!("❌ Unknown namespace: {}", ns);
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

pub async fn handle_frame(
    frame: H3XFrame,
    send: &mut SendStream,
    registry: NamespaceRegistry,
) {
    match frame.frame_type {
        FrameType::Ping => handle_ping(frame, send).await,
        FrameType::Auth => handle_auth(frame, registry).await,
        FrameType::Event => handle_event(frame, registry).await,
        _ => eprintln!("❌ Unsupported frame type: {:?}", frame.frame_type),
    }
}


