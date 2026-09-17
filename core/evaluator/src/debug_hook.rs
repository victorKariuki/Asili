//! Real step-through debugging support: the `DebugHook` trait `pata-dap` drives a running
//! program through, plus `RealDebugHook`, the actual (not test-only) implementation the
//! interpreter's own statement-execution loop calls into (`eval_stmt_impl`, via
//! `Runtime::debug_hook`).
//!
//! Lives here (not in `pata-dap`) because `core/evaluator` must be able to implement this trait
//! without depending on `pata/dap` — this project's crates never have a `core/` crate depend on
//! a `pata/` one (`pata-dap` depends on `asili-evaluator`, the normal direction, matching
//! `pata-cli`/`pata-runner`'s existing precedent). `pata-dap` re-exports `DebugHook` from here
//! rather than defining its own copy, so both sides of the contract stay in sync by construction.

use crate::env::Env;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

/// A running program's debug-control surface — implemented by `RealDebugHook` below (wired into
/// the evaluator via `Runtime::debug_hook`) and, in `pata-dap`'s own test suite, by a canned test
/// double exercising the DAP protocol layer independent of a real running program.
pub trait DebugHook: Send + Sync {
    /// Called by `eval_stmt_impl` immediately before `should_pause`, with a fresh snapshot of
    /// every currently-in-scope binding (innermost-scope-wins on a shadowed name) — so a real
    /// pause always has up-to-date data ready for a `variables` request before it can possibly
    /// block. Takes an already-formatted snapshot rather than a live `&Env` reference, keeping
    /// this trait's shape independent of the evaluator's own internal `Env` type.
    fn record_bindings(&self, bindings: Vec<(String, String)>);
    /// Called before executing the statement at `line`. Returning `true` means the caller
    /// (`eval_stmt_impl`) should treat execution as having genuinely paused and later resumed —
    /// a real implementation blocks the calling thread internally (e.g. on a channel recv or a
    /// condvar wait) until `resume()` is called from another thread, rather than the caller
    /// polling in a loop.
    fn should_pause(&self, line: usize) -> bool;
    fn resume(&self);
    /// Snapshot of currently-in-scope bindings, for a `variables` DAP request — whatever was
    /// last passed to `record_bindings`.
    fn current_bindings(&self) -> Vec<(String, String)>;
}

/// The real, evaluator-side `DebugHook`: pauses at configured breakpoint lines, blocking the
/// executing thread on a `Mutex<bool>` + `Condvar` (the same primitive `pata-dap`'s own test
/// double used before a real implementation existed) until `resume()` is called from the DAP
/// server's own request-handling thread. `record_bindings` is called by `eval_stmt_impl` right
/// before `should_pause`, so a `variables` request issued while genuinely paused reflects the
/// real, live environment at the paused line — not canned data.
pub struct RealDebugHook {
    breakpoints: Mutex<Vec<usize>>,
    paused: Arc<(Mutex<bool>, std::sync::Condvar)>,
    bindings: Mutex<Vec<(String, String)>>,
    did_pause: AtomicBool,
    /// The line currently paused at, or `-1` when not paused — a caller (e.g. `pata-dap`'s
    /// launch-monitor thread) polls this to know when/where to report a real pause, since
    /// `should_pause` itself blocks the calling (interpreter) thread and can't be polled for its
    /// return value until it's already over.
    paused_at_line: AtomicI64,
}

impl RealDebugHook {
    pub fn new(breakpoints: Vec<usize>) -> Self {
        Self {
            breakpoints: Mutex::new(breakpoints),
            paused: Arc::new((Mutex::new(false), std::sync::Condvar::new())),
            bindings: Mutex::new(Vec::new()),
            did_pause: AtomicBool::new(false),
            paused_at_line: AtomicI64::new(-1),
        }
    }

    pub fn set_breakpoints(&self, lines: Vec<usize>) {
        *self.breakpoints.lock().unwrap() = lines;
    }

    /// Whether execution has genuinely paused at least once — real observability for a caller
    /// that wants to confirm a breakpoint actually fired, not just that `should_pause` was
    /// called (mirrors the same-purpose flag the earlier canned test double had).
    pub fn did_pause(&self) -> bool {
        self.did_pause.load(Ordering::SeqCst)
    }

    /// The line currently paused at, or `None` when execution isn't paused right now.
    pub fn paused_at_line(&self) -> Option<usize> {
        let line = self.paused_at_line.load(Ordering::SeqCst);
        if line < 0 { None } else { Some(line as usize) }
    }
}

impl Default for RealDebugHook {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Flatten `env`'s in-scope bindings into the `(name, debug-string)` pairs `DebugHook::
/// record_bindings` takes — innermost scope wins on a name collision, matching `Env::get`'s own
/// shadowing semantics. Called by `eval_stmt_impl` immediately before `should_pause`, so a real
/// pause always has an up-to-date snapshot passed in before it can possibly block. A free
/// function (not a trait method) so the trait itself stays independent of `Env`'s concrete type.
pub fn snapshot_bindings(env: &Env) -> Vec<(String, String)> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (name, value) in env.iter_innermost_first() {
        if seen.insert(name.clone()) {
            out.push((name, format!("{value:?}")));
        }
    }
    out
}

impl DebugHook for RealDebugHook {
    fn record_bindings(&self, bindings: Vec<(String, String)>) {
        *self.bindings.lock().unwrap() = bindings;
    }

    fn should_pause(&self, line: usize) -> bool {
        if !self.breakpoints.lock().unwrap().contains(&line) {
            return false;
        }

        self.did_pause.store(true, Ordering::SeqCst);
        self.paused_at_line.store(line as i64, Ordering::SeqCst);
        let (lock, cvar) = &*self.paused;
        let mut is_paused = lock.lock().unwrap();
        *is_paused = true;
        while *is_paused {
            is_paused = cvar.wait(is_paused).unwrap();
        }
        self.paused_at_line.store(-1, Ordering::SeqCst);
        true
    }

    fn resume(&self) {
        let (lock, cvar) = &*self.paused;
        let mut is_paused = lock.lock().unwrap();
        *is_paused = false;
        cvar.notify_all();
    }

    fn current_bindings(&self) -> Vec<(String, String)> {
        self.bindings.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn should_pause_returns_false_for_a_line_with_no_breakpoint() {
        let hook = RealDebugHook::new(vec![10]);
        assert!(!hook.should_pause(5));
        assert!(!hook.did_pause());
    }

    #[test]
    fn should_pause_blocks_until_resume_is_called_from_another_thread() {
        let hook = Arc::new(RealDebugHook::new(vec![7]));
        let hook_for_pause = Arc::clone(&hook);
        let pause_thread = std::thread::spawn(move || hook_for_pause.should_pause(7));

        for _ in 0..100 {
            if hook.did_pause() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(hook.did_pause(), "should_pause should have started blocking by now");

        hook.resume();
        assert!(pause_thread.join().unwrap());
    }

    #[test]
    fn snapshot_bindings_reflects_real_environment_state() {
        let mut env = Env::new();
        env.define("x", Value::Namba(42.0));
        let hook = RealDebugHook::new(vec![]);
        hook.record_bindings(snapshot_bindings(&env));

        let bindings = hook.current_bindings();
        assert!(bindings.iter().any(|(name, value)| name == "x" && value.contains("42")));
    }

    /// Shadowing: an inner scope's binding for a name must win over an outer scope's, matching
    /// `Env::get`'s own semantics — proves `snapshot_bindings` doesn't just dump every scope
    /// unconditionally and let a later (outer) duplicate silently overwrite the real value.
    #[test]
    fn snapshot_bindings_respects_inner_scope_shadowing() {
        let mut env = Env::new();
        env.define("x", Value::Namba(1.0));
        env.push_scope();
        env.define("x", Value::Namba(2.0));
        let hook = RealDebugHook::new(vec![]);
        hook.record_bindings(snapshot_bindings(&env));

        let bindings = hook.current_bindings();
        let x_values: Vec<_> = bindings.iter().filter(|(name, _)| name == "x").collect();
        assert_eq!(x_values.len(), 1, "shadowed name must appear once, not once per scope");
        assert!(x_values[0].1.contains('2'), "the inner (shadowing) value must win, got: {:?}", x_values[0]);
    }

    #[test]
    fn set_breakpoints_replaces_the_configured_lines() {
        let hook = RealDebugHook::new(vec![1, 2]);
        hook.set_breakpoints(vec![99]);
        assert!(!hook.should_pause(1));
    }

    #[test]
    fn paused_at_line_reports_none_before_and_after_a_pause_and_the_real_line_while_paused() {
        let hook = Arc::new(RealDebugHook::new(vec![9]));
        assert_eq!(hook.paused_at_line(), None, "not paused yet");

        let hook_for_pause = Arc::clone(&hook);
        let pause_thread = std::thread::spawn(move || hook_for_pause.should_pause(9));

        for _ in 0..100 {
            if hook.did_pause() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(hook.paused_at_line(), Some(9), "must report the real paused line while blocked");

        hook.resume();
        pause_thread.join().unwrap();
        assert_eq!(hook.paused_at_line(), None, "must report None again once resumed");
    }
}
