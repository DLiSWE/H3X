#[derive(Clone)]
pub struct ClientParams {
    pub client_id: String,
    pub namespaces: Vec<String>,
    pub token: String,
}

impl ClientParams {
    pub fn new(client_id: String, namespaces: Vec<String>, token: String) -> Self {
        Self { client_id, namespaces, token }
    }

    pub fn client_id(&self) -> String {
        self.client_id.clone()
    }

    pub fn namespaces(&self) -> Vec<String> {
        self.namespaces.clone()
    }

    pub fn token(&self) -> String {
        self.token.clone()
    }
}
