# Spec changelog

All notable changes to the Asili specification are recorded here.

## Unreleased

- Architecture ([02-architecture-and-files.md](02-architecture-and-files.md)): added a
  `/extensions/vscode` row to the project-structure table (previously undocumented despite
  existing); noted the linter and DAP server in the `/pata` row's tooling list.
- Tooling ([06-tooling-and-ecosystem.md](06-tooling-and-ecosystem.md)): added `#[kabla]`/`#[baada]`
  to the system-attributes table (setup/teardown fixtures run by `pata jaribu` around every
  `#[jaribio]` test in the same module, including when the test itself failed); updated the
  `pata jaribu` row for the new `--muda <sekunde>` per-test timeout flag and `--chanjo` real
  line-level coverage tracking, and the `pata thibitisha` row for its type-stability check
  against the most recent `v<semver>` git tag.
- Resolved decisions ([08-resolved-decisions.md](08-resolved-decisions.md)): added 9.12 HTTP-server-shaped stack — the bounded-thread-pool-over-async decision for `mkondo_tumikia`/`mkondo_tumikia_http` (extends the 1:1-OS-thread model from 9.11 rather than introducing `tokio` as a second concurrency substrate), `mkondo_tumikia`'s blocking/per-connection model, `EvalError::Coded` as an additive (not restructuring) error-model change, `rustls`-over-`native-tls` for TLS, and `httparse`-based HTTP/1.1 framing as a separate entry point (`mkondo_tumikia_http`, not a mode flag) with its explicit chunked/pipelining/100-continue scope cuts.
- Standard library ([05-standard-library.md](05-standard-library.md)): added `kwa_json`/`kutoka_json` (`Value`↔JSON codec, under the `mfumo` module) with per-variant encoding rules; `MkondoSikilizaji`/`mkondo_tumikia`/`mkondo_tumikia_http` (listening socket, bounded worker pool, HTTP/1.1-framed variant) with `OmbiHttp`/`JibuHttp` struct shapes; `TlsUsanidi`/`tls_sanidi`; `Mkondo.soma_bailisi(kikomo)` (bounded, non-EOF-seeking read); `njia_na_kikomo` (bounded channel constructor alongside `njia()`); `Kasha_GC<T>`'s `kasha_gc_dhaifu`/`.imarisha()` weak-reference pair.
- Tooling ([06-tooling-and-ecosystem.md](06-tooling-and-ecosystem.md)): `pata nadhifu` section rewritten from a one-line description to the actual formatting rules (token-stream pretty-printer, call/index-hugging, generic-bracket vs. comparison-operator disambiguation, comment preservation, blank-line handling, string/char re-escaping, lex-error fallback) now that the formatter is a real implementation, not a line-based text transform.
- Resolved decisions ([08-resolved-decisions.md](08-resolved-decisions.md)): added 9.11 Concurrency — 1:1 OS-thread scheduling model for `tenda` (not M:N green threads), cross-thread value rules (`Kasha_GC<T>`/`Faili`/`Mkondo` rejected from crossing threads), `tenda`'s return-value-via-`njia`-not-`subiri_tenda` contract, and `fungo`'s explicit `.funga()`/`.fungua()` vs. atomic `.pata()`/`.weka()` split.
- Standard library ([05-standard-library.md](05-standard-library.md)): filled in real signatures for `tenda`/`subiri_tenda`/`njia`/`fungo` (previously "exact API defined at implementation") and for `Faili`/`Mkondo`/`Kumbukumbu<T>` (previously prose-only); noted `Sifa` now has method-signature completeness checking, not nominal-only; added `Seti<T>`'s real method table and `Namba_Kuu`/`Namba_Sahihi`'s construction/casting rules.
- Type system ([04-type-system.md](04-type-system.md)): Sifa entry updated from "types implement Sifa via shughuli ya X kwa Trait" to describe the real completeness check (`SEM105`) and the explicit no-trait-object/dyn-dispatch scope boundary; Data Shapes table updated for `Seti<T>` (implemented) and `Mfululizo` (still blocked, now with a cited reason — the borrow-checker lifetime decision); arbitrary-precision section expanded with construction/casting notes.
- Implementation (Nuru comparison plan): Phase A (docs/howto, install UX, examples), B (LSP split, builtins split), D (REPL, docs/repl + ?topic), E (VSCode extension), F2 (DAP doc note). Phase C (eval/value split) and E2/F1 deferred. See [docs/design/dap-later.md](../design/dap-later.md) for DAP.
- Architecture, standard library, tooling, execution ([02-architecture-and-files.md](02-architecture-and-files.md), [05-standard-library.md](05-standard-library.md), [06-tooling-and-ecosystem.md](06-tooling-and-ecosystem.md), [07-execution-and-roadmap.md](07-execution-and-roadmap.md)): clarified stdlib (built-in + lib/std surface) and third-party modules (tegemezi). See [docs/design/implementation-status.md](../design/implementation-status.md) for current tooling/structure status.
- Tooling ([06-tooling-and-ecosystem.md](06-tooling-and-ecosystem.md)): Phase II Mwalimu (LSP) and Wasm: `pata mwalimu` and `pata-lsp` binary; LSP provides diagnostics and hover for `.as` files. Wasm: evaluator and `asili-wasm` driver build for `wasm32-unknown-unknown` with spec-compliant I/O stubs; `run_source` entry point; CI workflow for wasm32 build.
- Tooling ([06-tooling-and-ecosystem.md](06-tooling-and-ecosystem.md)): `pata nadhifu` documented as line-based, best-effort; AST-based formatter may follow. Phase II: mfumo builtin stubs, test runner (exit codes, summary, `--list`), docs/examples, evaluator undefined-variable message improved to quote name.
- Execution and roadmap ([07-execution-and-roadmap.md](07-execution-and-roadmap.md)): Phase I feature–phase map updated to reflect core complete (umbo, shughuli ya, Orodha index, Kamusi, Herufi, Jozi, linganisha struct/Jozi).
- Architecture and files ([02-architecture-and-files.md](02-architecture-and-files.md)): parser and evaluator layout described (cursor/parse/semantic split; integration tests in `tests/`).
- Standard library ([05-standard-library.md](05-standard-library.md)): Phase I builtin modules (hisabati, mfumo) noted as resolved without disk I/O.
- Philosophy and EDP ([01-philosophy-and-edp.md](01-philosophy-and-edp.md)): clarified ownership-default memory model (ownership/borrowing by default; managed/GC only via opt-in modules; `wazi` as unsafe escape hatch) and added Mwalimu Context Maps requirement for ownership/borrowing diagnostics in Foundation/Validation.
- Execution and roadmap ([07-execution-and-roadmap.md](07-execution-and-roadmap.md)): memory strategy changed from GC-default to ownership + borrowing with deterministic drop; allocation failure defaults to Abort unless stricter allocator is configured; GC-style behavior moved to optional managed modules layered on ownership.
- Type system ([04-type-system.md](04-type-system.md)): added normative ownership/borrowing rules (single owner, moves by default, post-move invalidation, `azima`/`azima_tenda`, aliasing rule, deterministic drop and `tupa`, `Nakala`/`.nakala()`, lifetime inference defaults) and tied `Rejeo`/`Rejeo_Tenda` directly to borrow syntax.
- Resolved decisions ([08-resolved-decisions.md](08-resolved-decisions.md)): added 9.10 Ownership and borrowing; updated 9.1 to require recoverable errors as data (`Tokeo<T,E>` / `Chaguo<T>` / `T?`) and define `paparika` as unrecoverable, non-catchable panic.
- Syntax and stdlib error model ([03-syntax.md](03-syntax.md), [05-standard-library.md](05-standard-library.md)): introduced `paparika` keyword entry and panic semantics; reinforced `Tokeo<T,E>` for recoverable errors.
- Standard library resources ([05-standard-library.md](05-standard-library.md)): documented resource-handle types (`Faili`, `Mkondo`, `Kumbukumbu<T>`), deterministic cleanup on drop with `tupa` for early release, I/O traits (`Inasomeka`, `Inandikika`), and ownership-oriented hardware handles (`Pini`, `Bafa<T>`).
- Overview and maintenance ([../SPECIFICATION.md](../SPECIFICATION.md), [00-maintenance.md](00-maintenance.md)): quick reference now includes `Panic` and `Ownership` rows; maintenance now explicitly classifies GC-default to ownership-default as a major substrate shift.
- Operator and numeric-edge model ([03-syntax.md](03-syntax.md), [04-type-system.md](04-type-system.md), [05-standard-library.md](05-standard-library.md), [08-resolved-decisions.md](08-resolved-decisions.md)): added complete operator families (including `siyo_biti`, compound assignment, and `wakati milele`), IEEE-754 edge-state vocabulary (`Ukomo`, `Siyo_Namba`), friction terms (`Mfuriko`, `Ufinyu`), and `hisabati` helpers (`duara`, `absolute`) with fallible math signatures normalized to `Tokeo<Namba, Kosa>`.
- Consistency fixes ([SPECIFICATION.md](../SPECIFICATION.md), [07-execution-and-roadmap.md](07-execution-and-roadmap.md), [08-resolved-decisions.md](08-resolved-decisions.md), [04-type-system.md](04-type-system.md), [05-standard-library.md](05-standard-library.md)): aligned master title to v1.1, clarified Phase II as opt-in managed/GC module work, removed ownership/lifetime deferral contradiction, and made fixed-width arithmetic semantics normative (wrapping by default; checked flows via `Tokeo`).
- Type system ([04-type-system.md](04-type-system.md)): corrected trait-implementation syntax
  from bare `shughuli ya` to `shughuli ya X kwa Trait`.
- Tooling ([06-tooling-and-ecosystem.md](06-tooling-and-ecosystem.md)): added `pata jenga
  --tenda`, `pata tenda`, `pata thibitisha`, and `pata repl` to the command table (previously
  undocumented despite existing); expanded the Mwalimu (LSP) feature list to include
  completion, goto-definition, find-references, rename, workspace symbols, and signature help
  (previously described as diagnostics/hover only).
- Standard library ([05-standard-library.md](05-standard-library.md)): added a dedicated
  `Kasha_GC<T>` subsection under Moduli ya Msingi (previously only referenced in passing from
  the roadmap doc).
- Spec directory moved from top-level `spec/` to `docs/spec/`, and `SPECIFICATION.md` to
  `docs/SPECIFICATION.md`, consolidating all documentation (spec, design, language, REPL,
  howto) under one `docs/` tree; all internal and external cross-references updated. Root-level
  `PHASE_I.md`/`PHASE_II.md` (superseded, redundant with
  [docs/design/implementation-status.md](../design/implementation-status.md)) removed.

## 1.1

- Spec maintenance document ([00-maintenance.md](00-maintenance.md)): versioning, where to edit, process, changelog, checklist.
- Logic expansion: advanced control flow (`linganisha`, `vunja`, `endelea`, `lebo`), error propagation (`jaribu`, `?`).
- Type system: abstraction (Sifa, Jumla\<T\>), memory and references (Rejeo, Muda_wa_Kuishi, Kiashiria, Gundi), data shapes (Chaguo\<T\>, Mfululizo, Jozi, Seti), modules (Pakiti/Moduli).
- Standard library: concurrency primitives (njia, fungo), async (sawia, subiri) deferred.
- Tooling: Kielelezo! (macros), system attributes (#[ndani], #[jaribio], #[sharti], #[kiunganishi]).
- Resolved decisions: 9.7 Chaguo vs T?, 9.8 Error propagation, 9.9 Attributes; Deferred note for references and async.
- Feature–phase map and dependency notes in Execution and Roadmap.

## 1.0

- Initial specification: philosophy, EDP, architecture, syntax, type system, standard library, tooling, execution pipeline, memory strategy, contribution workflow, resolved decisions (9.1–9.6).
