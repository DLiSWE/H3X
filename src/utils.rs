// utils.rs
use prost::Message;
use crate::state::registry::NamespaceRegistry;
use crate::protocol::h3x as pb;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// Generic protobuf helpers (no bincode needed)
pub fn serialize<M: Message>(msg: &M) -> Vec<u8> {
    let mut buf = Vec::with_capacity(msg.encoded_len());
    msg.encode(&mut buf).expect("encode failed");
    buf
}

pub fn serialize_len_delimited<M: Message>(msg: &M) -> Vec<u8> {
    let mut buf = Vec::with_capacity(msg.encoded_len() + 5);
    msg.encode_length_delimited(&mut buf).expect("encode failed");
    buf
}

pub fn deserialize<M: Default + Message>(bytes: &[u8]) -> Result<M, prost::DecodeError> {
    M::decode(bytes)
}

pub fn deserialize_len_delimited<M: Default + Message>(bytes: &[u8]) -> Result<(M, usize), prost::DecodeError> {
    let mut cur = std::io::Cursor::new(bytes);
    let msg = M::decode_length_delimited(&mut cur)?;
    Ok((msg, cur.position() as usize))
}

// Same helper, just simplify signature
pub fn extract_namespace_from_client_id(client_id: &str) -> Option<&str> {
    client_id.strip_prefix("client_id:")
}

pub async fn validate_auth(payload: &pb::Auth, registry: &NamespaceRegistry) -> bool {
    let map = registry.read().await;
    let id = &payload.client_id;


    match map.get(id) {
        Some(meta) => {
            meta.token == payload.token
        }
        None => {
            println!("❌ No such client ID in registry: {}", id);
            false
        }
    }
}