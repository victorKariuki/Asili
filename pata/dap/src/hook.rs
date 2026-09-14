//! The contract `core/evaluator` must eventually implement for real step-through debugging.
//! `pata-dap` is built and tested against `MockHook` (see `mock_hook.rs`) so the protocol layer
//! is complete and correct independent of the evaluator work landing — see
//! `docs/design/pata-implementation-spec.md` Section 20 for the full design rationale (why this
//! boundary is drawn here, why `core/evaluator` isn't touched by this crate at all).

/// A running program's debug-control surface. `core/evaluator` doesn't implement this yet —
/// doing so needs a hook inside the interpreter's statement-execution loop (checking
/// `should_pause` before each statement, blocking on `resume` when it returns `true`), which is
/// real evaluator-level work outside this crate's scope.
pub trait DebugHook: Send + Sync {
    /// Called before executing the statement at `line`. Returning `true` pauses execution and
    /// blocks (e.g. on a channel recv) until `resume()` is called from another thread.
    fn should_pause(&self, line: usize) -> bool;
    fn resume(&self);
    /// Snapshot of currently-in-scope bindings, for a `variables` DAP request.
    fn current_bindings(&self) -> Vec<(String, String)>;
}
