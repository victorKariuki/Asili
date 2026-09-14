# `pata jaribu`

Purpose: discover and execute test suites.

Flags:
- `--chuja <pattern>` — run or list only tests whose name contains pattern
- `--simama-haraka` — stop on first failure
- `--orodha` — list test names (one per line), do not run; exit 0 if discovery succeeds
- `--nyuzi-za-jaribio <n>` — execute tests in parallel using n worker threads; default (sequential)
- `--json` — output structured JSON instead of text; includes test count, pass/fail breakdown, per-test details

Exit codes:
- 0 — all tests passed (or listing/discovery succeeded)
- 1 — one or more tests failed
- 2 — usage or config error

Success:
- discovers `#[jaribio]`-tagged functions
- compiles and executes tests
- prints summary; final line varies by output format (text: "majaribio yote yamefaulu" or "majaribio N yameshindwa"; JSON: structured result object)

Failures:
- compile/type-check failure
- runtime panic (`paparika`) in tests
