use anyhow::Result;
use quinn::{Connection, SendStream};
use tokio::time::{sleep, Duration};

use crate::protocol::h3x as pb;   // prost-generated module
use crate::protocol::h3x::Frame;  // has read_from / write_to impls

use crate::client::send::fetch_events; // keep your helpers; they should build pb::Frame internally
use super::send::ack_event;            // same: should send a pb::Frame::AckEvent

pub async fn handle_event_frame(
    frame: pb::Frame,
    send: &mut SendStream,
) -> Result<()> {
    match (pb::FrameType::try_from(frame.r#type), frame.payload) {
        (Ok(pb::FrameType::Event), Some(pb::frame::Payload::Event(event))) => {
            println!("📥 Received Event [{}]: {}", event.namespace, event.r#type);

            // Retry ack if it fails
            let mut attempts = 0usize;
            let max_retries = 5usize;
            let mut delay = Duration::from_secs(1);

            while attempts < max_retries {
                match ack_event(frame.stream_id, event.namespace.clone(), event.id.clone(), send).await {
                    Ok(_) => {
                        println!("✅ Acked event {}", event.id);
                        break;
                    }
                    Err(e) => {
                        attempts += 1;
                        eprintln!("❌ Failed to ack event (attempt {attempts}): {e}");
                        sleep(delay).await;
                        delay *= 2;
                    }
                }
            }

            if attempts == max_retries {
                eprintln!("❌ Giving up ack after {} attempts for event {}", max_retries, event.id);
            }
        }
        // Not an Event frame; ignore or log as needed.
        (kind, _) => {
            eprintln!("ℹ️ handle_event_frame called with non-Event kind: {:?}", kind);
        }
    }

    Ok(())
}

pub async fn replay_events(
    conn: &Connection,
    namespace: String,
) -> Result<()> {
    // Open bidirectional stream to request replay
    let (mut send, mut recv) = conn.open_bi().await?;

    // Send FetchEvents request (your helper should build a pb::Frame internally)
    let stream_id: u32 = send.id().index().try_into().unwrap_or(0);
    fetch_events(stream_id, namespace.clone(), 100, &mut send).await;

    // Expect an EventsBatch in response
    let response = match Frame::read_from(&mut recv).await? {
        Some(resp) => resp,
        None => {
            eprintln!("❌ Unexpected EOF waiting for EventsBatch response");
            return Ok(());
        }
    };

    let (kind, payload) = (pb::FrameType::try_from(response.r#type), response.payload);
    let batch = match (kind, payload) {
        (Ok(pb::FrameType::EventsBatch), Some(pb::frame::Payload::EventsBatch(batch))) => batch,
        other => {
            eprintln!("❌ Unexpected or missing EventsBatch response: {:?}", other);
            return Ok(());
        }
    };

    println!("🔁 Replaying {} persisted events", batch.events.len());

    for event in batch.events {
        println!("📥 Replaying Event [{}]: {}", event.namespace, event.r#type);

        // Acknowledge after processing
        let mut attempts = 0usize;
        let max_retries = 5usize;
        let mut delay = Duration::from_secs(1);

        while attempts < max_retries {
            match ack_event(response.stream_id, event.namespace.clone(), event.id.clone(), &mut send).await {
                Ok(_) => {
                    println!("✅ Acked replayed event {}", event.id);
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    eprintln!("❌ Failed to ack replayed event (attempt {attempts}): {e}");
                    sleep(delay).await;
                    delay *= 2;
                }
            }
        }

        if attempts == max_retries {
            eprintln!("❌ Giving up on ack for replayed event {}", event.id);
        }
    }

    if let Err(e) = send.finish().await {
        eprintln!("❌ Failed to finish stream after replay: {e}");
    }

    Ok(())
}
