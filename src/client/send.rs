use crate::protocol::frame::H3XFrame;
use crate::protocol::payloads::AckEventPayload;
use crate::protocol::types::FrameType;
use crate::utils::serialize_payload;

use quinn::SendStream;
use uuid::Uuid;

pub async fn ack_event(
    stream_id: u32,
    namespace: String,
    event_id: Uuid,
    send: &mut SendStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = AckEventPayload {
        namespace,
        event_id,
    };

    let frame = H3XFrame {
        stream_id,
        frame_type: FrameType::AckEvent,
        payload: serialize_payload(&payload),
    };

    frame.write_to(send).await?;
    send.finish().await?;
    Ok(())
}
