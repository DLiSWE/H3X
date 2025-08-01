use sled::{Db, IVec, Result, open};
use std::sync::Arc;
use crate::protocol::frame::H3XFrame;
use crate::protocol::payloads::EventPayload;
use crate::utils::deserialize_payload;

#[derive(Clone)]
pub struct EventQueue {
    pub db: Arc<Db>,
}

impl EventQueue {
    pub fn new(path: &str) -> Result<Self> {
        let db = open(path)?;
        Ok(Self { db: Arc::new(db) })
    }

    pub fn enqueue(&self, frame: &mut H3XFrame) -> Result<()> {
        // Deserialize the payload to extract the namespace
        let event: EventPayload = deserialize_payload(&frame.payload);
        let tree = self.db.open_tree(&event.namespace)?;
        let id = self.db.generate_id()?;
        tree.insert(id.to_be_bytes(), frame.encode())?;
        Ok(())
    }

    pub fn iter(&self, namespace: &str) -> Result<impl Iterator<Item = (u64, H3XFrame)>> {
        let tree = self.db.open_tree(namespace)?;
        Ok(tree.iter().filter_map(|res| {
            res.ok().and_then(|(k, v)| {
                let id = u64::from_be_bytes(k.as_ref().try_into().ok()?);
                let frame = H3XFrame::decode(&v)?;
                Some((id, frame))
            })
        }))
    }

    pub fn remove(&self, namespace: &str, id: u64) -> Result<Option<IVec>> {
        let tree = self.db.open_tree(namespace)?;
        tree.remove(id.to_be_bytes())
    }
}
