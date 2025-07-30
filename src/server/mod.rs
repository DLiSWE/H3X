mod handlers;
mod payloads;
mod state;

use quinn::{Endpoint, ServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::tls::generate_or_load_cert;
use crate::protocol::frame::H3XFrame;
use handlers::handle_frame;
use state::NamespaceRegistry;

pub async fn run_server() {
    let (cert_chain, key) = generate_or_load_cert();
    let server_config = ServerConfig::with_single_cert(cert_chain, key).unwrap();
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let endpoint = Endpoint::server(server_config, addr).unwrap();
    let registry: NamespaceRegistry = Arc::new(RwLock::new(HashMap::new()));

    println!("🚀 Server listening on {}", addr);

    while let Some(connecting) = endpoint.accept().await {
        let registry = registry.clone();
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
                        let registry = registry.clone();
                        tokio::spawn(async move {
                            while let Ok(Some(frame)) = H3XFrame::read_from(&mut recv).await {
                                println!("📦 Frame received: {:?}", frame.frame_type);
                                handle_frame(frame, &mut send, registry.clone()).await;
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
