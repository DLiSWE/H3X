mod tls;
mod server;
mod client;
mod protocol;
mod utils;

use tokio::signal;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("server") => {
            let server_task = tokio::spawn(async {
                server::run_server().await;
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
            client::run_client().await;
        }

        _ => eprintln!("Usage: cargo run -- [server|client]"),
    }
}
