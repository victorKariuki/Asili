//! Mwalimu — Asili Language Server.
//! Library entry point: modules and LanguageServer impl.
//!
//! Implemented capabilities:
//!   - text_document_sync: full-document sync on open/change
//!   - hover: keyword + symbol type information
//!   - diagnostics: lex, parse, semantic, and lint errors published on change
//!   - semantic_tokens: keyword/type/function/variable/parameter/property token types
//!   - document_formatting: AST-aware formatting via pata-fmt
//!   - completion: keywords, builtin types, builtin functions, module-level declarations
//!   - definition: jump to declaration for functions/structs/traits/enums/constants
//!   - references: find all token occurrences of a symbol (with/without declaration)
//!   - document_symbol: file outline (functions, structs, traits, enums, constants)
//!   - workspace_symbol: cross-document symbol search by name prefix
//!   - rename: rename all occurrences of a symbol in the open document

mod diagnostics;
mod doc_store;
mod format;
pub mod hover;
mod hover_format;
pub mod semantic;
mod server;
pub mod symbols;
pub mod types;

pub use server::{run_stdio, run_stdio_blocking};
pub use tower_lsp;

use diagnostics::{asili_diagnostics_to_lsp_with_source, run_lex_parse};
use server::Backend;
use tower_lsp::{
    lsp_types::{
        CompletionOptions, CompletionParams, CompletionResponse,
        DocumentSymbolParams, DocumentSymbolResponse,
        GotoDefinitionParams, GotoDefinitionResponse,
        InitializeParams, InitializeResult,
        OneOf,
        ReferenceParams,
        RenameParams, WorkspaceEdit,
        SemanticTokenModifier, SemanticTokenType,
        SemanticTokensFullOptions, SemanticTokensLegend,
        SemanticTokensOptions, SemanticTokensServerCapabilities,
        ServerCapabilities, ServerInfo,
        TextDocumentSyncCapability, TextDocumentSyncKind,
        WorkspaceSymbolParams,
        WorkspaceServerCapabilities, WorkspaceFoldersServerCapabilities,
    },
    LanguageServer,
};

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        _params: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        let legend = SemanticTokensLegend {
            token_types: semantic::TOKEN_TYPES
                .iter()
                .map(|s| SemanticTokenType::new(s))
                .collect(),
            token_modifiers: semantic::TOKEN_MODIFIERS
                .iter()
                .map(|s| SemanticTokenModifier::new(s))
                .collect(),
        };
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::FULL,
            )),
            // Existing capabilities
            hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
            semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
                SemanticTokensOptions {
                    legend,
                    range: None,
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                    ..Default::default()
                },
            )),
            document_formatting_provider: Some(OneOf::Left(true)),
            // New capabilities
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![".".to_string(), " ".to_string()]),
                ..Default::default()
            }),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Left(true)),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: None,
                }),
                file_operations: None,
            }),
            ..ServerCapabilities::default()
        };
        Ok(InitializeResult {
            capabilities,
            server_info: Some(ServerInfo {
                name: "mwalimu".to_string(),
                version: Some("0.2.0".to_string()),
            }),
        })
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }

    // ── Document sync ──────────────────────────────────────────────────────────

    async fn did_open(&self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = params.text_document.text;
        self.documents.insert(uri.clone(), text.clone()).await;
        let diags = run_lex_parse(&text);
        let lsp_diags = asili_diagnostics_to_lsp_with_source(&diags, &text);
        let _ = self
            .client
            .publish_diagnostics(uri.parse().unwrap(), lsp_diags, None)
            .await;
    }

    async fn did_change(&self, params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = match params.content_changes.first() {
            Some(change) => change.text.clone(),
            None => return,
        };
        self.documents.insert(uri.clone(), text.clone()).await;
        let diags = run_lex_parse(&text);
        let lsp_diags = asili_diagnostics_to_lsp_with_source(&diags, &text);
        let _ = self
            .client
            .publish_diagnostics(uri.parse().unwrap(), lsp_diags, None)
            .await;
    }

    // ── Hover ──────────────────────────────────────────────────────────────────

    async fn hover(
        &self,
        params: tower_lsp::lsp_types::HoverParams,
    ) -> tower_lsp::jsonrpc::Result<Option<tower_lsp::lsp_types::Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        Ok(hover::compute_hover(&text, pos.line, pos.character))
    }

    // ── Semantic tokens ────────────────────────────────────────────────────────

    async fn semantic_tokens_full(
        &self,
        params: tower_lsp::lsp_types::SemanticTokensParams,
    ) -> tower_lsp::jsonrpc::Result<
        Option<tower_lsp::lsp_types::SemanticTokensResult>,
    > {
        let uri = params.text_document.uri;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let tokens = semantic::analyze_semantic_tokens(&text);
        Ok(tokens.map(tower_lsp::lsp_types::SemanticTokensResult::Tokens))
    }

    // ── Formatting ─────────────────────────────────────────────────────────────

    async fn formatting(
        &self,
        params: tower_lsp::lsp_types::DocumentFormattingParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<tower_lsp::lsp_types::TextEdit>>> {
        let uri = params.text_document.uri;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        Ok(format::format_to_edits(&text))
    }

    // ── Completion ─────────────────────────────────────────────────────────────

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let items = symbols::completion_items(&text);
        Ok(Some(CompletionResponse::Array(items)))
    }

    // ── Goto definition ────────────────────────────────────────────────────────

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri.clone();
        let pos = params.text_document_position_params.position;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let location = symbols::goto_definition(&text, &uri, pos.line, pos.character);
        Ok(location.map(GotoDefinitionResponse::Scalar))
    }

    // ── References ─────────────────────────────────────────────────────────────

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<tower_lsp::lsp_types::Location>>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let include_decl = params.context.include_declaration;
        let locs = symbols::find_references(&text, &uri, pos.line, pos.character, include_decl);
        Ok(if locs.is_empty() { None } else { Some(locs) })
    }

    // ── Document symbols (outline) ─────────────────────────────────────────────

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> tower_lsp::jsonrpc::Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let syms = symbols::document_symbols(&text);
        Ok(Some(DocumentSymbolResponse::Nested(syms)))
    }

    // ── Workspace symbols ──────────────────────────────────────────────────────

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<tower_lsp::lsp_types::SymbolInformation>>> {
        let all_docs = self.documents.all().await;
        let results = symbols::workspace_symbols(all_docs.into_iter(), &params.query);
        Ok(if results.is_empty() { None } else { Some(results) })
    }

    // ── Rename ─────────────────────────────────────────────────────────────────

    async fn rename(
        &self,
        params: RenameParams,
    ) -> tower_lsp::jsonrpc::Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let new_name = params.new_name;

        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };

        let word = match symbols::word_at(&text, pos.line, pos.character) {
            Some(w) => w,
            None => return Ok(None),
        };

        let ranges = symbols::rename_locations(&text, &word);
        if ranges.is_empty() {
            return Ok(None);
        }

        let edits: Vec<tower_lsp::lsp_types::TextEdit> = ranges
            .into_iter()
            .map(|r| tower_lsp::lsp_types::TextEdit {
                range: r,
                new_text: new_name.clone(),
            })
            .collect();

        let mut changes = std::collections::HashMap::new();
        changes.insert(uri, edits);

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }
}
