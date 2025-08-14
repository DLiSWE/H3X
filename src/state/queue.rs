use sled::{Db, IVec, Result, open};
use std::sync::Arc;
use prost::Message;

use crate::protocol::h3x::{
    Frame as H3XFrame,
    frame, // for the oneof
};

#[derive(Clone)]
pub struct EventQueue {
    pub db: Arc<Db>,
}

impl EventQueue {
    pub fn new(path: &str) -> Result<Self> {
        let db = open(path)?;
        Ok(Self { db: Arc::new(db) })
    }

    /// Enqueue a single Event frame into its namespace tree.
    /// Expects `frame.payload` to be `Some(frame::Payload::Event(_))`.
    pub fn enqueue(&self, frame: &H3XFrame) -> Result<()> {
        let ns = match &frame.payload {
            Some(frame::Payload::Event(ev)) => &ev.namespace,
            _ => {
                // You can choose to return an error instead; using sled::Error here would be awkward,
                // so we no-op with Ok(()) to keep the signature. Change if you prefer strictness.
                eprintln!("enqueue: skipping non-Event frame");
                return Ok(());
            }
        };

        let tree = self.db.open_tree(ns)?;
        let id = self.db.generate_id()?;
        let value = frame.encode_to_vec();
        tree.insert(id.to_be_bytes(), value)?;
        Ok(())
    }

    /// Fetch up to `max` frames from a namespace, decoding each prost Frame.
    /// Returns owned data to avoid lifetime issues with sled iterators.
    pub fn fetch(&self, namespace: &str, max: Option<usize>) -> Result<Vec<(u64, H3XFrame)>> {
        let tree = self.db.open_tree(namespace)?;
        let iter = tree.iter();
        let mut out = Vec::new();

        for res in iter {
            let (k, v) = match res {
                Ok(kv) => kv,
                Err(e) => {
                    eprintln!("fetch: sled iter error: {e}");
                    continue;
                }
            };

            if k.len() != 8 {
                eprintln!("fetch: bad key length {}, skipping", k.len());
                continue;
            }

            let id = u64::from_be_bytes(k.as_ref().try_into().unwrap());
            match H3XFrame::decode(v.as_ref()) {
                Ok(frame) => out.push((id, frame)),
                Err(e) => eprintln!("fetch: failed to decode prost Frame: {e}"),
            }

            if let Some(limit) = max {
                if out.len() >= limit {
                    break;
                }
            }
        }

        Ok(out)
    }

    /// Remove a stored frame by numeric ID from a namespace tree.
    pub fn remove(&self, namespace: &str, id: u64) -> Result<Option<IVec>> {
        let tree = self.db.open_tree(namespace)?;
        tree.remove(id.to_be_bytes())
    }
}
