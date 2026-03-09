# Changelog

All notable changes to the Asili implementation (repository and tooling) are recorded here. For specification changes, see [spec/CHANGELOG.md](spec/CHANGELOG.md).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **Msingi (prelude):** Constructors `tokeo(val)`, `kosa(ujumbe)`, `chaguo(val)`, `kamusi()`; constant `TUPU`. Type methods: `Chaguo` — `angu`, `ni_tupu`, `ni_po`, `hakikisha`; `Tokeo` — `ni_kosa`, `kosa`, `angu`; `Kamusi` — `funguo`, `vipo`.
- **Stdlib modules:** `runtime`, `syscall`, `kiungo`, `sambamba` (builtins and export tables); mfumo signals `sikiliza_ishara`, `rejesha_ishara` with eval-loop dispatch; `Anuani` type and signal state for Unix.
- **Prelude constant:** `TUPU` seeded in runtime and declared in msingi.

### Changed

- **Msingi:** Removed procedural helpers `chaguo_au_namba` and `ni_hamna` in favor of Chaguo methods (`.angu()`, `.ni_tupu()`, etc.). Examples (astar, data_structures) updated to use method calls.
- **Prelude:** Semantic and pipeline use prelude-only (msingi) for initial scope; duplicate-import checks attribute prelude names to msingi.
- **I/O and time:** `chapisha`, `paparika`, `omba` moved to `matumizi`; time helpers and `sasa` in `majira`; file ops in `faili`. Explicit `leta matumizi` (or relevant module) required for I/O.

### Fixed

- Boolean output for `Ukweli` cast to `Neno`: `false` now prints as `"si_kweli"` (was `"sikweli"`).
- Unreachable code in `mfumo::toka()` on non-WASM targets.
- Test fixtures and examples updated for modular stdlib (e.g. `leta matumizi` where `chapisha` is used).

## [0.1.0] (initial implementation)

- Core: lexer, parser, semantic analysis, evaluator, diagnostics.
- CLI (Pata): `jenga`, `jaribu`, `njozi`, `nadhifu`, `thibitisha`, `mwalimu` (LSP).
- Stdlib (built-in): msingi, mfumo, majira, matumizi, faili, hisabati.
- Types: Namba, Neno, Ukweli, Tupu, Hamna, Chaguo, Tokeo, Orodha, Kamusi, Jozi, Struct, Wakati, Herufi, bit-width casts.
- Control flow: ikiwa/au_ikiwa/vinginevyo, wakati, kwa, linganisha, vunja/endelea, jaribu/?.
- Examples and spec-aligned layout.

[Unreleased]: https://github.com/.../compare/v0.1.0...HEAD
[0.1.0]: https://github.com/.../releases/tag/v0.1.0
