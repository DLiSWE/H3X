use bincode::{encode_to_vec, decode_from_slice, config::standard, Decode, Encode};
use quinn::Connection;
use std::time::Duration;
use tokio::signal;
use tokio::time::sleep;

use crate::protocol::frame::H3XFrame;
use crate::protocol::types::FrameType;

pub fn serialize_payload<T: Encode>(payload: &T) -> Vec<u8> {
    encode_to_vec(payload, standard()).expect("serialization failed")
}

pub fn deserialize_payload<T: Decode<()>>(data: &[u8]) -> T {
    decode_from_slice::<T, _>(data, standard())
        .expect("deserialization failed")
        .0
}

pub async fn start_ping_loop(conn: Connection) {
    let mut retry_delay = Duration::from_secs(1);

    let shutdown_signal = async {
        signal::ctrl_c().await.expect("Failed to listen for shutdown signal");
        println!("👋 Received Ctrl+C, shutting down...");
    };

    let ping_loop = async {
        loop {
            match conn.open_bi().await {
                Ok((mut send, mut recv)) => {
                    retry_delay = Duration::from_secs(1);

                    let frame = H3XFrame {
                        stream_id: 99,
                        frame_type: FrameType::Ping,
                        payload: vec![],
                    };

                    if let Err(e) = frame.write_to(&mut send).await {
                        eprintln!("❌ Failed to send frame: {e}");
                        continue;
                    }

                    if let Err(e) = send.finish().await {
                        eprintln!("❌ Failed to finish stream: {e}");
                        continue;
                    }

                    match H3XFrame::read_from(&mut recv).await {
                        Ok(Some(reply)) => println!("📬 Server replied: {:?}", reply),
                        Ok(None) => println!("📴 Server closed stream"),
                        Err(e) => eprintln!("❌ Failed to read response: {e}"),
                    }
                }
                Err(e) => {
                    eprintln!(
                        "❌ Failed to open stream: {e}. Retrying in {}s...",
                        retry_delay.as_secs()
                    );
                    sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(Duration::from_secs(30));
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    };

    tokio::select! {
        _ = shutdown_signal => {}
        _ = ping_loop => {}
    }
}

