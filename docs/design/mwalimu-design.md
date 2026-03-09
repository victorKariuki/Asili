# Mwalimu (LSP) design

Mwalimu is the Asili Language Server, implemented in the [pata/lsp](../../pata/lsp/) crate.

## Architecture

- **Transport:** stdio (stdin/stdout). The editor spawns the server and communicates via JSON-RPC over stdio.
- **Libraries:** Uses `tower-lsp` and re-exports from `tower_lsp::lsp_types`. Depends on `asili_lexer`, `asili_parser`, and `asili_diagnostics`; no evaluator dependency for diagnostics/hover.
- **Document store:** In-memory map of URI → full text. On `didOpen` and `didChange` we update the store and re-run lex/parse (and optional semantic) to publish diagnostics.
- **Diagnostics:** Lex and parse (and optionally semantic check) run on each change. Asili `Diagnostic` (from parser/diagnostics) is mapped to LSP `Diagnostic` (range, message, severity, code, source).
- **Hover:** On hover we resolve the position to a token (from lexer) and optionally to a function name (from parsed module). We return a short markdown string (keyword, function, or identifier).

## Entry points

- **Binary:** `pata-lsp` (crate binary) runs the server via `run_stdio().await`.
- **CLI:** `pata mwalimu` calls `pata_lsp::run_stdio_blocking()` so the same server runs in-process.

## Future work

- Split into modules (server, diagnostics, hover, doc_store) for easier addition of completion, goto-definition, references, document symbols.
- Optional dependency on an analysis crate for symbol table and type-of-node when adding goto-def or completion.
- Format-on-save would require the LSP to call a shared format API (e.g. from an extracted format crate).

See [spec/06-tooling-and-ecosystem.md](../../spec/06-tooling-and-ecosystem.md) and [docs/nuru-comparison.md](../nuru-comparison.md) (LSP structure).
