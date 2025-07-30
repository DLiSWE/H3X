use bincode::Decode;
use serde::Deserialize;

/// Payload structure for Auth frames
#[derive(Debug, Deserialize, Decode)]
pub struct AuthPayload {
    pub client_id: String,
    pub token: String,
    pub namespace: String,
}

/// Payload structure for Event frames
#[derive(Debug, Deserialize, Decode)]
pub struct EventPayload {
    pub r#type: String,
    pub level: Option<String>,
    pub message: String,
    pub namespace: String,
    pub timestamp: String,
}
