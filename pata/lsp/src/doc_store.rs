//! In-memory document store (URI → full text).

use std::collections::HashMap;
use tokio::sync::RwLock;

/// Thread-safe document store for LSP. Maps document URI to content.
pub struct DocStore {
    inner: RwLock<HashMap<String, String>>,
}

impl DocStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub async fn insert(&self, uri: String, text: String) {
        self.inner.write().await.insert(uri, text);
    }

    pub async fn get(&self, uri: &str) -> Option<String> {
        self.inner.read().await.get(uri).cloned()
    }
}

impl Default for DocStore {
    fn default() -> Self {
        Self::new()
    }
}
