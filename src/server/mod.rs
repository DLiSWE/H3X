mod handlers;
mod payloads;

use quinn::{Endpoint, ServerConfig};
use std::net::SocketAddr;

use crate::tls::generate_or_load_cert;
use crate::protocol::frame::H3XFrame;
use handlers::handle_frame;


pub async fn run_server() {
    let (cert_chain, key) = generate_or_load_cert();
    let server_config = ServerConfig::with_single_cert(cert_chain, key).unwrap();
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let endpoint = Endpoint::server(server_config, addr).unwrap();

    println!("🚀 Server listening on {}", addr);

    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            let conn = match connecting.await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("❌ Failed to establish connection: {:?}", e);
                    return;
                }
            };

            println!("✅ Connection from {}", conn.remote_address());

            loop {
                match conn.accept_bi().await {
                    Ok((mut send, mut recv)) => {
                        tokio::spawn(async move {
                            while let Ok(Some(frame)) = H3XFrame::read_from(&mut recv).await {
                                println!("📦 Frame received: {:?}", frame.frame_type);
                                handle_frame(frame, &mut send).await;
                            }
                        });
                    }

                    Err(e) => {
                        eprintln!("❌ BI stream error: {:?}", e);
                        return;
                    }
                }
            }
        });
    }
}
