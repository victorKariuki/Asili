//! Mwalimu — Asili Language Server (Phase II).
//! Library entry point: modules and LanguageServer impl.
//
// TODO(Phase II): Implement remaining LSP capabilities:
//   - completion_provider: keyword list + module-level function/struct names
//   - definition_provider: map Ident tokens to their declaration Span
//   - references_provider: find all usages of a symbol across open files
//   - document_symbol_provider: outline of functions/structs/traits in the file
//   - rename_provider: rename a symbol across all files in the workspace
//   - document_formatting_provider: call pipeline::format::canonical_format on the document
//   - semantic_tokens_provider: highlight keywords, types, functions with distinct token types
//   - workspace_symbol_provider: cross-file symbol search
// Each requires registering the capability in initialize() ServerCapabilities.

mod diagnostics;
mod doc_store;
mod hover;
mod server;

pub use server::{run_stdio, run_stdio_blocking};

use diagnostics::{asili_diagnostics_to_lsp, run_lex_parse};
use server::Backend;
use tower_lsp::{
    lsp_types::{
        InitializeParams, InitializeResult, ServerCapabilities, ServerInfo,
        TextDocumentSyncCapability, TextDocumentSyncKind,
    },
    LanguageServer,
};

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            ..ServerCapabilities::default()
        };
        Ok(InitializeResult {
            capabilities,
            server_info: Some(ServerInfo {
                name: "mwalimu".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = params.text_document.text;
        self.documents.insert(uri.clone(), text.clone()).await;
        let diags = run_lex_parse(&text);
        let lsp_diags = asili_diagnostics_to_lsp(&diags);
        let _ = self.client.publish_diagnostics(uri.parse().unwrap(), lsp_diags, None).await;
    }

    async fn did_change(&self, params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = match params.content_changes.first() {
            Some(change) => change.text.clone(),
            None => return,
        };
        self.documents.insert(uri.clone(), text.clone()).await;
        // TODO(Phase II): Also run semantic analysis here and publish semantic diagnostics
        // (type errors, unknown variables, unused Tokeo, etc.) in addition to lex/parse errors.
        let diags = run_lex_parse(&text);
        let lsp_diags = asili_diagnostics_to_lsp(&diags);
        let _ = self.client.publish_diagnostics(uri.parse().unwrap(), lsp_diags, None).await;
    }

    async fn hover(&self, params: tower_lsp::lsp_types::HoverParams) -> tower_lsp::jsonrpc::Result<Option<tower_lsp::lsp_types::Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        Ok(hover::compute_hover(&text, pos.line, pos.character))
    }
}
