use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use bincode::{Encode, Decode};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub struct EventPayload {
    #[bincode(with_serde)]
    pub id: Uuid,
    pub namespace: String,
    pub r#type: String,
    pub message: String,
    pub data: Vec<u8>,
    pub timestamp: i64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub struct EventsBatchPayload {
    pub events: Vec<EventPayload>,
}


#[derive(Debug, Deserialize, Encode, Decode)]
pub struct AckEventPayload {
    #[bincode(with_serde)]
    pub event_id: Uuid,
    pub namespace: String,
}

#[derive(Debug, Deserialize, Encode, Decode)]
pub struct FetchEventsPayload {
    pub namespace: String,
    pub max: usize,
}

#[derive(Debug, Deserialize, Decode)]
pub struct AuthPayload {
    pub client_id: String,
    pub token: String,
    pub namespace: String,
}

