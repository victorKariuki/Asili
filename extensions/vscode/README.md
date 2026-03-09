# Asili VS Code Extension

Syntax highlighting and LSP (Mwalimu) for Asili (`.as`, `.asi`).

## Install from repo

1. Open the repo in VS Code.
2. Open `extensions/vscode` in the workspace.
3. Run **Run Extension** from the Debug panel (F5), or package a vsix: `vsce package` (requires `npm i -g @vscode/vsce`).

## LSP configuration

Ensure `pata` (or `pata-lsp`) is on your PATH. The extension starts the language server with:

- **Command:** `pata` (or value of `asili.lsp.serverPath`)
- **Args:** `["mwalimu"]` (or `asili.lsp.serverArgs`)

If you use the standalone `pata-lsp` binary, set `asili.lsp.serverPath` to `pata-lsp` and `asili.lsp.serverArgs` to `[]`.

## Features

- Syntax highlighting (keywords, strings, numbers, comments)
- Diagnostics and hover via Mwalimu LSP

See [docs/howto/02-use-lsp.md](../../docs/howto/02-use-lsp.md) for editor setup.
