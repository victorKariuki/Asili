# Phase II decisions

Short record of key decisions for Phase II (Synthesis).

## LSP and Wasm

- **LSP:** Implemented as a separate crate `pata/lsp` (tower-lsp, stdio). Delivered: diagnostics, hover. Goto-definition and completion deferred.
- **Wasm:** Evaluator and `driver/wasm` build for `wasm32-unknown-unknown`. Builtins use cfg gates for std::env, SystemTime, println so wasm gets stubs. Single entry: `asili_wasm::run_source(source)`.

## Stdlib and modules

- **Built-in stdlib:** hisabati and mfumo are resolved without disk (builtin_module_exports + evaluator builtins). Same modules have an in-language surface in `lib/std/` (`.asi`/`.as`).
- **Third-party:** `[tegemezi]` in pata.toml; dependencies added with `pata ongeza <lib>`.

## Tooling

- **Test runner:** Exit codes 0/1/2, summary line, `--list` mode. Documented in spec and jaribu command doc.
- **Docs/examples:** Getting started in README; howto in `docs/howto/`; examples in `examples/` with README.

See [spec/07-execution-and-roadmap.md](../../spec/07-execution-and-roadmap.md) for the phase map and [spec/CHANGELOG.md](../../spec/CHANGELOG.md) for changelog entries.
