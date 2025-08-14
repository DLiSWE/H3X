use quinn::SendStream;

use crate::protocol::h3x as pb;

// ACK an event back to the server
pub async fn ack_event(
    stream_id: u32,
    namespace: String,
    event_id: String,
    send: &mut SendStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let frame = pb::Frame {
        version: 1,
        stream_id,
        r#type: pb::FrameType::AckEvent as i32,
        payload: Some(pb::frame::Payload::AckEvent(pb::AckEvent {
            namespace,
            event_id: event_id.to_string(), // proto expects string
        })),
    };

    frame.write_to(send).await?;
    Ok(())
}

// Request events for a namespace
pub async fn fetch_events(
    stream_id: u32,
    namespaces: Vec<String>,
    max: usize,
    send: &mut SendStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let limit = u32::try_from(max).unwrap_or(u32::MAX);

    let frame = pb::Frame {
        version: 1,
        stream_id,
        r#type: pb::FrameType::FetchEvents as i32,
        payload: Some(pb::frame::Payload::FetchEvents(pb::FetchEvents {
            namespaces,
            limit,
        })),
    };

    frame.write_to(send).await?;
    Ok(())
}
