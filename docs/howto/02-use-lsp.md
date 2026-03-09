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

- **Diagnostics** — Lex, parse, and semantic errors are published as you type. Messages are Swahili-centric and include error codes.
- **Hover** — Hover over identifiers and keywords for short documentation (e.g. keyword name, function name).

Goto-definition and completion may be added in a later phase.

## Spec reference

See [spec/06-tooling-and-ecosystem.md](../spec/06-tooling-and-ecosystem.md) (Mwalimu section) and [pata/lsp/README.md](../../pata/lsp/README.md).
