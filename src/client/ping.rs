use quinn::Connection;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use std::convert::TryFrom;

use crate::protocol::h3x::{
    Frame as H3XFrame,
    FrameType,
    frame,
    Ping,
    EventsBatch,
    AckEvent,
};

const PROTO_VERSION: u32 = 1;
const CONTROL_STREAM_ID: u32 = 99;

#[derive(Debug)]
pub enum Shutdown {
    ManualInterrupt,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub async fn start_ping_loop(conn: Connection) -> Result<(), Shutdown> {
    let mut retry_delay = Duration::from_secs(1);
    let mut consecutive_failures = 0;

    loop {
        match conn.open_bi().await {
            Ok((mut send, mut recv)) => {
                retry_delay = Duration::from_secs(1);
                consecutive_failures = 0;

                // --- Send Ping with payload in the oneof ---
                let ping_frame = H3XFrame {
                    version: PROTO_VERSION,
                    stream_id: CONTROL_STREAM_ID,
                    r#type: FrameType::Ping as i32,
                    payload: Some(frame::Payload::Ping(Ping {
                        timestamp_ms: now_ms(),
                        seq: 0, // add sequencing if you want
                    })),
                };

                if let Err(e) = ping_frame.write_to(&mut send).await {
                    eprintln!("❌ Failed to send Ping: {e}");
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                // Keep stream open so server can reply Pong and push EventsBatch.

                // --- Read loop: Pong / EventsBatch / others ---
                while let Ok(Some(frame)) = H3XFrame::read_from(&mut recv).await {
                        let Ok(ft) = FrameType::try_from(frame.r#type) else {
                            eprintln!("⚠️ Unknown frame type value: {}", frame.r#type);
                            continue;
                        };

                    match ft {
                        FrameType::Pong => {
                            if let Some(frame::Payload::Pong(p)) = frame.payload {
                                println!("📬 Pong received (echo_ts={}ms server_ts={}ms seq={})",
                                    p.echo_timestamp_ms, p.server_time_ms, p.seq);
                            } else {
                                println!("📬 Pong received (no payload)");
                            }
                        }

                        FrameType::EventsBatch => {
                            match frame.payload {
                                Some(frame::Payload::EventsBatch(EventsBatch { events })) => {
                                    if !events.is_empty() {
                                        println!("🚚 Received {} event(s)", events.len());
                                    }
                                    // Process events, then ack each one
                                    for ev in events {
                                        let ack = AckEvent {
                                            namespace: ev.namespace.clone(),
                                            event_id: ev.id.clone(),
                                        };
                                        let ack_frame = H3XFrame {
                                            version: PROTO_VERSION,
                                            stream_id: CONTROL_STREAM_ID,
                                            r#type: FrameType::AckEvent as i32,
                                            payload: Some(frame::Payload::AckEvent(ack)),
                                        };
                                        if let Err(e) = ack_frame.write_to(&mut send).await {
                                            eprintln!("⚠️ Failed to send AckEvent for {}: {e}", ev.id);
                                        }
                                    }
                                }
                                other => {
                                    eprintln!("⚠️ EventsBatch frame missing payload (got: {:?})", other.is_some());
                                }
                            }
                        }

                        other => {
                            eprintln!("⚠️ Unhandled frame type: {:?}", other);
                        }
                    }
                }

                // Stream ended; short delay and reopen.
                sleep(Duration::from_secs(5)).await;
            }

            Err(e) => {
                consecutive_failures += 1;
                eprintln!(
                    "❌ Failed to open BI stream: {e}. Retrying in {}s...",
                    retry_delay.as_secs()
                );
                sleep(retry_delay).await;
                retry_delay = (retry_delay * 2).min(Duration::from_secs(30));

                if consecutive_failures > 5 {
                    eprintln!("❌ Too many failures opening stream. Triggering reconnect.");
                    break;
                }
            }
        }
    }

    Ok(())
}
