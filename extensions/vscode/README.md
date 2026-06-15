![Asili Banner](assets/Asili_Banner.png)

<p align="center">
  <img src="assets/Asili_Logo.png" alt="Asili Logo" width="110" />
</p>

# Asili Language Support

Syntax highlighting and LSP (Mwalimu) for the Asili language (`.as`, `.asi`).

## Features

- Syntax highlighting — keywords, strings, numbers, comments
- Diagnostics and hover via the Mwalimu language server

## Setup

The extension starts `pata mwalimu` as the language server. Make sure `pata` is on your PATH, or set a custom path in workspace settings:

```json
{
  "asili.serverPath": "/path/to/pata-cli",
  "asili.serverArgs": ["mwalimu"]
}
```

Build `pata` from source:

```bash
cargo build -p pata-cli
```

See the [project README](../../README.md) for full build instructions.
