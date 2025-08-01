use quinn::Connection;
use std::time::Duration;
use tokio::signal::ctrl_c;
use tokio::time::sleep;

use crate::protocol::frame::H3XFrame;
use crate::protocol::payloads::{EventPayload, EventsBatchPayload};
use crate::protocol::types::FrameType;
use crate::client::send::ack_event;
use crate::utils::deserialize_payload;

#[derive(Debug)]
pub enum Shutdown {
    ManualInterrupt,
}

pub async fn start_ping_loop(conn: Connection) -> Result<(), Shutdown> {
    let mut retry_delay = Duration::from_secs(1);
    let mut consecutive_failures = 0;

    let shutdown_signal = async {
        ctrl_c().await.expect("Failed to listen for shutdown signal");
        println!("👋 Received Ctrl+C, shutting down...");
        Err(Shutdown::ManualInterrupt)
    };

    let ping_loop = async {
        loop {
            match conn.open_bi().await {
                Ok((mut send, mut recv)) => {
                    retry_delay = Duration::from_secs(1);
                    consecutive_failures = 0;

                    let frame = H3XFrame {
                        stream_id: 99,
                        frame_type: FrameType::Ping,
                        payload: vec![],
                    };

                    if let Err(e) = frame.write_to(&mut send).await {
                        eprintln!("❌ Failed to send frame: {e}");
                        break;
                    }

                    if let Err(e) = send.finish().await {
                        eprintln!("❌ Failed to finish stream: {e}");
                        break;
                    }

                    match H3XFrame::read_from(&mut recv).await {
                        Ok(Some(reply)) => match reply.frame_type {
                            FrameType::Pong => {
                                println!("📬 Server replied: {:?}", reply);
                            }

                            FrameType::Event => {
                                let event: EventPayload = deserialize_payload(&reply.payload);
                                println!("📥 Received event: {} ({})", event.r#type, event.id);

                                // ACK after handling
                                if let Ok((mut ack_send, _)) = conn.open_bi().await {
                                    if let Err(e) = ack_event(
                                        reply.stream_id,
                                        event.namespace.clone(),
                                        event.id,
                                        &mut ack_send,
                                    )
                                    .await
                                    {
                                        eprintln!("❌ Failed to ack event: {e}");
                                    } else {
                                        println!("✅ Acked event: {}", event.id);
                                    }
                                } else {
                                    eprintln!("❌ Failed to open stream for ack");
                                }
                            }

                            FrameType::EventsBatch => {
                                let batch: EventsBatchPayload = deserialize_payload(&reply.payload);

                                for event in batch.events {
                                    println!("📥 Batch event: {} ({})", event.r#type, event.id);

                                    if let Ok((mut ack_send, _)) = conn.open_bi().await {
                                        if let Err(e) = ack_event(
                                            reply.stream_id,
                                            event.namespace.clone(),
                                            event.id,
                                            &mut ack_send,
                                        )
                                        .await
                                        {
                                            eprintln!("❌ Failed to ack batch event: {e}");
                                        } else {
                                            println!("✅ Acked batch event: {}", event.id);
                                        }
                                    }
                                }
                            }

                            _ => {
                                eprintln!("⚠️ Unhandled frame type: {:?}", reply.frame_type);
                            }
                        },

                        Ok(None) => {
                            println!("📴 Server closed stream");
                            break;
                        }

                        Err(e) => {
                            eprintln!("❌ Failed to read response: {e}");
                            break;
                        }
                    }
                }

                Err(e) => {
                    consecutive_failures += 1;
                    eprintln!(
                        "❌ Failed to open stream: {e}. Retrying in {}s...",
                        retry_delay.as_secs()
                    );
                    sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(Duration::from_secs(30));

                    if consecutive_failures > 5 {
                        eprintln!("❌ Too many failed attempts. Triggering reconnect.");
                        break;
                    }
                }
            }

            sleep(Duration::from_secs(5)).await;
        }

        Ok(())
    };

    tokio::select! {
        result = shutdown_signal => result,
        result = ping_loop => result,
    }
}


