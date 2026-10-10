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

## API documentation

Generated API reference (for reading the code, not for using the `pata` CLI or the Asili
language — see the [wiki](https://github.com/victorKariuki/Asili/wiki) for that) is published at
<https://victorkariuki.github.io/Asili/>, rebuilt automatically on every push to `main`
(`.github/workflows/docs.yml`).

To build it locally:

- **Rust (rustdoc):** `cargo doc --workspace --no-deps --open` — covers every crate under
  `core/` and `pata/`. CI runs this with `RUSTDOCFLAGS=-D warnings`, so a broken intra-doc link or
  unescaped `<T>`/`[...]` in a doc comment fails the build, not just warns — fix it rather than
  working around it (wrap generic syntax like `` `Kasha_GC<T>` `` in backticks, escape `#[attr]`
  and `<placeholder>` text the same way).
- **VS Code extension (TypeDoc):** `cd extensions/vscode/docs-tooling && npm install && npm run
  typedoc` — outputs to `docs-site/typedoc/` at the repo root. TypeDoc is isolated in its own
  `docs-tooling/` package with its own pinned TypeScript (`^5.9`), separate from the extension's
  own `devDependencies` (`^7.0`) — TypeDoc's peer-dependency range doesn't support the newer
  compiler yet. Don't move TypeDoc into `extensions/vscode/package.json` directly without checking
  that range first.

## Project structure

- **core/** — Language core: `diagnostics` (shared error/diagnostic types), `lexer`, `parser`
  (includes the semantic analyzer), `evaluator` (the bytecode compiler, the in-house `nguvu` native backend and its host, one
  implementation of the semantics, plus built-ins). Add or
  extend built-ins under `core/evaluator/src/builtins/`. `core/` never depends on `pata/` — a
  type `core/` needs to expose to `pata/` (e.g. the `DebugHook` trait `pata-dap` drives) is
  defined on the `core/` side and re-exported, not the other way around.
- **pata/** — the toolchain, one crate per concern: `cli` (the `pata` binary — entrypoint
  `pata/cli/src/main.rs`, commands in `commands/`, pipeline in `pipeline/`), `core` (shared
  module resolver, used by both `cli` and `lsp` so they don't reimplement it separately), `fmt`
  (`pata nadhifu`), `lint` (`pata-lint`), `package` (dependency resolution/lockfile/registry),
  `runner` (`.asb` bytecode execution), `lsp` (Mwalimu language server), `dap` (Debug Adapter
  Protocol server), `config` (finding `pata.toml` and reading a tool's section of it, shared by
  `fmt`, `lint` and `lsp`).
- **extensions/vscode/** — the VS Code extension (`asili` on the Marketplace once published).
  `src/extension.ts` is the entry point; bundled with `esbuild` (see `esbuild.js`) rather than
  shipping `node_modules` in the packaged `.vsix`. `make install-ext` from the repo root builds,
  packages, and installs it in one step.
- **lib/std/** — `.asi` interface stubs for the standard library, generated from `core/parser/src/builtins.rs` (`BUILTIN_MODULES`). Never edit them by hand: change the table, then run `ASILI_GOLDEN=write cargo test -p pata-core stdlib_stubs` (`cargo test -p pata-core` fails while they differ).
- **driver/** — targets beside the native runner: `wasm` (the browser playground build) and
  `kifaa` (`asili-kifaa`, the `no_std` runtime Cortex-M firmware links with strict code built by
  `pata jenga --lengo cortex-m`).
- **docs/assurance/** — safety requirements, traceability, risk register and tool-qualification
  evidence.
- **docs/spec/** — Formal language specification. Spec changes should be reflected in [docs/spec/CHANGELOG.md](docs/spec/CHANGELOG.md).

## Code and style

- **Rust:** Format with `cargo fmt`. Follow existing patterns in each crate (e.g. error handling, naming).
- **Asili source:** Use Swahili keywords and the style shown in `examples/` and `docs/spec/`.
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
- **Engines agree** (any change to the evaluator, native code or operator/method semantics):  
  `cargo test -p asili-evaluator --test engines_agree --test native_tiers`  
  (on x86-64 or AArch64; changes to `nguvu` code generation should also be run for the other
  architecture — CI runs arm64 natively, or locally under qemu as the `performance-guardrails`
  skill describes)
- **Performance** (same scope): `examples/sudoku/bench/run.sh` — Asili's native solve should stay
  at or below clang `-O2` C and report 90,665 attempts. See
  [docs/design/performance.md](docs/design/performance.md).

- **Optimizer fuzzing** (any change to `nguvu`'s lowering or optimizer):  
  `ASILI_FUZZ_PROGRAMS=2000 cargo test --release -p asili-evaluator --test fuzz`
- **Cortex-M** (any change to `nguvu/t32.rs`, `codegen_t32.rs`, `device.rs` or `driver/kifaa`):  
  `cargo test -p asili-evaluator --test kifaa` — needs `clang`, `ld.lld`, `qemu-arm-static`
  and `rustup target add thumbv7em-none-eabihf`; skipped with a note where they are missing.

New behavior should be covered by tests where practical (parser, semantic, evaluator, or CLI tests as appropriate).

**Safety requirements.** Behaviour listed in
[docs/assurance/requirements.md](docs/assurance/requirements.md) is traced to the tests that
verify it: a test names the requirements it verifies in a `// Verifies: REQ-…` comment above
it, and `cargo test -p asili-evaluator --test traceability` fails when a requirement has no
test or the generated matrix (`docs/assurance/traceability.md`) is stale — regenerate it with
`ASILI_TRACE=write cargo test -p asili-evaluator --test traceability`. A change to such
behaviour updates the requirement, its tests and, where the risks change,
[docs/assurance/risk-register.md](docs/assurance/risk-register.md).

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

- Language and execution semantics are defined in [docs/SPECIFICATION.md](docs/SPECIFICATION.md) and the [docs/spec/](docs/spec/) directory. Proposed language or spec changes are best discussed (e.g. in an issue or PR) before large edits.
- Design notes and decisions live under [docs/design/](docs/design/). Significant tooling or architecture changes may warrant an update there or in the spec changelog.

## Questions

Open an issue for questions about contribution workflow, architecture, or the specification.
