use crate::protocol::types::FrameType;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct H3XFrame {
    pub stream_id: u32,
    pub frame_type: FrameType,
    pub payload: Vec<u8>,
}

impl H3XFrame {
    // Encodes the H3XFrame into a byte vector.
    // The format is: [stream_id(4 bytes)][frame_type(1 byte)][payload_length(4 bytes)][payload]
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(9 + self.payload.len());
        // 4 bytes for stream_id, 1 byte for frame_type, 4 bytes for payload length
        buf.extend_from_slice(&self.stream_id.to_be_bytes());
        buf.push(self.frame_type as u8);
        buf.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    // Decodes a raw byte slice into an H3XFrame.
    // Returns None if the slice is too short or if the length does not match the payload.
    pub fn decode(raw: &[u8]) -> Option<H3XFrame> {
        // Assuming the first 4 bytes are stream_id, the next byte is frame_type,
        // the next 4 bytes are payload length, and the rest is the payload.
        let stream_id = u32::from_be_bytes(raw[0..4].try_into().ok()?);
        let frame_type = FrameType::from(raw[4]);
        let len = u32::from_be_bytes(raw[5..9].try_into().ok()?) as usize;
        if raw.len() < 9 + len {
            return None;
        }
        let payload = raw[9..9 + len].to_vec();
        Some(H3XFrame {
            stream_id,
            frame_type,
            payload,
        })
    }

    // Reads an H3XFrame from an asynchronous reader.
    // Returns None if the end of the stream is reached before a complete frame is read.
    pub async fn read_from<R: AsyncReadExt + Unpin>(reader: &mut R) -> tokio::io::Result<Option<H3XFrame>> {
        let mut header = [0u8; 9];
        if reader.read_exact(&mut header).await.is_err() {
            return Ok(None);
        }

        let stream_id = u32::from_be_bytes(header[0..4].try_into().unwrap());
        let frame_type = FrameType::from(header[4]);
        let len = u32::from_be_bytes(header[5..9].try_into().unwrap()) as usize;

        let mut payload = vec![0u8; len];
        reader.read_exact(&mut payload).await?;
        Ok(Some(H3XFrame { stream_id, frame_type, payload }))
    }

    // Writes the H3XFrame to an asynchronous writer.
    // This will encode the frame and write it to the writer.
    pub async fn write_to<W: AsyncWriteExt + Unpin>(&self, writer: &mut W) -> tokio::io::Result<()> {
        let buf = self.encode();
        writer.write_all(&buf).await?;
        writer.flush().await
    }

}
