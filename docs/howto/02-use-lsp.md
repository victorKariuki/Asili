# Using the LSP (Mwalimu)

The **Mwalimu** Language Server provides diagnostics and hover for `.as` files in editors that support the Language Server Protocol.

## Running the server

**From the workspace or any directory:**

- **Via Pata:** `pata mwalimu` — runs the LSP over stdio (blocking).
- **Standalone binary:** `pata-lsp` (after `cargo build -p pata-lsp`) — same behavior.

The server uses **stdin/stdout** for LSP communication. Do not run it in a terminal by hand; your editor starts it and talks to it over stdio.

## Editor configuration

Configure your editor to start the language server with one of:

| Command | Arguments |
|--------|------------|
| `pata` | `["mwalimu"]` |
| `pata-lsp` | (none) |

**Example (VS Code settings.json):** If `pata` or `pata-lsp` is on your PATH:

```json
{
  "asili-lsp.serverPath": "pata",
  "asili-lsp.serverArgs": ["mwalimu"]
}
```

Or use the **Asili VS Code extension** from the repo: open [extensions/vscode/](../../extensions/vscode/) in VS Code and run **Run Extension** (F5), or run `vsce package` to build a `.vsix` and install it.

## Features

- **Diagnostics** — Lex, parse, semantic, and lint errors published as you type with Swahili error codes.
- **Hover** — Hover over identifiers for type signatures; hover keywords for documentation.
- **Semantic tokens** — Token-type classifications (keyword, type, function, variable, parameter, property) for rich editor highlighting.
- **Document formatting** — Format the active `.as` file (e.g. Shift+Alt+F in VS Code).
- **Completion** — Keyword list, built-in types, built-in functions, and all module-level declarations (functions, structs, traits, constants).
- **Goto definition** — Jump to the declaration of any function, struct, trait, enum, or constant in the file.
- **Find references** — List all usages of a symbol across the file (with or without the declaration site).
- **Document symbols** — File outline showing all top-level declarations with their kinds and fields (visible in VS Code's breadcrumb and Outline panel).
- **Workspace symbols** — Search symbols by name prefix across all open documents.
- **Rename** — Rename all occurrences of a symbol in the open document.

## Spec reference

See [spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md) (Mwalimu section) and [pata/lsp/README.md](../../pata/lsp/README.md).
