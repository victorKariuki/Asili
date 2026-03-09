# Asili Workspace Scaffold

This repository is scaffolded from the Asili specification.

## Getting started

**Prerequisites:** Rust toolchain (e.g. `rustup`).

**Build:** From the workspace root, run `cargo build`. To build an Asili project and produce bytecode, use `pata jenga` from that project’s directory (requires `pata.toml` and entrypoint, e.g. `src/kuu.as`).

**Run:** From an Asili project directory, run `pata jenga --run` to compile and execute the entrypoint.

**Tests:** Run `pata jaribu` to discover and run `#[jaribio]` tests. Use `pata jaribu --list` to list test names without running them.

**New project:** Run `pata njozi` in an empty directory to create a new project with `pata.toml` and `src/kuu.as`.

See `examples/` for small runnable samples; copy an example into your project’s `src/kuu.as` (or set it as entrypoint) and run `pata jenga --run`.

## Docs

- **How-to:** [docs/howto/](docs/howto/) — getting started, running tests, using the LSP.
- **Design:** [docs/design/](docs/design/) — Mwalimu (LSP) design, Phase II decisions.
- **Spec:** [spec/](spec/) and [SPECIFICATION.md](SPECIFICATION.md) — language and execution reference.
- **Stdlib API templates:** [lib/docs/](lib/docs/) — `.asdoc` templates for `pata maelezo`.

## Layout

- `core/` — Kiini (lexer, parser, evaluator, diagnostics)
- `driver/` — Dereva/Mfumo target adapters
- `pata/` — CLI, package, formatter, LSP tooling
- `lib/` — Standard library, tests, docs templates
- `src/` — Application Asili source
- `target/` — Build artifacts (`.asb`, binaries, debug output)
- `spec/` — Language specification
- `examples/` — Sample Asili programs (copy into a project to run)
