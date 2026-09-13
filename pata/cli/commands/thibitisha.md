# `pata thibitisha`

Purpose: CI-style verification gate.

Inputs:
- `--kiwango-cha-jaribio <0-100>` — optional; minimum ratio (percent) of `#[jaribio]` test
  functions to public `kazi` in the project. Not enforced unless passed (no default threshold).

Checks:
- project compiles
- docs required for public `kazi`, `umbo`, and `sifa`
- formatting is canonical
- test coverage ratio, only when `--kiwango-cha-jaribio` is given

Success:
- zero exit status

Failures:
- missing public docs
- compile/type-check failures
- formatting violations
- test coverage below `--kiwango-cha-jaribio`'s threshold, when given

Not yet implemented (tracked in `docs/design/implementation-status.md`):
- type stability (breaking public-signature changes between versions)
- ABI compatibility (`kiunganishi`-exported functions vs. declared C signatures)
- trait completeness (every `sifa` in `[tegemezi]` fully implemented)
