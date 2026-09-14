# `pata jaribu`

Purpose: discover and execute test suites.

Fixtures: a `#[kabla]`-tagged function runs immediately before every `#[jaribio]` test in the
same module (source file); a `#[baada]`-tagged function runs immediately after — including when
the test itself failed, since teardown exists to release resources setup acquired regardless of
outcome. A `#[kabla]` failure fails the test without running its body, naming the fixture in the
message (`kabla '<name>' imeshindwa: ...`); a `#[baada]` failure fails an otherwise-passing test
the same way (`baada '<name>' imeshindwa: ...`) — a test's own failure always takes precedence
over a teardown failure's message. Multiple `#[baada]` functions in the same module all run even
if an earlier one fails. Fixtures are module-scoped, not project-wide or per-test-function.

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
