use sled::Db;
use anyhow::Result;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

fn main() -> Result<()> {
    // let db: Db = sled::open("data/event_queue.db")?;

    // let test_event = EventPayload {
    //     id: Uuid::new_v4(),
    //     namespace: "env_namespace".to_string(),
    //     r#type: "Error".to_string(),
    //     message: "Connection timeout to upstream API".to_string(),
    //     data: b"{\"code\":504,\"service\":\"auth-service\"}".to_vec(),
    //     timestamp: Utc::now().timestamp(),
    //     metadata: {
    //         let mut meta = HashMap::new();
    //         meta.insert("severity".to_string(), "high".to_string());
    //         meta.insert("service".to_string(), "auth-service".to_string());
    //         meta.insert("env".to_string(), "production".to_string());
    //         meta
    //     },
    // };

    // let serialized_payload = serialize_payload(&test_event);

    // let frame = H3XFrame {
    //     stream_id: 1,
    //     frame_type: FrameType::Event,
    //     payload: serialized_payload,
    // };

    // // ✅ Match the format expected by sled.scan_prefix(namespace + ":")
    // let key = format!("{}:{}", test_event.namespace, test_event.id);

    // db.insert(key.clone(), frame.encode())?;
    // db.flush()?;

    // println!("✅ Injected test event into sled under key: {}", key);
    Ok(())
}