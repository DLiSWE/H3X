use anyhow::{bail, Result};
use quinn::Connection;
use crate::protocol::h3x as pb;
use crate::protocol::h3x::Frame;
use std::convert::TryFrom;

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
