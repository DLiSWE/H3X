use crate::protocol::h3x::Frame;
use prost::Message;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_VARINT_BYTES: usize = 10; // protobuf varint max for u64

impl Frame {
    /// Encode self as a length-delimited protobuf message (varint length + message bytes).
    pub fn encode_len_delimited(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.encoded_len() + 5);
        self.encode_length_delimited(&mut buf).expect("encode failed");
        buf
    }

    /// Decode a length-delimited Frame from a byte slice.
    /// Returns (frame, bytes_consumed).
    pub fn decode_len_delimited(raw: &[u8]) -> Result<(Self, usize), prost::DecodeError> {
        let mut cursor = std::io::Cursor::new(raw);
        let frame = Self::decode_length_delimited(&mut cursor)?;
        let used = cursor.position() as usize;
        Ok((frame, used))
    }

    /// Read a single length-delimited Frame from an async reader.
    /// Returns Ok(None) on clean EOF before any bytes are read.
    pub async fn read_from<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Option<Self>> {
        Self::read_from_with_limit(reader, usize::MAX).await
    }

    /// Same as `read_from` but enforces a maximum message size.
    /// Useful to protect against unbounded allocations.
    pub async fn read_from_with_limit<R: AsyncRead + Unpin>(
        reader: &mut R,
        max_len: usize,
    ) -> io::Result<Option<Self>> {
        // 1) Read protobuf varint length prefix (u64).
        let (len, saw_eof_early) = read_varint_u64(reader).await?;
        if saw_eof_early {
            // Clean EOF before any varint byte: no more frames.
            return Ok(None);
        }
        let len: usize = len
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "length too large"))?;

        if len > max_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("frame too large: {} > {}", len, max_len),
            ));
        }

        // 2) Read exact number of message bytes.
        let mut msg = vec![0u8; len];
        reader.read_exact(&mut msg).await?;

        // 3) Decode the message (non-delimited; we already consumed the length).
        let frame = Frame::decode(&msg[..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode: {e}")))?;
        Ok(Some(frame))
    }

    /// Write this Frame to an async writer using length-delimited encoding.
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        let buf = self.encode_len_delimited();
        writer.write_all(&buf).await?;
        writer.flush().await
    }
}

/// Read a protobuf varint (u64) from `reader`.
/// Returns (value, saw_eof_early):
/// - `saw_eof_early = true` means we hit EOF before reading any varint byte (clean EOF).
/// - Otherwise, EOF mid-varint yields an UnexpectedEof error.
async fn read_varint_u64<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<(u64, bool)> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;

    for i in 0..MAX_VARINT_BYTES {
        let mut byte = [0u8; 1];
        let n = reader.read(&mut byte).await?;
        if n == 0 {
            // EOF
            if i == 0 {
                // No bytes read at all: clean EOF.
                return Ok((0, true));
            } else {
                // EOF mid-varint: error.
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "eof while reading varint",
                ));
            }
        }

        let b = byte[0];
        value |= ((b & 0x7F) as u64) << shift;

        if (b & 0x80) == 0 {
            // Last byte of varint.
            return Ok((value, false));
        }

        shift += 7;
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "varint too long",
    ))
}
