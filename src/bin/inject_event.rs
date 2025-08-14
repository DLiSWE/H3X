use anyhow::Result;
use chrono::Utc;
use sled::Db;
use std::collections::HashMap;
use uuid::Uuid;

use h3x::protocol::h3x::{
    frame, // the module that prost creates for the `oneof` payload
    Event,
    Frame,
    FrameType,
};
use prost::Message;

const PROTO_VERSION: u32 = 1;

fn main() -> Result<()> {
    let db: Db = sled::open("data/event_queue.db")?;

    let key = inject_event(&db, "env_namespace")?;
    println!("✅ Injected test event into sled under key: {key}");

    Ok(())
}

/// Create a sample Event, wrap it in a Frame, and persist to sled.
/// Returns the sled key used: "{namespace}:{uuid}".
fn inject_event(db: &Db, namespace: &str) -> Result<String> {
    let id = Uuid::new_v4().to_string();

    // Build the Event (prost-generated struct)
    let event = Event {
        id: id.clone(),
        namespace: namespace.to_string(),
        r#type: "Error".to_string(),
        message: "Connection timeout to upstream API".to_string(),
        data: br#"{"code":504,"service":"auth-service"}"#.to_vec(),
        timestamp: Utc::now().timestamp(),
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("severity".into(), "high".into());
            meta.insert("service".into(), "auth-service".into());
            meta.insert("env".into(), "production".into());
            meta
        },
    };

    // Wrap it in a Frame envelope (version + type + oneof payload)
    let frame = Frame {
        version: PROTO_VERSION,
        stream_id: 1,
        r#type: FrameType::Event as i32,
        payload: Some(frame::Payload::Event(event)),
    };

    // Serialize with plain protobuf encoding (no extra length prefix).
    // This is ideal if your sled values are single messages per key.
    let bytes = frame.encode_to_vec();

    // Key shape expected by your replay code
    let key = format!("{namespace}:{id}");

    db.insert(key.as_bytes(), bytes)?;
    db.flush()?;

    Ok(key)
}
