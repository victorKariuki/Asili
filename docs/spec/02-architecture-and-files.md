# 2. Architecture and Files

Previous: [Philosophy and EDP](01-philosophy-and-edp.md) | [Overview](../SPECIFICATION.md) | Next: [Syntax](03-syntax.md)

---

## Project structure (Substrate)

The repository follows a Linux-kernel–style modular layout. Each directory is a self-contained unit with clear boundaries.

| Path | Name | Role |
|------|------|------|
| **/core** | Kiini | Platform-agnostic lexer, parser, AST, evaluator, diagnostics (Mwalimu error reporting). |
| **/driver** | Mfumo / Dereva | Hardware/OS abstraction. **Dereva** is the internal HAL/FFI layer; **mfumo** is the high-level System API (StdLib) built on it. |
| **/pata** | Tooling | CLI, package manager, formatter (Nadhifu), linter, LSP (Mwalimu), Debug Adapter Protocol server. |
| **/extensions/vscode** | Kiendelezi | VS Code extension: syntax highlighting, Mwalimu LSP client. Separate build/package tooling (`npm`/`esbuild`), not part of the Cargo workspace. |
| **/lib** | Maktaba | Standard library (Msingi) and tests, docs (`.asdoc`). Stdlib has two layers: surface in `lib/std/` (`.as`/`.asi`); implementation as built-in modules (evaluator/CLI) that can resolve without disk. Third-party modules come from `[tegemezi]` in `pata.toml`. |
| **/target** | Pato | Generated binaries and intermediate `.asb` bytecode. |

**Subsystem maintainers (Msimamizi):** Kiini (Core), Dereva (Drivers/FFI), Pata (Tooling), Msingi (StdLib). Patches that cross subsystems require acks from the relevant maintainers.

---

## File and artifact conventions

| Extension | Role | Owner |
|-----------|------|--------|
| **.as** | Asili source (e.g. `main.as`, `hesabu.as`) | Author |
| **.asi** | Asili interface (headers / Sifa declarations) | Author |
| **pata.toml** | Project manifest (metadata, dependencies) | Author |
| **pata.lock** | Lockfile for deterministic builds | Tool |
| **.asb** | Asili bytecode (VM input) | Tool |
| **.asm** | Asili assembly (debug / `pata jenga --nguvu`) | Tool |
| **.asdoc** | Documentation templates for `pata maelezo` | Author / Tool |

---

## Scalable architecture (directory breakdown)

### /core (Kiini)

- **lexer/** — Tokenizes Swahili input from `.as` and `.asi` files.
- **parser/** — Generates the AST; split into cursor (token stream), parse (statements/expressions), and semantic (types + analyzer). Integration tests live in `tests/`.
- **evaluator/** — Tree-walking engine that processes `.asb` (Asili Bytecode). Integration tests live in `tests/`.
- **diagnostics/** — The "Mwalimu" error reporting system (Context Map).

### /driver (Mfumo)

- **posix/** — Linux/macOS system calls.
- **win32/** — Windows system calls.
- **wasm/** — Browser-based system stubs.
- **embedded/** — Bare-metal HAL. Handles `.asm` (Asili Assembly) generation for low-level debugging.

### /pata (Tooling)

- **cli/** — Command-line interface and REPL.
- **package/** — Manages `pata.toml` and generates `pata.lock`.
- **fmt/** — The "Nadhifu" formatter for `.as` and `.asi` files.
- **lsp/** — The "Mwalimu" Language Server.

### /lib (Maktaba)

- **std/** — Standard library surface (written in `.as` and `.asi`). These files are the in-language interface for the same modules the builtins implement (e.g. hisabati, mfumo, moduli).
- **tests/** — Integration tests for the entire ecosystem.
- **docs/** — Uses `.asdoc` templates to generate documentation via `pata maelezo`.

### /target (Pato)

Generated binaries and intermediate `.asb` bytecode.

---

Previous: [Philosophy and EDP](01-philosophy-and-edp.md) | [Overview](../SPECIFICATION.md) | Next: [Syntax](03-syntax.md)
