mod handlers;

use quinn::{Endpoint, ServerConfig};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

use std::convert::TryFrom;

use crate::state::queue::EventQueue;
use crate::tls::generate_or_load_cert;
use crate::state::registry::{ClientMetadata, NamespaceRegistry};

use crate::protocol::h3x::{
    Frame as H3XFrame,
    FrameType,
};

pub async fn run_server(client_id: String, token: String, ns: String) {
    let (cert_chain, key) = generate_or_load_cert();
    let server_config = ServerConfig::with_single_cert(cert_chain, key).unwrap();
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let endpoint = Endpoint::server(server_config, addr).unwrap();

    let event_queue = EventQueue::new("data/event_queue.db")
        .expect("Failed to initialize event queue");

    let mut registry_map: HashMap<String, ClientMetadata> = HashMap::new();
    registry_map.insert(
        format!("client_id:{}", client_id.clone()),
        ClientMetadata {
            client_id,
            token,
        },
    );

    // registry setup...
    let registry: NamespaceRegistry = Arc::new(RwLock::new(registry_map));
    println!("🚀 Server listening on {}", addr);

    while let Some(connecting) = endpoint.accept().await {
        let registry = registry.clone();
        let queue = event_queue.clone();

        tokio::spawn(async move {
            let conn = match connecting.await {
                Ok(c) => c,
                Err(e) => { eprintln!("❌ Failed to establish connection: {:?}", e); return; }
            };

            println!("✅ Connection from {}", conn.remote_address());

            loop {
                match conn.accept_bi().await {
                    Ok((mut send, mut recv)) => {
                        let registry = registry.clone();
                        let queue = queue.clone();

                        tokio::spawn(async move {
                            loop {
                                match H3XFrame::read_from(&mut recv).await {
                                    Ok(Some(frame)) => {
                                        println!("📦 Frame received: {:?}", FrameType::try_from(frame.r#type));
                                        if let Err(e) = handlers::handle_frame(
                                            frame,
                                            &mut send,
                                            &mut recv,
                                            registry.clone(),
                                            queue.clone(),
                                        ).await {
                                            eprintln!("❌ Handler error: {e}");
                                            break;
                                        }
                                    }
                                    Ok(None) => { println!("📴 Stream closed by client."); break; }
                                    Err(e) => { eprintln!("❌ Stream read error: {:?}", e); break; }
                                }
                            }
                        });
                    }
                    Err(e) => { eprintln!("❌ BI stream error: {:?}", e); return; }
                }
            }
        });
    }
}
