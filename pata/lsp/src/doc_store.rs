//! In-memory document store (URI → full text), plus a per-document diagnostics cache keyed by
//! content hash — the incremental-resolution fix from `docs/design/pata-implementation-spec.md`
//! Section 19: skip re-lex/re-parse/re-analyze entirely on a `didChange` whose new text hashes
//! identically to what's already cached (a real case some editors hit — a cursor-only movement
//! or no-op edit event still fires `didChange`), rather than the full salsa-style
//! incremental-computation framework that section's own "Decision made" explicitly rejected as
//! a rewrite, not an incremental improvement.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Diagnostic;

/// One document's last-computed analysis, valid as long as its text still hashes to
/// `content_hash` — same `DefaultHasher` approach as `pata_core::interface_registry`'s own
/// `fingerprint()`, for consistency across this codebase's few content-hash use sites rather
/// than picking a third hashing scheme.
#[derive(Clone)]
struct CachedAnalysis {
    content_hash: u64,
    diagnostics: Vec<Diagnostic>,
}

fn hash_text(text: &str) -> u64 {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

/// Thread-safe document store for LSP. Maps document URI to content, plus a parallel
/// content-hash-keyed diagnostics cache.
pub struct DocStore {
    inner: RwLock<HashMap<String, String>>,
    cache: RwLock<HashMap<String, CachedAnalysis>>,
    /// `#[cfg(test)]`-only counter of how many times `compute_or_reuse_diagnostics`'s closure
    /// actually ran (a real recomputation, not a cache hit) — the only way to observe "was this
    /// actually re-analyzed" from outside, per Section 19's own acceptance-check note, since a
    /// cache hit and a cache miss are otherwise externally indistinguishable (both return a
    /// `Vec<Diagnostic>`).
    #[cfg(test)]
    pub recompute_count: std::sync::atomic::AtomicUsize,
}

impl DocStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
            #[cfg(test)]
            recompute_count: std::sync::atomic::AtomicUsize::new(0),
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
        self.cache.write().await.remove(uri);
    }

    /// Return all (uri, text) pairs currently stored.
    pub async fn all(&self) -> Vec<(String, String)> {
        self.inner.read().await.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Return `uri`'s cached diagnostics if `text` hashes identically to what's cached for it
    /// (a real cache hit — same content, nothing to recompute), else run `compute` and cache the
    /// fresh result under `text`'s hash. `compute` is only ever invoked on an actual miss.
    pub async fn diagnostics_for(
        &self,
        uri: &str,
        text: &str,
        compute: impl FnOnce() -> Vec<Diagnostic>,
    ) -> Vec<Diagnostic> {
        let hash = hash_text(text);
        if let Some(cached) = self.cache.read().await.get(uri) {
            if cached.content_hash == hash {
                return cached.diagnostics.clone();
            }
        }

        #[cfg(test)]
        self.recompute_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let diagnostics = compute();
        self.cache.write().await.insert(
            uri.to_string(),
            CachedAnalysis { content_hash: hash, diagnostics: diagnostics.clone() },
        );
        diagnostics
    }

    /// Invalidate `uri`'s cached analysis without touching its stored text — used for
    /// cross-file invalidation: when a document a cached one `leta`s changes, the cached one's
    /// diagnostics may now be stale (a newly-broken or newly-fixed cross-file call) even though
    /// its own text didn't change, so its hash-based cache entry must be dropped rather than
    /// trusted on the next lookup.
    pub async fn invalidate(&self, uri: &str) {
        self.cache.write().await.remove(uri);
    }
}

impl Default for DocStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn diagnostics_for_computes_on_first_call() {
        let store = DocStore::new();
        let diags = store.diagnostics_for("file:///a.as", "text v1", Vec::new).await;
        assert_eq!(diags, Vec::<Diagnostic>::new());
        assert_eq!(store.recompute_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn diagnostics_for_reuses_cache_on_identical_text() {
        let store = DocStore::new();
        let _ = store.diagnostics_for("file:///a.as", "same text", Vec::new).await;
        let _ = store.diagnostics_for("file:///a.as", "same text", || {
            panic!("must not recompute on a hash match")
        }).await;
        assert_eq!(store.recompute_count.load(Ordering::SeqCst), 1, "second identical-text call must be a cache hit, not a second recompute");
    }

    #[tokio::test]
    async fn diagnostics_for_recomputes_on_changed_text() {
        let store = DocStore::new();
        let _ = store.diagnostics_for("file:///a.as", "v1", Vec::new).await;
        let _ = store.diagnostics_for("file:///a.as", "v2", Vec::new).await;
        assert_eq!(store.recompute_count.load(Ordering::SeqCst), 2, "different text must genuinely recompute, not reuse v1's cache entry");
    }

    #[tokio::test]
    async fn invalidate_forces_recompute_even_with_identical_text() {
        let store = DocStore::new();
        let _ = store.diagnostics_for("file:///a.as", "same text", Vec::new).await;
        store.invalidate("file:///a.as").await;
        let _ = store.diagnostics_for("file:///a.as", "same text", Vec::new).await;
        assert_eq!(store.recompute_count.load(Ordering::SeqCst), 2, "an explicit invalidate must force recomputation on the next call, even with unchanged text");
    }

    #[tokio::test]
    async fn remove_clears_both_text_and_cache() {
        let store = DocStore::new();
        store.insert("file:///a.as".to_string(), "text".to_string()).await;
        let _ = store.diagnostics_for("file:///a.as", "text", Vec::new).await;
        store.remove("file:///a.as").await;

        assert_eq!(store.get("file:///a.as").await, None);
        // After remove, the cache entry is gone too -- the next diagnostics_for call for the
        // same URI must recompute, not find a stale leftover entry.
        let _ = store.diagnostics_for("file:///a.as", "text", Vec::new).await;
        assert_eq!(store.recompute_count.load(Ordering::SeqCst), 2, "remove() must clear the cache entry, not just the text");
    }

    #[tokio::test]
    async fn different_uris_have_independent_cache_entries() {
        let store = DocStore::new();
        let _ = store.diagnostics_for("file:///a.as", "same text", Vec::new).await;
        let _ = store.diagnostics_for("file:///b.as", "same text", Vec::new).await;
        assert_eq!(store.recompute_count.load(Ordering::SeqCst), 2, "two different URIs with identical text must not share a cache entry");
    }
}
