mod ping;
pub mod send;

use ping::{start_ping_loop, Shutdown};
use crate::tls::generate_or_load_cert;

use quinn::{ClientConfig, Connection, Endpoint};
use rustls::{ClientConfig as RustlsClientConfig, RootCertStore};

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub async fn run_client() {
    let mut endpoint = match Endpoint::client("0.0.0.0:0".parse().unwrap()) {
        Ok(ep) => ep,
        Err(e) => {
            eprintln!("❌ Failed to create endpoint: {e}");
            return;
        }
    };

    endpoint.set_default_client_config(build_client_config());

    loop {
        match connect_to_server(&endpoint).await {
            Ok(conn) => {
                println!("🤝 Connected to server.");
                match start_ping_loop(conn).await {
                    Ok(_) => {
                        println!("🔁 Attempting reconnection after connection closed...");
                    }
                    Err(Shutdown::ManualInterrupt) => {
                        println!("👋 Graceful shutdown complete.");
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Could not connect to server: {e}");
                sleep(Duration::from_secs(3)).await;
            }
        }
    }
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
