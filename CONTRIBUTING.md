# Contributing to Asili

Contributions are welcome. This document outlines how to build, test, and submit changes.

## Development setup

1. **Clone the repository** and open the workspace root.
2. **Install Rust** (e.g. [rustup](https://rustup.rs/)).
3. **Build:**  
   `cargo build`
4. **Run tests:**  
   `cargo test`  
   (Skip the recursion-depth stress test if needed:  
   `cargo test -p asili-evaluator --test integration -- --skip recursion_depth`)

## Project structure

- **core/** — Language core: lexer, parser, semantic analyzer, evaluator. Add or extend built-ins under `core/evaluator/src/builtins/`. Stdlib export contracts live in `pata/cli/src/pipeline/builtin_modules.rs`.
- **pata/** — CLI (`pata-cli`), runner, LSP. Entrypoint is `pata/cli/src/main.rs`; commands are in `commands/`, pipeline in `pipeline/`.
- **lib/std/** — `.asi` interface stubs for the standard library; keep these in sync with built-in modules and `builtin_modules.rs`.
- **spec/** — Formal language specification. Spec changes should be reflected in [spec/CHANGELOG.md](spec/CHANGELOG.md).

## Code and style

- **Rust:** Format with `cargo fmt`. Follow existing patterns in each crate (e.g. error handling, naming).
- **Asili source:** Use Swahili keywords and the style shown in `examples/` and `spec/`.
- **Imports:** Keep imports at the top of files; avoid inline imports (see workspace rules if configured).

## Testing

- **Unit and integration:**  
  `cargo test`
- **Evaluator integration:**  
  `cargo test -p asili-evaluator --test integration`
- **CLI:**  
  `cargo test -p pata-cli`
- **Examples:** From an example directory (e.g. `examples/asi_sample`):  
  `cargo run -p pata-cli -- jenga --tenda`
  
  to confirm it builds and runs.

New behavior should be covered by tests where practical (parser, semantic, evaluator, or CLI tests as appropriate).

## Submitting changes

This project uses [Git-Flow](https://github.com/nvie/gitflow).

1. **Branch** using Git-Flow commands:
   - Features: `git flow feature start <name>`
   - Bugfixes: `git flow bugfix start <name>`
2. **Develop** and **commit** with clear, concise messages.
3. **Finish** your work with Git-Flow:
   - Features: `git flow feature finish <name>`
   - Bugfixes: `git flow bugfix finish <name>`
4. **Push** your changes to the remote repository.
5. **Pull Request:** Open a pull request against `main` (usually for releases) or `develop` as per Git-Flow practices.
6. **Describe** what changed and why; reference any issues or spec sections if relevant.
7. Ensure **CI** (if present) and local `cargo test` pass.

## Specification and design

- Language and execution semantics are defined in [SPECIFICATION.md](SPECIFICATION.md) and the [spec/](spec/) directory. Proposed language or spec changes are best discussed (e.g. in an issue or PR) before large edits.
- Design notes and decisions live under [docs/design/](docs/design/). Significant tooling or architecture changes may warrant an update there or in the spec changelog.

## Questions

Open an issue for questions about contribution workflow, architecture, or the specification.
