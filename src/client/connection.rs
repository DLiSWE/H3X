use anyhow::{bail, Result};
use quinn::Connection;
use crate::{client::event::handle_event_frame, protocol::h3x as pb};
use crate::protocol::h3x::Frame;
use std::convert::TryFrom;
use crate::protocol::h3x::{frame, FrameType};


const PROTO_VERSION: u32 = 1;

pub async fn authenticate(conn: &Connection, client_id: String, token: String, namespaces: Vec<String>) -> Result<()> {
    let auth = pb::Auth { client_id, token, namespaces };

    let (mut send, mut recv) = conn.open_bi().await?;
    let frame = pb::Frame {
        version: 1,
        stream_id: 0,
        r#type: pb::FrameType::Auth as i32,
        payload: Some(pb::frame::Payload::Auth(auth)),
    };
    frame.write_to(&mut send).await?;

    match Frame::read_from(&mut recv).await? {
        Some(reply) => match (pb::FrameType::try_from(reply.r#type), reply.payload) {
            (Ok(pb::FrameType::AuthAck),   _) => Ok(()),
            (Ok(pb::FrameType::AuthError), _) => bail!("❌ Auth rejected by server"),
            (Ok(other), payload) => bail!("❌ Unexpected frame during auth: {:?} {:?}", other, payload),
            (Err(bad), _) => bail!("❌ Unknown FrameType value: {}", bad),
        },
        None => bail!("❌ No response received after authentication"),
    }
}

pub async fn receive_loop(conn: &Connection, namespaces: Vec<String>) -> Result<()> {
    // Open a BI stream to request & receive events
    let (mut send, mut recv) = conn.open_bi().await?;

    // Ask server for events (you can set `limit` if you support it)
    let fetch = pb::FetchEvents { namespaces, limit: 0 };
    let fetch_frame = pb::Frame {
        version: PROTO_VERSION,
        stream_id: 1,
        r#type: FrameType::FetchEvents as i32,
        payload: Some(frame::Payload::FetchEvents(fetch)),
    };
    fetch_frame.write_to(&mut send).await?;

    // Read frames until stream closes or an error occurs
    loop {
        match pb::Frame::read_from(&mut recv).await? {
            None => {
                println!("ℹ️ Server closed stream.");
                break;
            }
            Some(incoming) => {
                match FrameType::try_from(incoming.r#type) {
                    Ok(FrameType::Event) => {
                        // Your per-event handler with ack+retry
                        handle_event_frame(incoming, &mut send).await?;
                    }
                    Ok(FrameType::EventsBatch) => {
                        // Split the batch and reuse the same event handler
                        if let Some(frame::Payload::EventsBatch(batch)) = incoming.payload {
                            for ev in batch.events {
                                let single = pb::Frame {
                                    version: PROTO_VERSION,
                                    stream_id: incoming.stream_id,
                                    r#type: FrameType::Event as i32,
                                    payload: Some(frame::Payload::Event(ev)),
                                };
                                handle_event_frame(single, &mut send).await?;
                            }
                        } else {
                            eprintln!("❌ EventsBatch frame missing payload");
                        }
                    }
                    Ok(other) => {
                        eprintln!("ℹ️ Ignoring frame type: {:?}", other);
                    }
                    Err(_) => {
                        eprintln!("❌ Unknown frame type: {}", incoming.r#type);
                    }
                }
            }
        }
    }

    Ok(())
}
