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
- type stability: when the project is a git repository with at least one `v<semver>` tag, the
  current public function signatures are diffed against that tag's (the highest by real semver
  ordering, not lexical). Flags a removed public function, a changed parameter count, a changed
  parameter type at the same position, or a changed return type. No tag, or not a git repo at
  all, means nothing to check — not an error.
- formatting is canonical
- test coverage ratio, only when `--kiwango-cha-jaribio` is given

Success:
- zero exit status

Failures:
- missing public docs
- compile/type-check failures
- a `sifa` with zero impls anywhere in the project
- a `#[kiunganishi]` function with a non-FFI-safe parameter or return type
- a breaking public-API change since the most recent `v<semver>` git tag
- formatting violations
- test coverage below `--kiwango-cha-jaribio`'s threshold, when given

Not yet implemented (tracked in `docs/design/implementation-status.md`):
- type stability for `umbo`/`sifa` signature changes (struct fields, trait method signatures) —
  the function-signature check above extends to these mechanically, not a new design, once
  picked up
- full ABI compatibility (`kiunganishi`-exported functions vs. a declared C signature) — no
  C-signature declaration syntax exists yet (`kiungo`/FFI is a Phase IV stub); the FFI-safety
  check above is the real, checkable prerequisite, not a placeholder for this
