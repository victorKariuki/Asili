//! Real step-through debugging support: the `DebugHook` trait `pata-dap` drives a running
//! program through, plus `RealDebugHook`, the actual (not test-only) implementation the
//! native code's host calls into at every statement of a debug build (`Opcode::Line`).
//!
//! Lives here (not in `pata-dap`) because `core/evaluator` must be able to implement this trait
//! without depending on `pata/dap` — this project's crates never have a `core/` crate depend on
//! a `pata/` one (`pata-dap` depends on `asili-evaluator`, the normal direction, matching
//! `pata-cli`/`pata-runner`'s existing precedent). `pata-dap` re-exports `DebugHook` from here
//! rather than defining its own copy, so both sides of the contract stay in sync by construction.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

/// A running program's debug-control surface — implemented by `RealDebugHook` below (wired into
/// native code's host by `run_main_with_debug_hook`) and, in `pata-dap`'s own test suite, by a canned test
/// double exercising the DAP protocol layer independent of a real running program.
pub trait DebugHook: Send + Sync {
    /// Called by native code's host immediately before `should_pause`, with a fresh snapshot of
    /// every currently-in-scope binding (innermost-scope-wins on a shadowed name) — so a real
    /// pause always has up-to-date data ready for a `variables` request before it can possibly
    /// block. Takes an already-formatted snapshot rather than live registers, keeping
    /// this trait's shape independent of the host's frames.
    fn record_bindings(&self, bindings: Vec<(String, String)>);
    /// Called before executing the statement at `line`. Returning `true` means the caller
    /// (the host) should treat execution as having genuinely paused and later resumed —
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
/// server's own request-handling thread. `record_bindings` is called by the host right
/// before `should_pause`, so a `variables` request issued while genuinely paused reflects the
/// real, live environment at the paused line — not canned data.
pub struct RealDebugHook {
    breakpoints: Mutex<Vec<usize>>,
    paused: Arc<(Mutex<bool>, std::sync::Condvar)>,
    bindings: Mutex<Vec<(String, String)>>,
    did_pause: AtomicBool,
    /// The line currently paused at, or `-1` when not paused — a caller (e.g. `pata-dap`'s
    /// launch-monitor thread) polls this to know when/where to report a real pause, since
    /// `should_pause` itself blocks the calling (program) thread and can't be polled for its
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
        if line < 0 {
            None
        } else {
            Some(line as usize)
        }
    }
}

impl Default for RealDebugHook {
    fn default() -> Self {
        Self::new(Vec::new())
    }
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
        assert!(
            hook.did_pause(),
            "should_pause should have started blocking by now"
        );

        hook.resume();
        assert!(pause_thread.join().unwrap());
    }
}
