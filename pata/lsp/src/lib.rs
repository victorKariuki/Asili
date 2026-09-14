//! Mwalimu — Asili Language Server.
//! Library entry point: modules and LanguageServer impl.
//!
//! Implemented capabilities:
//!   - text_document_sync: full-document sync on open/change/close (did_close clears both the
//!     in-memory document and its last-published diagnostics)
//!   - hover: keyword + symbol type information
//!   - diagnostics: lex, parse, semantic, and lint errors published on change — cross-file
//!     aware for project-local `leta` targets via the resolved workspace (see `workspace`),
//!     re-published for every open document whenever a watched file changes on disk
//!   - semantic_tokens: keyword/type/function/variable/parameter/property/enumMember token
//!     types, with declaration/readonly/defaultLibrary modifiers
//!   - document_formatting: AST-aware formatting via pata-fmt
//!   - code_action: quick-fixes for diagnostics the client reports back (currently: "Add doc
//!     comment" for LINT202)
//!   - code_lens: "▶ Run Test" above every `#[jaribio]` function (paired with the
//!     `asili.runTest` command in the VS Code extension's `extension.ts`)
//!   - folding_range: every multi-line `{ ... }` block, computed by bracket-matching on the
//!     raw source (see `symbols::folding_ranges`)
//!   - signature_help: parameter hints while typing a call — real parameter names for
//!     user-defined and resolved cross-file functions, type-only for builtins (`FnContract`
//!     carries no parameter names)
//!   - completion: keywords, builtin types, builtin functions, module-level declarations
//!   - definition: jump to declaration for functions/structs/traits/enums/constants
//!   - references: token occurrences of a symbol in the current file, extended to every
//!     module that transitively imports it (`WorkspaceIndex::transitive_importers`) when the
//!     symbol is an exported declaration
//!   - document_highlight: references scoped to the current file only, for "highlight
//!     occurrences" without opening the references panel
//!   - document_symbol: file outline (functions, structs, traits, enums, constants)
//!   - workspace_symbol: searches both open documents (their live buffer) and every other
//!     project-local file the resolved workspace knows about, so a struct/function in a file
//!     you haven't opened as a tab yet is still found
//!   - prepare_rename / rename: validates the cursor is on a real, renameable symbol before
//!     the client shows the rename UI, then renames all occurrences in the open document
//!   - inlay_hint: inferred types on un-annotated `weka`/`thabiti` bindings, via
//!     `SemanticAnalyzer::inlay_type_hints` (the same scope-resolution walk that drives
//!     semantic_tokens)
//!
//! Incremental re-resolution is deliberately not a full salsa-style per-file recomputation
//! framework (`docs/design/pata-implementation-spec.md` Section 19's own "Decision made":
//! pulling in `salsa` and restructuring around query-based recomputation is a rewrite, not an
//! incremental improvement). Two real, scoped wins instead:
//!   - `did_change_watched_files` invalidates only the specific project root(s) the changed
//!     files actually belong to (`workspace::affected_project_roots`), not the entire cache — an
//!     unrelated sibling project under the same VS Code workspace folder stays a cache hit.
//!   - `DocStore::diagnostics_for` caches each document's last-computed diagnostics keyed by a
//!     content hash: a `didChange` whose new text hashes identically to what's cached (a real
//!     case some editors fire — a no-op edit event, e.g. a purely-cursor-movement change) skips
//!     re-lex/re-parse/re-analyze entirely instead of recomputing on every keystroke regardless
//!     of whether anything actually changed.
//! What's still missing is finer-grained *within* one project root on a genuine content change:
//! a cache miss still re-walks that whole project's import graph from its entrypoint — there's
//! no API to cheaply re-resolve just the one file that changed within an already-resolved graph
//! (true per-file salsa-style recomputation). See `crate::workspace`'s module doc for the
//! cross-file model this is all built on and its known limitations (path-based imports plus
//! registry-resolved vendored dependencies via `pata_core::find_module_file`; still no
//! `pata.lock`-driven version-constraint awareness).

mod diagnostics;
mod doc_store;
mod format;
pub mod hover;
mod hover_format;
pub mod inlay_hints;
pub mod semantic;
mod server;
pub mod signature;
pub mod symbols;
pub mod types;
pub mod workspace;
pub mod actions;

pub use server::{run_stdio, run_stdio_blocking};
pub use tower_lsp;

use diagnostics::{asili_diagnostics_to_lsp_with_source, run_lex_parse};
use server::Backend;
use tower_lsp::{
    lsp_types::{
        CodeActionOrCommand, CodeActionParams,
        CodeActionProviderCapability, CodeActionResponse,
        CodeLens, CodeLensOptions, CodeLensParams,
        CompletionOptions, CompletionParams, CompletionResponse,
        DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams,
        DocumentSymbolParams, DocumentSymbolResponse,
        DidChangeWatchedFilesParams, DidChangeWatchedFilesRegistrationOptions,
        FileSystemWatcher, GlobPattern, Registration,
        FoldingRange, FoldingRangeParams, FoldingRangeProviderCapability,
        GotoDefinitionParams, GotoDefinitionResponse,
        InlayHintParams,
        InitializeParams, InitializeResult, InitializedParams,
        OneOf,
        PrepareRenameResponse,
        ReferenceParams,
        RenameOptions,
        RenameParams, TextDocumentPositionParams, WorkspaceEdit,
        SemanticTokenModifier, SemanticTokenType,
        SemanticTokensFullOptions, SemanticTokensLegend,
        SemanticTokensOptions, SemanticTokensServerCapabilities,
        ServerCapabilities, ServerInfo,
        ParameterInformation, ParameterLabel,
        SignatureHelp, SignatureHelpOptions, SignatureHelpParams, SignatureInformation,
        TextDocumentSyncCapability, TextDocumentSyncKind,
        WorkspaceSymbolParams,
        WorkspaceServerCapabilities, WorkspaceFoldersServerCapabilities,
    },
    LanguageServer,
};

/// `.asi` files are interface *stubs* — bare signatures like `kazi chapisha(ujumbe: Neno) -> Tupu`
/// with no body, meant to be read by `InterfaceRegistry`'s own lenient line-based reader
/// (`pata/cli/src/pipeline/interface_registry.rs::parse_asi_content`), not the full `.as`
/// grammar. The extension registers `.asi` under the same "asili" language as `.as` (for
/// consistent syntax highlighting), which means every `.asi` file reaches this server too — but
/// running full lex/parse/semantic diagnostics against one always fails (a real `kazi ... -> T`
/// with no `{ }` is a parse error in the full grammar) and would show a permanent, unfixable
/// "block inahitaji '{'" error on every interface stub in the project. Diagnostics are the only
/// LSP feature that surfaces this loudly (hover/semantic-tokens/etc. already degrade silently
/// on unparseable content), so this is the one place that needs to know about the distinction.
fn is_interface_stub(uri: &str) -> bool {
    uri.ends_with(".asi")
}

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
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                retrigger_characters: None,
                work_done_progress_options: Default::default(),
            }),
            code_lens_provider: Some(CodeLensOptions { resolve_provider: Some(false) }),
            // New capabilities
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![".".to_string(), " ".to_string()]),
                ..Default::default()
            }),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            document_highlight_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: Default::default(),
            })),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: None,
                }),
                file_operations: None,
            }),
            inlay_hint_provider: Some(OneOf::Left(true)),
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

    /// Fires once the client has processed `initialize`'s result. Project resolution itself is
    /// lazy and per-file (`Backend::workspace_for`, triggered from `did_open`/`did_change`/etc.)
    /// rather than eager here, since the workspace *folder* isn't necessarily a project root at
    /// all (see `crate::workspace`'s module doc) — there's nothing useful to eagerly resolve
    /// against just yet. This is only where the watched-files registration happens.
    async fn initialized(&self, _params: InitializedParams) {
        // Dynamic registration so external changes (a file edited outside VS Code, `git pull`,
        // a formatter running elsewhere) also invalidate the resolved workspace — without
        // this, the only trigger for re-resolving is an edit made *through* this editor
        // session. Best-effort: some clients don't support dynamic registration for this, and
        // there's nothing useful to do about that beyond staying resolved-at-startup, so
        // failure here is silently ignored rather than surfaced as an error.
        let watchers = vec!["**/*.as", "**/pata.toml", "**/pata.lock"]
            .into_iter()
            .map(|pattern| FileSystemWatcher {
                glob_pattern: GlobPattern::String(pattern.to_string()),
                kind: None,
            })
            .collect();
        if let Ok(register_options) = serde_json::to_value(DidChangeWatchedFilesRegistrationOptions { watchers }) {
            let _ = self
                .client
                .register_capability(vec![Registration {
                    id: "asili-watch-project-files".to_string(),
                    method: "workspace/didChangeWatchedFiles".to_string(),
                    register_options: Some(register_options),
                }])
                .await;
        }
    }

    /// Fires on a change to any file matching the watchers registered in `initialized` above.
    /// Incremental: `params.changes` names exactly which files changed, so only the project
    /// root(s) those files actually belong to (via `find_project_root`) are dropped from the
    /// cache — every other cached project (an unrelated sibling under the same VS Code
    /// workspace folder, or a project nobody's editing right now) is left untouched and stays a
    /// cache hit on the next `workspace_for` call. Previously this cleared the *entire* cache on
    /// any watched-file event anywhere, forcing a full re-resolution of every open document's
    /// project regardless of whether that project was actually affected — the real cost the
    /// production-readiness doc's "no incremental re-resolution" gap named. A changed file with
    /// no discoverable project root (outside any known `pata.toml`) contributes nothing to
    /// invalidate, which is correct: nothing cached could depend on it. Then re-publish
    /// diagnostics for every currently open document (each against its own project, if any), so
    /// e.g. a sibling file edited outside the editor that broke (or fixed) a cross-file call is
    /// reflected immediately rather than waiting for the next edit in an open file.
    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let changed_paths: Vec<std::path::PathBuf> = params
            .changes
            .iter()
            .filter_map(|change| change.uri.to_file_path().ok())
            .collect();
        let affected_roots = crate::workspace::affected_project_roots(&changed_paths);

        if !affected_roots.is_empty() {
            let mut workspaces = self.workspaces.write().await;
            for root in &affected_roots {
                workspaces.remove(root);
            }
        }

        for (uri_str, text) in self.documents.all().await {
            if is_interface_stub(&uri_str) {
                continue;
            }
            let file_path = uri_str
                .parse::<tower_lsp::lsp_types::Url>()
                .ok()
                .and_then(|u| u.to_file_path().ok());
            let workspace = match &file_path {
                Some(path) => self.workspace_for(path).await,
                None => None,
            };
            // The document's own text may be unchanged, but an external change to a *different*
            // file it `leta`s can still stale its diagnostics (a cross-file call that just broke
            // or got fixed) — drop any cached entry from before this event so diagnostics_for
            // below genuinely recomputes rather than serving a same-text cache hit that predates
            // the external change, and so it caches the fresh result for the next did_change.
            self.documents.invalidate(&uri_str).await;
            let lsp_diags = self
                .documents
                .diagnostics_for(&uri_str, &text, || {
                    let diags = run_lex_parse(&text, workspace.as_ref(), file_path.as_deref());
                    asili_diagnostics_to_lsp_with_source(&diags, &text)
                })
                .await;
            if let Ok(uri) = uri_str.parse() {
                let _ = self.client.publish_diagnostics(uri, lsp_diags, None).await;
            }
        }
    }

    // ── Document sync ──────────────────────────────────────────────────────────

    async fn did_open(&self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let uri_str = uri.to_string();
        let text = params.text_document.text;
        self.documents.insert(uri_str.clone(), text.clone()).await;
        let lsp_diags = if is_interface_stub(&uri_str) {
            vec![]
        } else {
            let file_path = uri.to_file_path().ok();
            let workspace = match &file_path {
                Some(path) => self.workspace_for(path).await,
                None => None,
            };
            // Populates the diagnostics cache too (not just computing once and discarding) so
            // an immediate no-op didChange right after open — some clients fire one — is a real
            // cache hit rather than a second identical recomputation.
            self.documents
                .diagnostics_for(&uri_str, &text, || {
                    let diags = run_lex_parse(&text, workspace.as_ref(), file_path.as_deref());
                    asili_diagnostics_to_lsp_with_source(&diags, &text)
                })
                .await
        };
        let _ = self.client.publish_diagnostics(uri, lsp_diags, None).await;
    }

    /// Incremental per Section 19 of `pata-implementation-spec.md`: `DocStore::diagnostics_for`
    /// skips re-lex/re-parse/re-analyze entirely when the incoming text hashes identically to
    /// what's already cached for this URI — a real case some editors hit (a no-op edit event,
    /// e.g. purely a cursor move some clients still fire `didChange` for). Cross-file
    /// invalidation (a change to module A stales cached diagnostics for every B that `leta`s A)
    /// is handled separately in `did_change_watched_files`/the resolved workspace's
    /// `reverse_deps`, not here — an in-editor edit to the *currently open* document doesn't by
    /// itself invalidate any other document's cache entry until that edit is saved to disk and a
    /// watcher event fires (this mirrors the pre-existing behavior for cross-file diagnostics
    /// generally: another open document only sees the *saved* state of its imports, not
    /// keystroke-by-keystroke edits in a different tab).
    async fn did_change(&self, params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let uri_str = uri.to_string();
        let text = match params.content_changes.first() {
            Some(change) => change.text.clone(),
            None => return,
        };
        self.documents.insert(uri_str.clone(), text.clone()).await;
        let lsp_diags = if is_interface_stub(&uri_str) {
            vec![]
        } else {
            let file_path = uri.to_file_path().ok();
            let workspace = match &file_path {
                Some(path) => self.workspace_for(path).await,
                None => None,
            };
            self.documents
                .diagnostics_for(&uri_str, &text, || {
                    let diags = run_lex_parse(&text, workspace.as_ref(), file_path.as_deref());
                    asili_diagnostics_to_lsp_with_source(&diags, &text)
                })
                .await
        };
        let _ = self.client.publish_diagnostics(uri, lsp_diags, None).await;
    }

    async fn did_close(&self, params: tower_lsp::lsp_types::DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(uri.as_str()).await;
        // Clear whatever diagnostics were last published for this file — otherwise closing a
        // file with warnings/errors leaves them in the Problems panel forever, since nothing
        // else ever republishes an empty list for a URI the client no longer has open.
        let _ = self.client.publish_diagnostics(uri, vec![], None).await;
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

    // ── Signature help ─────────────────────────────────────────────────────────

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> tower_lsp::jsonrpc::Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let workspace = match uri.to_file_path() {
            Ok(path) => self.workspace_for(&path).await,
            Err(()) => None,
        };
        let Some(info) = signature::compute_signature_help(&text, pos.line, pos.character, workspace.as_ref()) else {
            return Ok(None);
        };
        let parameters = info
            .params
            .iter()
            .map(|p| ParameterInformation {
                label: ParameterLabel::Simple(p.clone()),
                documentation: None,
            })
            .collect();
        let active_parameter = if info.params.is_empty() {
            None
        } else {
            Some(info.active_param.min(info.params.len() - 1) as u32)
        };
        Ok(Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: info.label,
                documentation: None,
                parameters: Some(parameters),
                active_parameter: None,
            }],
            active_signature: Some(0),
            active_parameter,
        }))
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
        let file_path = uri.to_file_path().ok();
        Ok(format::format_to_edits(&text, file_path.as_deref()))
    }

    // ── Code actions ───────────────────────────────────────────────────────────
    // The client sends back whatever diagnostics from our own publish_diagnostics overlap the
    // requested range/selection (`params.context.diagnostics`) — no need to re-run lint
    // ourselves. The actual diagnostic→edit logic lives in `actions::action_for_diagnostic`
    // (unit-tested there against real lint-rule message text); this handler just fans it out
    // over the diagnostics the client handed back.

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.clone();
        let actions: Vec<CodeActionOrCommand> = params
            .context
            .diagnostics
            .iter()
            .filter_map(|diag| actions::action_for_diagnostic(diag, &uri))
            .map(CodeActionOrCommand::CodeAction)
            .collect();

        Ok(if actions.is_empty() { None } else { Some(actions) })
    }

    // ── Code lens ──────────────────────────────────────────────────────────────

    async fn code_lens(
        &self,
        params: CodeLensParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let lenses = symbols::test_code_lenses(&text, &uri);
        Ok(if lenses.is_empty() { None } else { Some(lenses) })
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
    // Cross-file (beyond this handler's own single-file search): only pursued when the word
    // under the cursor is actually an exported declaration in the current file — a purely
    // local variable can't legitimately be referenced from another module, so there's nothing
    // to gain from searching one. When it is, search every module that *transitively* imports
    // this one (`WorkspaceIndex::transitive_importers`) rather than every file in the project —
    // correct, since nothing outside that set could resolve a `leta` to this module at all.

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
        let mut locs = symbols::find_references(&text, &uri, pos.line, pos.character, include_decl);

        if let Some(word) = symbols::word_at(&text, pos.line, pos.character) {
            if let Some(module) = symbols::parse_module(&text) {
                if symbols::is_exported_declaration(&module, &word) {
                    let cur_path = uri.to_file_path().ok();
                    if let Some(cur_name) = cur_path
                        .as_deref()
                        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                    {
                        let workspace = match cur_path.as_deref() {
                            Some(p) => self.workspace_for(p).await,
                            None => None,
                        };
                        if let Some(ws) = workspace.as_ref() {
                            for importer_name in ws.transitive_importers(&cur_name) {
                                let Some(wm) = ws.find(&importer_name) else { continue };
                                let Ok(other_uri) = tower_lsp::lsp_types::Url::from_file_path(&wm.path) else { continue };
                                if other_uri == uri {
                                    continue; // already covered by find_references above
                                }
                                let other_text = match self.documents.get(other_uri.as_str()).await {
                                    Some(t) => t,
                                    None => match tokio::fs::read_to_string(&wm.path).await {
                                        Ok(t) => t,
                                        Err(_) => continue,
                                    },
                                };
                                locs.extend(symbols::find_references_in(&word, &other_uri, &other_text));
                            }
                        }
                    }
                }
            }
        }

        Ok(if locs.is_empty() { None } else { Some(locs) })
    }

    // ── Document highlight ─────────────────────────────────────────────────────
    // Lighter-weight than references: highlight other occurrences of the symbol under the
    // cursor in the *current* file only, without opening the references panel. find_references
    // already does exactly this single-file token search — reuse it directly, always with
    // include_declaration: true (the declaration site should highlight too, same as every
    // other editor's "highlight occurrences" behavior).

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        let uri = params.text_document_position_params.text_document.uri.clone();
        let pos = params.text_document_position_params.position;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let locs = symbols::find_references(&text, &uri, pos.line, pos.character, true);
        if locs.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            locs.into_iter()
                .map(|l| DocumentHighlight {
                    range: l.range,
                    kind: Some(DocumentHighlightKind::TEXT),
                })
                .collect(),
        ))
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
    // Merges two sources: open documents (their live, possibly-unsaved buffer) and every
    // project those open documents' files belong to (every other project-local file in it,
    // parsed from disk at the last resolution) — without the latter, searching for a
    // struct/function in a file you haven't opened as a tab yet would silently find nothing.
    // Note this only reaches projects reachable from an *already-open* file — a sibling Asili
    // project nobody has opened anything from yet (e.g. a different `examples/*` folder in
    // this repo) isn't discovered by a bare directory walk here, only by the same per-file
    // resolution every other feature uses.

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<tower_lsp::lsp_types::SymbolInformation>>> {
        let all_docs = self.documents.all().await;
        let open_paths: std::collections::HashSet<std::path::PathBuf> = all_docs
            .iter()
            .filter_map(|(uri, _)| uri.parse::<tower_lsp::lsp_types::Url>().ok())
            .filter_map(|uri| uri.to_file_path().ok())
            .collect();

        let mut results = symbols::workspace_symbols(all_docs.into_iter(), &params.query);
        let mut seen_roots = std::collections::HashSet::new();
        for path in &open_paths {
            let Some(root) = crate::workspace::find_project_root(path) else { continue };
            if !seen_roots.insert(root) {
                continue;
            }
            if let Some(ws) = self.workspace_for(path).await {
                results.extend(symbols::workspace_symbols_from_index(&ws, &params.query, &open_paths));
            }
        }
        Ok(if results.is_empty() { None } else { Some(results) })
    }

    // ── Folding ranges ─────────────────────────────────────────────────────────

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let ranges = symbols::folding_ranges(&text);
        Ok(if ranges.is_empty() { None } else { Some(ranges) })
    }

    // ── Rename ─────────────────────────────────────────────────────────────────
    // Validates the cursor is actually on a renameable symbol *before* the client shows the
    // rename UI — without this, `rename` below just silently no-ops on a bad position (a
    // keyword, a number, empty space) and the user has no idea why nothing happened.

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let pos = params.position;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let Some((word, range)) = symbols::word_at_range(&text, pos.line, pos.character) else {
            return Ok(None);
        };
        if !symbols::can_rename(&word) {
            return Ok(None);
        }
        Ok(Some(PrepareRenameResponse::Range(range)))
    }

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

    // ── Inlay hints ────────────────────────────────────────────────────────────

    async fn inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<tower_lsp::lsp_types::InlayHint>>> {
        let uri = params.text_document.uri;
        let text = match self.documents.get(uri.as_str()).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let hints = inlay_hints::compute_inlay_hints(&text);
        Ok(if hints.is_empty() { None } else { Some(hints) })
    }
}
