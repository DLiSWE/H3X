// src/server/handlers.rs
use quinn::SendStream;

use crate::protocol::frame::H3XFrame;
use crate::protocol::types::FrameType;
use super::payloads::{AuthPayload, EventPayload};
use crate::utils::deserialize_payload;

pub async fn handle_auth(frame: H3XFrame) {
    let auth: AuthPayload = deserialize_payload(&frame.payload);
    println!("🔐 AUTH: client_id={}, token={}", auth.client_id, auth.token);
}

pub async fn handle_event(frame: H3XFrame) {
    let event: EventPayload = deserialize_payload(&frame.payload);
    println!("📨 EVENT [{}]: {}", event.r#type, event.message);
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

pub async fn handle_frame(frame: H3XFrame, send: &mut SendStream) {
    match frame.frame_type {
        FrameType::Ping => handle_ping(frame, send).await,
        FrameType::Auth => handle_auth(frame).await,
        FrameType::Event => handle_event(frame).await,
        _ => eprintln!("❌ Unsupported frame type: {:?}", frame.frame_type),
    }
}


