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

    /// Drop a document from the store on `textDocument/didClose`. Without this, closed files
    /// stay resident in server memory for the life of the session, and their last-published
    /// diagnostics are never cleared (nothing re-publishes an empty list for them).
    pub async fn remove(&self, uri: &str) {
        self.inner.write().await.remove(uri);
    }

    /// Return all (uri, text) pairs currently stored.
    pub async fn all(&self) -> Vec<(String, String)> {
        self.inner.read().await.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

impl Default for DocStore {
    fn default() -> Self {
        Self::new()
    }
}
