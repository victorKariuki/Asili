# DAP (Debug Adapter Protocol)

**Real step-through debugging works end-to-end.**

`pata/dap` is a real, tested binary crate (`pata-dap`) implementing the minimum viable DAP
surface: `initialize`, `launch`, `setBreakpoints`, `configurationDone`, `continue`, `stackTrace`,
`scopes`, `variables`, `threads`, `disconnect` — not the full DAP spec (no `stepIn`/`stepOut`,
watch expressions, or conditional breakpoints in this pass). It runs over real stdio using the
`dap` crate for wire-protocol (de)serialization, structurally mirroring `pata-lsp/src/server.rs`'s
shape even though DAP and LSP are different protocols.

## What's real

- The full request/response/event protocol handling, verified end-to-end against real
  `Content-Length`-framed wire bytes (`pata/dap/src/server.rs`'s own tests).
- **A real `DebugHook` implementation, in `core/evaluator` itself**
  (`asili_evaluator::debug_hook::RealDebugHook`), wired directly into the interpreter's
  statement-execution loop (`eval_stmt_impl`, via `Runtime::debug_hook`) — a breakpoint genuinely
  pauses the executing thread (blocking on a `Mutex<bool>` + `Condvar`, not polling), `resume()`
  genuinely unblocks it from another thread, and `current_bindings()` reflects the real, live
  `Env` at the paused line (snapshotted immediately before the pause check).
- **`pata-dap` actually launches and runs the target program.** A `launch` naming a `.as` file,
  followed by `setBreakpoints` and `configurationDone`, compiles that file (lex → parse — single
  file only, no project/dependency resolution) and runs it with `RealDebugHook` attached via
  `asili_evaluator::run_main_with_debug_hook` (`pata/dap/src/runner.rs`). A lightweight monitor
  thread mirrors the hook's real paused-line into the session's own `stackTrace`-facing state.
- Verified two ways: `pata/dap/src/runner.rs`'s own integration test drives a real `launch`/
  `setBreakpoints`/`configurationDone` sequence against a real temp `.as` file and asserts the
  program genuinely pauses at the configured line; and manually, against the compiled `pata-dap`
  binary over a live stdio pipe with a full `initialize`→`launch`→`setBreakpoints`→
  `configurationDone`→`stackTrace`(polled until paused)→`variables`→`continue`→`disconnect`
  session — `variables` returned real bound values (`a: "Namba(1.0)"`, matching the program having
  already executed `weka a = 1` but not yet the breakpointed `weka b = 2`), not canned data.

## Where `DebugHook` lives, and why

The trait moved from `pata/dap/src/hook.rs` into `core/evaluator/src/debug_hook.rs`. `core/`
crates never depend on `pata/` crates in this project — but `core/evaluator` is the crate that
needs to *implement* the trait (inside its own statement-execution loop), so the trait itself has
to live somewhere `core/evaluator` can reach without a `pata/` dependency. `pata-dap` depends on
`asili-evaluator` instead (the normal direction — matching `pata-cli`/`pata-runner`'s existing
precedent) and re-exports `DebugHook` from there rather than keeping a second copy.

```rust
pub trait DebugHook: Send + Sync {
    fn record_bindings(&self, bindings: Vec<(String, String)>);
    fn should_pause(&self, line: usize) -> bool;
    fn resume(&self);
    fn current_bindings(&self) -> Vec<(String, String)>;
}
```

(`record_bindings` is new relative to the original sketch — the evaluator pushes a fresh snapshot
in immediately before `should_pause` might block, since the trait method takes no live `&Env`
reference of its own.)

`MockHook` (`pata/dap/src/mock_hook.rs`) still exists and is still useful: a canned test double
for exercising the DAP wire protocol independent of compiling/running a real program.

## What's still out of scope

No `stepIn`/`stepOut`, watch expressions, or conditional breakpoints — breakpoints are
line-only, statement-granularity. `launch` compiles a single file only (no `leta`-import
resolution via `pata-core`, no `pata.toml`-aware project build) — debugging a multi-file project
would need that extension. See `docs/design/pata-implementation-spec.md` Section 20 for the
original design rationale (why a separate crate, what was deliberately left out of the first
pass) and `docs/design/implementation-status.md` for where this sits relative to other work.
