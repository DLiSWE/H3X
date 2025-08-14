mod ping;
pub mod params;
pub mod builder;
pub mod send;
pub mod event;
pub mod connection;

use crate::client::connection::{authenticate, receive_loop};
use crate::tls::generate_or_load_cert;
use crate::client::event::replay_events;
use crate::client::params::ClientParams;
use crate::state::registry::{ClientMetadata};
use tokio_util::sync::CancellationToken;

use quinn::{ClientConfig, Connection, Endpoint};
use rustls::{ClientConfig as RustlsClientConfig, RootCertStore};

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

pub async fn run_client(params: ClientParams, cancel_token: CancellationToken) {
    let mut registry_map = HashMap::new();
    for ns in &params.namespaces {
        registry_map.insert(
            ns.clone(),
            ClientMetadata {
                client_id: params.client_id(),
                token: params.token.clone(),
            },
        );
    }

    let mut endpoint = match Endpoint::client("0.0.0.0:0".parse().unwrap()) {
        Ok(ep) => ep,
        Err(e) => {
            eprintln!("❌ Failed to create endpoint: {e}");
            return;
        }
    };

    endpoint.set_default_client_config(build_client_config());

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                println!("🛑 Cancel token received, shutting down client...");
                break;
            }
            _ = async {
                match connect_to_server(&endpoint).await {
                    Ok(conn) => {
                        println!("🤝 Connected to server.");

                        if let Err(e) = authenticate(&conn, params.client_id(), params.token(), params.namespaces()).await {
                            eprintln!("❌ Authentication failed: {e}");
                            return;
                        }

                        // Open one BI stream to fetch+receive events
                        if let Err(e) = receive_loop(&conn, params.namespaces().to_vec()).await {
                            eprintln!("❌ Receive loop ended: {e}");
                        }

                        if let Err(e) = replay_events(&conn, params.namespaces().to_vec()).await {
                            eprintln!("❌ Replay events failed: {e}");
                        }

                        println!("🔁 Attempting reconnection...");
                    }
                    Err(e) => {
                        eprintln!("❌ Could not connect to server: {e}");
                        sleep(Duration::from_secs(3)).await;
                    }
                }
            } => {}
        }
    }

    println!("👋 Client shut down cleanly.");
}

fn build_client_config() -> ClientConfig {
    let (cert_chain, _) = generate_or_load_cert();
    let cert = &cert_chain[0];

    let mut roots = RootCertStore::empty();
    roots.add(cert).unwrap();

    let client_crypto = RustlsClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();

    ClientConfig::new(Arc::new(client_crypto))
}

async fn connect_to_server(endpoint: &Endpoint) -> Result<Connection, quinn::ConnectionError> {
    let connecting = endpoint.connect("127.0.0.1:5000".parse().unwrap(), "localhost")
        .map_err(|e| {
            eprintln!("❌ Failed to start connection: {e}");
            quinn::ConnectionError::LocallyClosed
        })?;

    connecting.await
}
