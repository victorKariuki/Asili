# Spec changelog

All notable changes to the Asili specification are recorded here.

## Unreleased

- Implementation (Nuru comparison plan): Phase A (docs/howto, install UX, examples), B (LSP split, builtins split), D (REPL, docs/repl + ?topic), E (VSCode extension), F2 (DAP doc note). Phase C (eval/value split) and E2/F1 deferred. See [docs/design/dap-later.md](../docs/design/dap-later.md) for DAP.
- Architecture, standard library, tooling, execution ([02-architecture-and-files.md](02-architecture-and-files.md), [05-standard-library.md](05-standard-library.md), [06-tooling-and-ecosystem.md](06-tooling-and-ecosystem.md), [07-execution-and-roadmap.md](07-execution-and-roadmap.md)): clarified stdlib (built-in + lib/std surface) and third-party modules (tegemezi). See [docs/nuru-comparison.md](../docs/nuru-comparison.md) for tooling/structure comparison.
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
