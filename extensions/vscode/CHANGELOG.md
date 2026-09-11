# Changelog

All notable changes to the Asili VS Code extension are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0]

### Added

- **Marketplace icon** (`assets/Asili_Logo.png`) — previously missing from `package.json`, so
  the extension showed a generic placeholder icon.
- **Syntax highlighting for `pata.toml` and `pata.lock`** (new `pata-manifest` language +
  grammar) — these were plain, unstyled text before.
- **Semantic highlighting overhaul** (`pata-lsp`'s semantic token analyzer): every usage of a
  variable, parameter, or function now gets colored, not just its declaration; module-level
  constants, struct-literal field names, match-arm pattern bindings, and enum variants
  (`Some`/`Ok`/`Err`/custom, via a new `enumMember` token type) are all tracked; `thabiti`
  bindings get a `readonly` modifier, builtins get `defaultLibrary`.
- **`textDocument/codeAction`** — an "Add doc comment" quick-fix for the `LINT202` diagnostic.
- **`textDocument/codeLens`** — a "▶ Run Test" lens above every `#[jaribio]` function, backed
  by a new `asili.runTest` command (runs `pata jaribu --filter <name>` in an integrated
  terminal) and a new `asili.cliPath` setting.
- **`textDocument/foldingRange`**, **`textDocument/documentHighlight`**,
  **`textDocument/prepareRename`**, **`textDocument/signatureHelp`**.
- **Cross-file project awareness** — the language server now resolves a project's `pata.toml`
  and `leta`-imported sibling files, so:
  - a legitimate call into your own other `.as` file no longer false-positives as an undefined
    function or unknown module;
  - go-to-definition, hover, and signature help work for cross-file functions;
  - find-references extends to every file that (transitively) imports the module defining the
    symbol;
  - workspace symbol search covers the whole project, not just files already open as tabs;
  - the resolved project is invalidated and re-diagnosed on `pata.toml`/`.as`/`pata.lock`
    changes made outside the editor.

### Fixed

- `textDocument/didClose` wasn't implemented — closing a file left it resident in server memory
  and its diagnostics permanently stuck in the Problems panel.
- Every semantic token was rendered one line low: the analyzer passed the lexer's 1-based
  line/column straight through to the (0-based) LSP semantic-tokens protocol.
- The `LINT202` "lacks documentation comment" rule fired unconditionally on every function — it
  never actually checked for a preceding `#` comment.
- A redundant `onLanguage:asili` activation event, superseded by VS Code's automatic activation
  event generation from the language contribution.

## [0.1.0]

### Added

- Initial syntax highlighting (`.as`, `.asi`) and Mwalimu language server integration: hover,
  diagnostics, semantic tokens, completion, go-to-definition, find-references, document/
  workspace symbols, rename, formatting.
