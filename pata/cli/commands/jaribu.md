# `pata jaribu`

Purpose: execute test suites.

Inputs:
- `--filter <pattern>` — run or list only tests whose name contains pattern
- `--fail-fast` — stop on first failure
- `--list` — list test names (one per line), do not run; exit 0 if discovery succeeds

Exit codes:
- 0 — all tests passed (or `--list` succeeded)
- 1 — one or more tests failed
- 2 — usage or config error (e.g. invalid `--filter`)

Success:
- discovers `#[jaribio]`
- executes tests and prints summary; final line "majaribio yote yamefaulu" or "majaribio N yameshindwa"

Failures:
- compile/type-check failure
- runtime panic (`paparika`) in tests
