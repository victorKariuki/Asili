![Asili Banner](assets/Asili_Banner.png)

# <img src="assets/Asili_Logo.png" alt="" width="40" valign="middle" /> Asili

**Asili** (Origin / Nature) is a programming language that uses Swahili as the primary vocabulary for logic and structure. The toolchain (**Pata**) provides a single pipeline for learning, scripting, and embedded-style targets.

- **Syntax:** Swahili keywords (`kazi`, `weka`, `ikiwa`, `linganisha`, `rejesha`, …).
- **Types:** Primitives (`Namba`, `Neno`, `Ukweli`), collections (`Orodha<T>`, `Kamusi<K,V>`), `Chaguo<T>`, `Tokeo<T,E>`, structs and impls.
- **Stdlib:** Modular built-ins (msingi, mfumo, majira, matumizi, faili, hisabati, runtime, syscall, kiungo, sambamba) plus `lib/std` interface stubs.
- **Spec:** [docs/SPECIFICATION.md](docs/SPECIFICATION.md) and [docs/spec/](docs/spec/) define the language and execution model.

## Prerequisites

- **Rust** toolchain (e.g. [rustup](https://rustup.rs/)).

## Build and run

From the workspace root:

```bash
cargo build
```

To work with an Asili project (e.g. under `examples/asi_sample` or one created with `pata njozi`):

```bash
cd path/to/project
cargo run -p pata-cli -- jenga          # compile to bytecode
cargo run -p pata-cli -- jenga --tenda    # compile and execute entrypoint
```

If the CLI is installed as `pata`:

```bash
pata jenga
pata jenga --tenda
```

## CLI commands (Pata)

| Command | Description |
|--------|-------------|
| `pata jenga` | Build project; output `.asb` (and optional manifest). |
| `pata jenga --tenda` | Build and run the entrypoint (`kuu`). |
| `pata tenda <path>` | Run an already-built `.asb`/`.build.manifest` without rebuilding. |
| `pata jaribu` | Discover and run `#[jaribio]` tests. |
| `pata jaribu --orodha` | List test names only. |
| `pata repl` | Start interactive REPL. Use `?mada` for help topics, `?lugha en`/`?lugha sw` to switch languages. |
| `pata njozi <dir>` | Create a new project (e.g. `pata.toml`, `src/kuu.as`). |
| `pata ongeza <lib>` | Add a dependency to `pata.toml`. |
| `pata nadhifu` | Format Asili source (line-based, best-effort). |
| `pata thibitisha` | Check public API documentation. |
| `pata mwalimu` | Start LSP server (Mwalimu) for editors. |

## Layout

| Path | Contents |
|------|----------|
| `core/` | Lexer, parser, semantic analysis, evaluator, diagnostics. |
| `driver/` | Target adapters: `wasm` (implemented, browser + WASI); `embedded`, `posix`, `win32` (stub placeholders, not yet in the Cargo workspace). |
| `pata/` | CLI (`pata-cli`), runner, LSP (`pata-lsp`), formatter (`pata-fmt`), linter (`pata-lint`), package resolver (`pata-package`). |
| `lib/` | Standard library surface (`lib/std/*.asi` stubs). |
| `docs/spec/` | Language specification. |
| `examples/` | Sample Asili programs. |
| `docs/` | How-to and design notes. |

## Standard library modules

Available via `leta <moduli>` (e.g. `leta matumizi`):

- **msingi** — Prelude: constructors (`orodha`, `kamusi`, `jozi`, `tokeo`, `kosa`, `chaguo`), constants (`KWELI`, `SIYO_KWELI`, `TUPU`).
- **mfumo** — System: `vigezo`, `pata_env`, `toka`, `sikiliza_ishara`, `rejesha_ishara`.
- **majira** — Time: `sasa`, `majira`, `sekunde`, `kutoka_sekunde`, `umbiza`, `lala`.
- **matumizi** — I/O: `chapisha`, `onyo`, `makosa`, `paparika`, `omba`.
- **faili** — File system: `soma_faili`, `andika_faili`, `ongeza`, `vipo`, `futa`, `ukubwa`.
- **hisabati** — Math: `jumla`, `tofauti`, `zao`, `gawio`, `duara`, `kipeo`, `mizizi`, etc.
- **runtime** — `toleo`, `jina_os`.
- **syscall** — Raw syscall stub.
- **kiungo** — FFI stubs (`saza_kiungo`, `wito_kiungo`).
- **sambamba** — Concurrency stubs (`anza_mwendo`, `subiri_mwendo`).

## Documentation

Two top-level locations, split by what kind of question you're asking:

- **[docs/spec/](docs/spec/)** (entry point: [docs/SPECIFICATION.md](docs/SPECIFICATION.md)) — the formal, normative
  language and ecosystem specification: syntax, type system, stdlib contract, tooling contract,
  execution model, resolved design decisions. Changes here define what Asili *is*.
- **[docs/](docs/)** — implementation status, tutorials, and reference material for using what
  exists today:
  - [docs/design/implementation-status.md](docs/design/implementation-status.md) — **start
    here** for "is X actually done" — phase-by-phase checklist, known gaps, recommended order
    of work. The single progress/status doc; supersedes any older phase-status file.
  - [docs/language/](docs/language/) — language tutorial (syntax, functions, control flow,
    data structures, error handling, modules).
  - [docs/repl/](docs/repl/) — REPL topic docs (Swahili in `sw/`, English in `en/`), shown via
    `?mada` in `pata repl`; switch languages with `?lugha en`/`?lugha sw`.
  - [docs/howto/](docs/howto/) — getting started, running tests, using the LSP.
  - [docs/design/](docs/design/) — implementation-level design docs (e.g. Mwalimu/LSP
    architecture) that don't belong in the normative spec.

Root-level, project meta rather than language docs: [CONTRIBUTING.md](CONTRIBUTING.md),
[CHANGELOG.md](CHANGELOG.md), [SECURITY.md](SECURITY.md).

## License

This project is licensed under the **GNU General Public License v2.0** — see [LICENSE](LICENSE).
