//! LSP server: Backend struct and stdio entry points.

use tower_lsp::{Client, LspService, Server};
use crate::doc_store::DocStore;

pub struct Backend {
    pub client: Client,
    pub documents: DocStore,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DocStore::new(),
        }
    }
}

/// Run the Mwalimu LSP server over stdio (async). Use from the `pata-lsp` binary.
pub async fn run_stdio() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend::new(client));
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
