//! How much stack the running code has left, and growing it when short. `stacker` answers this
//! for a thread's own stack; a `sawia` task runs on a stack of its own (`kazi_sawia`), where
//! `stacker`'s per-thread guess is wrong, so the task scheduler sets the running stack's limit
//! here at every switch and these functions use it.

use std::cell::Cell;

thread_local! {
    /// Lowest usable address of the stack now running, when it is not the thread's own.
    static LIMIT: Cell<Option<usize>> = const { Cell::new(None) };
}

/// The stack pointer, approximately (the address of a local).
#[inline(always)]
fn sp() -> usize {
    let marker = 0u8;
    std::hint::black_box(&marker) as *const u8 as usize
}

/// Bytes of stack left below the current frame (`None` when unknown).
pub(crate) fn remaining() -> Option<usize> {
    match LIMIT.with(Cell::get) {
        Some(limit) => Some(sp().saturating_sub(limit)),
        None => stacker::remaining_stack(),
    }
}

/// Make `limit` the running stack's lowest usable address (`None`: the thread's own stack) and
/// return the previous one, for the caller to put back.
pub(crate) fn set_limit(limit: Option<usize>) -> Option<usize> {
    LIMIT.with(|l| l.replace(limit))
}

/// `f`, on a fresh segment of `size` bytes when fewer than `red_zone` are left.
pub(crate) fn maybe_grow<R>(red_zone: usize, size: usize, f: impl FnOnce() -> R) -> R {
    if remaining().is_some_and(|left| left >= red_zone) {
        return f();
    }
    stacker::grow(size, || {
        // The new segment's top is about here; keep a page of slack at its bottom.
        let previous = set_limit(Some(sp().saturating_sub(size) + 4096));
        let out = f();
        set_limit(previous);
        out
    })
}
