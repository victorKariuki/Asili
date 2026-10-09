//! Locks that never crash the program: a lock whose holder panicked (poisoned) is still taken —
//! the data it guards (handler tables, channels, thread handles, debugger state) stays usable,
//! and one failed thread must not bring the others down with it.

// Runtime code never panics on its own: an impossible state is an error the program sees
// (and its safe state handles), not a crash (see docs/design/safety-critical-roadmap.md §3).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Take `mutex`, recovering it if a thread panicked while holding it.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
