# Mwalimu (LSP) design

Mwalimu is the Asili Language Server, implemented in the [pata/lsp](../../pata/lsp/) crate.

## Architecture

- **Transport:** stdio (stdin/stdout). The editor spawns the server and communicates via JSON-RPC over stdio.
- **Libraries:** Uses `tower-lsp` and re-exports from `tower_lsp::lsp_types`. Depends on `asili_lexer`, `asili_parser`, `asili_diagnostics`, and `asili_evaluator` (added for richer hover/diagnostic type information).
- **Module layout:** split by concern, not one file — `server.rs` (protocol glue), `diagnostics.rs`,
  `hover.rs`/`hover_format.rs`, `doc_store.rs` (in-memory URI → text map), `semantic.rs`,
  `symbols.rs` (document/workspace symbols, goto-definition, find-references, rename),
  `signature.rs` (signature help), `types.rs`, `format.rs`, `workspace.rs` (cross-file/workspace
  indexing). `lib.rs` wires it together.
- **Document store:** In-memory map of URI → full text. On `didOpen`/`didChange` we update the
  store and re-run lex/parse (and semantic/lint) to publish diagnostics.
- **Diagnostics:** Lex, parse, semantic check, and lint run on each change. Asili `Diagnostic`
  (from parser/diagnostics) is mapped to LSP `Diagnostic` (range, message, severity, code, source).
- **Hover:** Resolves the position to a token and, where applicable, to semantic type information
  (variable/function/struct/trait signatures, generic type expansion) via `semantic.rs`/`types.rs`,
  not just a bare keyword/identifier lookup.

## Features implemented

- Diagnostics (lex/parse/semantic/lint), semantic tokens for editor highlighting
- Hover with type signatures
- Document formatting
- Completion (keywords, builtin types/functions, module-level declarations)
- Goto-definition, find-references, rename (single-document)
- Document symbols (file outline) and workspace symbols (cross-document search)
- Signature help

See [docs/howto/02-use-lsp.md](../howto/02-use-lsp.md) for the user-facing feature list — keep
that doc and this one in sync when adding LSP features.

## Entry points

- **Binary:** `pata-lsp` (crate binary) runs the server via `run_stdio().await`.
- **CLI:** `pata mwalimu` calls `pata_lsp::run_stdio_blocking()` so the same server runs in-process.

## Not yet implemented

- DAP (debugging) — separate protocol/server entirely; see [dap-later.md](dap-later.md).
- Format-on-save wiring beyond the existing `format.rs` document-formatting request (this is
  largely an editor-config concern, not missing server functionality).

See [spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md) and
[implementation-status.md](implementation-status.md) for overall project status.
