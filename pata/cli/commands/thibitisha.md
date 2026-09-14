# `pata thibitisha`

Purpose: CI-style verification gate.

Inputs:
- `--kiwango-cha-jaribio <0-100>` — optional; minimum ratio (percent) of `#[jaribio]` test
  functions to public `kazi` in the project. Not enforced unless passed (no default threshold).

Checks:
- project compiles
- docs required for public `kazi`, `umbo`, and `sifa`
- trait completeness: every `sifa` reachable from the project (local or via `leta` from a
  dependency/`.asi` stub) has at least one impl somewhere in the project
- FFI-safety: every `#[kiunganishi]`-tagged function's parameters/return type are C-ABI-safe
  primitives (not `Orodha`/`Kamusi`/struct/other heap-owning types)
- formatting is canonical
- test coverage ratio, only when `--kiwango-cha-jaribio` is given

Success:
- zero exit status

Failures:
- missing public docs
- compile/type-check failures
- a `sifa` with zero impls anywhere in the project
- a `#[kiunganishi]` function with a non-FFI-safe parameter or return type
- formatting violations
- test coverage below `--kiwango-cha-jaribio`'s threshold, when given

Not yet implemented (tracked in `docs/design/implementation-status.md`):
- type stability (breaking public-signature changes between versions) — needs a baseline (prior
  published version) to diff against; planned via a git-tag convention once `pata ongeza`
  actually fetches dependencies (see `docs/design/pata-production-readiness.md`)
- full ABI compatibility (`kiunganishi`-exported functions vs. a declared C signature) — no
  C-signature declaration syntax exists yet (`kiungo`/FFI is a Phase IV stub); the FFI-safety
  check above is the real, checkable prerequisite, not a placeholder for this
