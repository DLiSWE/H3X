mod tls;
mod server;
mod client;
mod protocol;
mod utils;
mod state;

use tokio::signal;
use dotenv::dotenv;

use client::builder::ClientBuilder;
use tokio_util::sync::CancellationToken;

use crate::client::run_client;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = std::env::var("H3X_CLIENT_TOKEN").unwrap_or("default_token".into());
    let ns = std::env::var("H3X_CLIENT_NAMESPACE").unwrap_or_else(|_| "default_namespace".into());
    let id = std::env::var("H3X_CLIENT_ID").unwrap_or_else(|_| "default_id".into());

    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("server") => {
            let server_task = tokio::spawn(async {
                server::run_server(id, token, ns).await;
            });

            let shutdown_task = tokio::spawn(async {
                signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
                println!("\n🛑 Received Ctrl+C, shutting down server...");
            });

            tokio::select! {
                _ = server_task => {},
                _ = shutdown_task => {},
            }

            println!("👋 Server exit complete.");
        }

        Some("client") => {
            let params = ClientBuilder::new()
                .namespace(ns)
                .token(token)
                .client_id(id)
                .build()
                .expect("Failed to build client params");

            let cancel_token = CancellationToken::new();
            let client_cancel = cancel_token.clone();

            let client_task = tokio::spawn(async move {
                run_client(params, client_cancel).await;
            });

            let shutdown_task = tokio::spawn(async move {
                tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
                println!("👋 Received Ctrl+C, shutting down client...");
                cancel_token.cancel(); // 🚨 trigger shutdown
            });

            tokio::select! {
                _ = client_task => {},
                _ = shutdown_task => {},
            }
        }

        _ => eprintln!("Usage: cargo run -- [server|client]"),
    }
}
