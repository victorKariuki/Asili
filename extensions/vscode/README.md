![Asili Banner](assets/Asili_Banner.png)

<p align="center">
  <img src="assets/Asili_Logo.png" alt="Asili Logo" width="110" />
</p>

# Asili Language Support

Syntax highlighting and full language server support (Mwalimu) for the Asili language
(`.as`, `.asi`), plus syntax highlighting for `pata.toml`/`pata.lock` project manifests.

## Features

### Highlighting

- Grammar-based syntax highlighting — keywords, strings, numbers, comments, attributes, operators.
- Semantic highlighting on top of that (needs the language server running): every *usage* of a
  variable, parameter, or function is colored, not just its declaration. Also covers module-level
  constants, struct-literal field names, match-arm pattern bindings, and enum variants
  (`Some`/`Ok`/`Err`/custom, via a dedicated token type distinct from real types). `thabiti`
  bindings render differently from `weka`, and builtins render differently from your own
  functions.
- Syntax highlighting for `pata.toml` and `pata.lock`.

### Editing

- Hover — type information for variables, parameters, functions, structs, and traits.
- Signature help — parameter hints while typing a call, with real parameter names for your own
  functions and cross-file calls into your own other project files.
- Completion — keywords, builtin types, builtin functions, module-level declarations.
- Code actions — quick-fixes for diagnostics (currently: "Add doc comment").
- Code lens — a "▶ Run Test" lens above every `#[jaribio]` function, running
  `pata jaribu --filter <name>` in an integrated terminal.
- Formatting, folding ranges, rename (with `prepareRename` validation), and "highlight
  occurrences" (document highlight).

### Navigation

- Go to definition, find references, document symbols (file outline), and workspace symbol
  search — all of these work **across files** in your project, not just the one you have open:
  a call into your own sibling `.as` file resolves correctly, and searching for a
  struct/function finds it even in a file you haven't opened as a tab yet.

### Diagnostics

- Lex, parse, semantic, and lint errors, published as you type.
- Cross-file aware: the language server resolves your project's `pata.toml` and `leta`-imported
  files, so a legitimate call into your own other project file is recognized instead of
  false-positiving as an undefined function or unknown module.
- Re-checked automatically when a `.as`/`pata.toml`/`pata.lock` file changes on disk — including
  edits made outside the editor.

## Setup

The extension starts `pata mwalimu` as the language server. Make sure `pata` is on your PATH, or set a custom path in workspace settings:

```json
{
  "asili.serverPath": "/path/to/pata-cli",
  "asili.serverArgs": ["mwalimu"]
}
```

The "Run Test" code lens shells out to the `pata` CLI separately from the language server (which
may be a bundled `pata-lsp` binary with no sibling `pata`) — set `asili.cliPath` if `pata` isn't
on your PATH:

```json
{
  "asili.cliPath": "/path/to/pata"
}
```

Cross-file features (diagnostics, go to definition, references, workspace symbols) need a
`pata.toml` at your workspace root to know what your project's files and dependencies are —
without one, everything still works for a single open file, just without the cross-file parts.

Build `pata` from source:

```bash
cargo build -p pata-cli
```

See the [project README](../../README.md) for full build instructions.
