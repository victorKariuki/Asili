# `pata jaribu`

Purpose: discover and execute test suites.

Flags:
- `--chuja <pattern>` — run or list only tests whose name contains pattern
- `--simama-haraka` — stop on first failure
- `--orodha` — list test names (one per line), do not run; exit 0 if discovery succeeds
- `--nyuzi-za-jaribio <n>` — execute tests in parallel using n worker threads; default (sequential)
- `--muda <sekunde>` — per-test wall-clock timeout in seconds (fractional allowed, e.g. `0.5`).
  A test that exceeds it is reported as a failed test (message: `muda umekwisha baada ya ...`),
  and the rest of the suite still runs. Default: no timeout. The evaluator has no cooperative
  cancellation hook, so a timed-out test's thread keeps running in the background rather than
  being forcibly killed — the timeout stops the suite from *waiting* on it, not the thread itself
  from existing.
- `--json` — output structured JSON instead of text; includes test count, pass/fail breakdown, per-test details

Exit codes:
- 0 — all tests passed (or listing/discovery succeeded)
- 1 — one or more tests failed (including any that timed out under `--muda`)
- 2 — usage or config error (including a non-numeric or non-positive `--muda` value)

Success:
- discovers `#[jaribio]`-tagged functions
- compiles and executes tests
- prints summary; final line varies by output format (text: "majaribio yote yamefaulu" or "majaribio N yameshindwa"; JSON: structured result object)

Failures:
- compile/type-check failure
- runtime panic (`paparika`) in tests
- a test exceeding `--muda`'s timeout, when given
