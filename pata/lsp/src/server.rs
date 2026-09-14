//! LSP server: Backend struct and stdio entry points.

use std::collections::HashMap;
use std::path::PathBuf;
use tower_lsp::{Client, LspService, Server};
use tokio::sync::RwLock;
use crate::doc_store::DocStore;
use crate::workspace::WorkspaceIndex;

pub struct Backend {
    pub client: Client,
    pub documents: DocStore,
    /// One resolved cross-file index (see `crate::workspace`) per discovered project root —
    /// i.e. per directory containing a `pata.toml` that some open/touched file lives under.
    /// Empty until the first file needing it is resolved; a file under no discoverable project
    /// simply never gets an entry, and every consumer treats that as "degrade to single-file
    /// behavior", not an error. Resolution is per-file (`Backend::workspace_for`), not
    /// per-VS-Code-workspace-folder, since one opened folder can contain several independent
    /// Asili projects with no `pata.toml` at its own root (see `crate::workspace`'s module doc).
    pub workspaces: RwLock<HashMap<PathBuf, WorkspaceIndex>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DocStore::new(),
            workspaces: RwLock::new(HashMap::new()),
        }
    }

    /// Resolve (and cache) the project a given file belongs to, by walking up to its nearest
    /// `pata.toml`. Returns `None` for a file under no discoverable project — callers should
    /// treat that as "no cross-file information available", not an error. Cheap on a cache hit;
    /// does real disk I/O via `spawn_blocking` on a miss.
    pub async fn workspace_for(&self, file_path: &std::path::Path) -> Option<WorkspaceIndex> {
        let root = crate::workspace::find_project_root(file_path)?;
        if let Some(index) = self.workspaces.read().await.get(&root) {
            return Some(index.clone());
        }
        let root_for_blocking = root.clone();
        let index = tokio::task::spawn_blocking(move || crate::workspace::resolve_workspace(&root_for_blocking))
            .await
            .ok()?;
        self.workspaces.write().await.insert(root, index.clone());
        Some(index)
    }
}

/// Run the Mwalimu LSP server over stdio (async). Use from the `pata-lsp` binary.
pub async fn run_stdio() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Run the LSP server over stdio, blocking the current thread. Use from `pata mwalimu`.
pub fn run_stdio_blocking() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(run_stdio());
}
