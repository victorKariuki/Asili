//! A fake `DebugHook`, real enough to exercise `pata-dap`'s protocol handling end-to-end
//! (breakpoints causing a real pause, `continue` causing a real resume, `variables` returning
//! real data) without actually running a target program — `core/evaluator`'s own
//! `debug_hook::RealDebugHook` (used by `pata-dap`'s real `launch` handling, see `runner.rs`) is
//! the implementation that drives an actual `.as` program. Kept as a real, always-buildable
//! module (not `#[cfg(test)]`-gated) so `pata-dap`'s protocol layer can still be exercised in a
//! "canned" mode independent of compiling/running a real program — useful for testing the DAP
//! wire format itself without needing a `.as` source file on disk.

use asili_evaluator::debug_hook::{DebugHook, RealDebugHook};
use std::sync::Mutex;

/// A fake debug session: pauses exactly like `RealDebugHook` (it wraps one — a real mutex +
/// condvar, so `should_pause` genuinely blocks until `resume()` is called from another thread),
/// but serves a fixed, canned set of variable bindings instead of the evaluator's live ones.
pub struct MockHook {
    gate: RealDebugHook,
    bindings: Mutex<Vec<(String, String)>>,
}

impl MockHook {
    pub fn new() -> Self {
        Self {
            gate: RealDebugHook::default(),
            bindings: Mutex::new(vec![
                ("x".to_string(), "42".to_string()),
                ("jina".to_string(), "\"mfano\"".to_string()),
            ]),
        }
    }

    pub fn set_breakpoints(&self, lines: Vec<usize>) {
        self.gate.set_breakpoints(lines);
    }

    pub fn set_bindings(&self, bindings: Vec<(String, String)>) {
        *self.bindings.lock().unwrap() = bindings;
    }

    pub fn did_pause(&self) -> bool {
        self.gate.did_pause()
    }
}

impl Default for MockHook {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugHook for MockHook {
    /// A no-op: `MockHook` serves fixed, canned bindings (set via `set_bindings`) regardless of
    /// what a real evaluator would have recorded.
    fn record_bindings(&self, _bindings: Vec<(String, String)>) {}

    fn should_pause(&self, line: usize) -> bool {
        self.gate.should_pause(line)
    }

    fn resume(&self) {
        self.gate.resume()
    }

    fn current_bindings(&self) -> Vec<(String, String)> {
        self.bindings.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn should_pause_returns_false_for_a_line_with_no_breakpoint() {
        let hook = MockHook::new();
        hook.set_breakpoints(vec![10]);
        assert!(
            !hook.should_pause(5),
            "a line with no breakpoint must not pause"
        );
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
        assert!(
            hook.did_pause(),
            "should_pause should have started blocking by now"
        );

        hook.resume();
        let result = pause_thread.join().unwrap();
        assert!(
            result,
            "should_pause must return true after actually pausing"
        );
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
