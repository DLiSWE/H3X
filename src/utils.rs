// utils.rs
use crate::state::registry::NamespaceRegistry;
use crate::protocol::h3x as pb;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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