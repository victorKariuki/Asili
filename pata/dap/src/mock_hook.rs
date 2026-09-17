//! A fake `DebugHook`, real enough to exercise `pata-dap`'s protocol handling end-to-end
//! (breakpoints causing a real pause, `continue` causing a real resume, `variables` returning
//! real data) without actually running a target program — `core/evaluator`'s own
//! `debug_hook::RealDebugHook` (used by `pata-dap`'s real `launch` handling, see `runner.rs`) is
//! the implementation that drives an actual `.as` program. Kept as a real, always-buildable
//! module (not `#[cfg(test)]`-gated) so `pata-dap`'s protocol layer can still be exercised in a
//! "canned" mode independent of compiling/running a real program — useful for testing the DAP
//! wire format itself without needing a `.as` source file on disk.

use asili_evaluator::debug_hook::DebugHook;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

/// A fake debug session: pauses whenever the current line is in `breakpoints`, tracks
/// paused/resumed state with a real mutex + condvar (so `should_pause` genuinely blocks a
/// calling thread until `resume()` is called from another one — not a busy-loop or an
/// immediate no-op), and serves a fixed, canned set of variable bindings.
pub struct MockHook {
    breakpoints: Mutex<Vec<usize>>,
    paused: Arc<(Mutex<bool>, std::sync::Condvar)>,
    bindings: Mutex<Vec<(String, String)>>,
    /// Set once `should_pause` has actually blocked at least once — lets a test assert a real
    /// pause happened, not just that the method was called.
    did_pause: AtomicBool,
}

impl MockHook {
    pub fn new() -> Self {
        Self {
            breakpoints: Mutex::new(Vec::new()),
            paused: Arc::new((Mutex::new(false), std::sync::Condvar::new())),
            bindings: Mutex::new(vec![
                ("x".to_string(), "42".to_string()),
                ("jina".to_string(), "\"mfano\"".to_string()),
            ]),
            did_pause: AtomicBool::new(false),
        }
    }

    pub fn set_breakpoints(&self, lines: Vec<usize>) {
        *self.breakpoints.lock().unwrap() = lines;
    }

    pub fn set_bindings(&self, bindings: Vec<(String, String)>) {
        *self.bindings.lock().unwrap() = bindings;
    }

    pub fn did_pause(&self) -> bool {
        self.did_pause.load(Ordering::SeqCst)
    }
}

impl Default for MockHook {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugHook for MockHook {
    /// A no-op: `MockHook` serves fixed, canned bindings (set via `set_bindings`) regardless of
    /// what a real evaluator would have recorded — that's the whole point of a canned test
    /// double for exercising the DAP protocol layer independent of a real running program.
    fn record_bindings(&self, _bindings: Vec<(String, String)>) {}

    fn should_pause(&self, line: usize) -> bool {
        if !self.breakpoints.lock().unwrap().contains(&line) {
            return false;
        }

        self.did_pause.store(true, Ordering::SeqCst);
        let (lock, cvar) = &*self.paused;
        let mut is_paused = lock.lock().unwrap();
        *is_paused = true;
        // Block the calling thread until resume() flips this back to false and notifies —
        // real blocking, not a spin loop, matching what a genuine evaluator hook would need to
        // do (halt the interpreter thread at a breakpoint until the debugger says go).
        while *is_paused {
            is_paused = cvar.wait(is_paused).unwrap();
        }
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
        let hook = MockHook::new();
        hook.set_breakpoints(vec![10]);
        assert!(!hook.should_pause(5), "a line with no breakpoint must not pause");
        assert!(!hook.did_pause());
    }

    #[test]
    fn should_pause_blocks_until_resume_is_called_from_another_thread() {
        let hook = Arc::new(MockHook::new());
        hook.set_breakpoints(vec![7]);

        let hook_for_pause = Arc::clone(&hook);
        let pause_thread = std::thread::spawn(move || {
            // This call must genuinely block until resume() is called below.
            hook_for_pause.should_pause(7)
        });

        // Give the pause thread a real chance to reach and block in should_pause before we
        // resume it -- a short, bounded wait, not a race assumption.
        for _ in 0..100 {
            if hook.did_pause() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(hook.did_pause(), "should_pause should have started blocking by now");

        hook.resume();
        let result = pause_thread.join().unwrap();
        assert!(result, "should_pause must return true after actually pausing");
    }

    #[test]
    fn current_bindings_returns_the_canned_variables() {
        let hook = MockHook::new();
        let bindings = hook.current_bindings();
        assert!(bindings.iter().any(|(k, _)| k == "x"));
    }

    #[test]
    fn set_bindings_replaces_the_canned_variables() {
        let hook = MockHook::new();
        hook.set_bindings(vec![("y".to_string(), "7".to_string())]);
        let bindings = hook.current_bindings();
        assert_eq!(bindings, vec![("y".to_string(), "7".to_string())]);
    }
}
