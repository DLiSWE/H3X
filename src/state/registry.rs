use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ClientMetadata {
    pub client_id: String,
    pub token: String,
}

pub type NamespaceRegistry = Arc<RwLock<HashMap<String, ClientMetadata>>>;
