# DAP (Debug Adapter Protocol)

**Protocol layer implemented; real step-through debugging still blocked on `core/evaluator`.**

`pata/dap` is a real, tested binary crate (`pata-dap`) implementing the minimum viable DAP
surface: `initialize`, `launch`, `setBreakpoints`, `configurationDone`, `continue`, `stackTrace`,
`scopes`, `variables`, `threads`, `disconnect` — not the full DAP spec (no `stepIn`/`stepOut`,
watch expressions, or conditional breakpoints in this pass). It runs over real stdio using the
`dap` crate for wire-protocol (de)serialization, structurally mirroring `pata-lsp/src/server.rs`'s
shape even though DAP and LSP are different protocols.

What's real today: the full request/response/event protocol handling, verified end-to-end
against real `Content-Length`-framed wire bytes (`pata/dap/src/server.rs`'s own tests), and
manually against the compiled `pata-dap` binary over a live stdio pipe.

What's still missing: `pata-dap` runs against `MockHook` (`pata/dap/src/mock_hook.rs`), a real
but canned fake — breakpoints, pause/resume, and variable inspection all work against fixed data,
not a running Asili program. Genuine step-through debugging needs `core/evaluator` to implement
the `DebugHook` trait (`pata/dap/src/hook.rs`):

```rust
pub trait DebugHook: Send + Sync {
    fn should_pause(&self, line: usize) -> bool;
    fn resume(&self);
    fn current_bindings(&self) -> Vec<(String, String)>;
}
```

This needs a hook inside the interpreter's own statement-execution loop (checking `should_pause`
before each statement, blocking on `resume` when it returns `true`) — real evaluator-level work,
out of `pata/`'s scope per `docs/design/pata-production-readiness.md`'s stated boundary
(`core/` treated as a given). Once that lands, wiring it into `pata-dap` in place of `MockHook`
is a small, mechanical follow-up, not a redesign — `pata-dap`'s server/session layer is already
written generically over any `H: DebugHook`.

See [docs/design/implementation-status.md](implementation-status.md) for where this sits relative
to other work, and `docs/design/pata-implementation-spec.md` Section 20 for the full original
design rationale (why a separate crate, why the trait is scoped this narrowly, what was
deliberately left out of this first pass).
