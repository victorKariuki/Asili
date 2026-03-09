# Mwalimu (pata-lsp)

Asili Language Server — Phase II deliverable. Provides LSP features for `.as` files over stdio.

## Running

- **From the workspace:** `cargo run -p pata-lsp` (stdio).
- **Via Pata CLI:** From an Asili project directory, `pata mwalimu` runs the same server (uses this crate as a library).

Editors that support LSP can start the server with command `pata` and arguments `["mwalimu"]`, or command `pata-lsp`, using stdio transport.

## Features

- **textDocument/didOpen**, **textDocument/didChange**: runs lex, parse, and semantic check; publishes diagnostics.
- **textDocument/hover**: returns short markdown for keywords, function names, and identifiers.

Uses `asili_lexer`, `asili_parser`, and `asili_diagnostics`; no evaluator dependency for diagnostics/hover.
