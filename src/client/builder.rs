use crate::client::params::ClientParams;

pub struct ClientBuilder {
    client_id: String,
    namespaces: Vec<String>,
    token: Option<String>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            client_id: "client_id:default".into(),
            namespaces: vec![],
            token: None,
        }
    }

    pub fn client_id<T: Into<String>>(mut self, id: T) -> Self {
        self.client_id = id.into();
        self
    }

    pub fn namespace<T: Into<String>>(mut self, ns: T) -> Self {
        self.namespaces = vec![ns.into()];
        self
    }

    pub fn namespaces<T: Into<String>>(mut self, list: Vec<T>) -> Self {
        self.namespaces = list.into_iter().map(|v| v.into()).collect();
        self
    }

    pub fn token<T: Into<String>>(mut self, token: T) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn build(self) -> Result<ClientParams, String> {
        if self.namespaces.is_empty() {
            return Err("At least one namespace is required".into());
        }

        let token = self.token.ok_or("Token must be provided")?;
        Ok(ClientParams::new(format!("client_id:{}", self.client_id), self.namespaces, token))
    }
}
