# Asili

**Asili** (Origin / Nature) is a programming language that uses Swahili as the primary vocabulary for logic and structure. The toolchain (**Pata**) provides a single pipeline for learning, scripting, and embedded-style targets.

- **Syntax:** Swahili keywords (`kazi`, `weka`, `ikiwa`, `linganisha`, `rejesha`, …).
- **Types:** Primitives (`Namba`, `Neno`, `Ukweli`), collections (`Orodha<T>`, `Kamusi<K,V>`), `Chaguo<T>`, `Tokeo<T,E>`, structs and impls.
- **Stdlib:** Modular built-ins (msingi, mfumo, majira, matumizi, faili, hisabati, runtime, syscall, kiungo, sambamba) plus `lib/std` interface stubs.
- **Spec:** [SPECIFICATION.md](SPECIFICATION.md) and [spec/](spec/) define the language and execution model.

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
| `pata jaribu` | Discover and run `#[jaribio]` tests. |
| `pata jaribu --orodha` | List test names only. |
| `pata njozi <dir>` | Create a new project (e.g. `pata.toml`, `src/kuu.as`). |
| `pata nadhifu` | Format Asili source (line-based, best-effort). |
| `pata thibitisha` | Check public API documentation. |
| `pata mwalimu` | Start LSP server (Mwalimu) for editors. |

## Layout

| Path | Contents |
|------|----------|
| `core/` | Lexer, parser, semantic analysis, evaluator, diagnostics. |
| `driver/` | Target adapters (e.g. Wasm). |
| `pata/` | CLI (`pata-cli`), runner, LSP. |
| `lib/` | Standard library surface (`lib/std/*.asi` stubs). |
| `spec/` | Language specification. |
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

- **How-to:** [docs/howto/](docs/howto/) — getting started, tests, LSP.
- **Design:** [docs/design/](docs/design/) — Mwalimu (LSP), phase decisions.
- **Spec:** [spec/](spec/) and [SPECIFICATION.md](SPECIFICATION.md).
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md).
- **Changelog:** [CHANGELOG.md](CHANGELOG.md).
- **Security:** [SECURITY.md](SECURITY.md).

## License

This project is licensed under the **GNU General Public License v2.0** — see [LICENSE](LICENSE).
