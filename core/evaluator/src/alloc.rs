//! A small, lock-free-on-the-fast-path memory allocator for the standalone runner.
//!
//! Asili programs allocate many short-lived small blocks (strings, generic values, frames).
//! musl's allocator — what the static runner links — takes a lock and several atomics on every
//! call, which made string-building loops run nearly three times slower than on glibc. This
//! allocator serves every request of up to 4 KiB with alignment up to 16 from per-thread free
//! lists, one per size class: an allocation pops a list or bumps a pointer through a 64 KiB
//! chunk, a free pushes the block back. No locks, no atomics, no system call in the steady
//! state. Anything larger or more aligned goes to the system allocator.
//!
//! Blocks are never returned to the operating system; a block freed on another thread joins
//! that thread's lists (memory is memory). Peak usage per size class bounds the footprint, as
//! in most slab allocators.

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

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::UnsafeCell;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Bytes this allocator holds from the system (chunks and large blocks): the program's memory.
static SYSTEM_BYTES: AtomicUsize = AtomicUsize::new(0);
/// The memory limit in bytes (`kikomo_kumbukumbu`), 0 for none.
static LIMIT: AtomicUsize = AtomicUsize::new(0);
/// Set once the limit is passed; native code's host turns it into an error at its next call.
static OVER: AtomicBool = AtomicBool::new(false);

/// Limit the program's memory to `bytes` (0: no limit). Enforced where this allocator is the
/// global one (the standalone runner): passing it is an error at the host's next call, which
/// stops the program through its safe state. The limit is soft — the allocation that passes it
/// still succeeds, so the runtime never fails half-way through its own bookkeeping.
pub fn set_limit(bytes: usize) {
    LIMIT.store(bytes, Ordering::Relaxed);
    OVER.store(false, Ordering::Relaxed);
    note(0);
}

/// Bytes held from the system now.
pub fn system_bytes() -> usize {
    SYSTEM_BYTES.load(Ordering::Relaxed)
}

/// Whether the memory limit has been passed.
pub(crate) fn over_limit() -> bool {
    OVER.load(Ordering::Relaxed)
}

/// `delta` more bytes held from the system.
fn note(delta: usize) {
    let now = SYSTEM_BYTES.fetch_add(delta, Ordering::Relaxed) + delta;
    let limit = LIMIT.load(Ordering::Relaxed);
    if limit != 0 && now > limit {
        OVER.store(true, Ordering::Relaxed);
    }
}

fn released(bytes: usize) {
    SYSTEM_BYTES.fetch_sub(bytes, Ordering::Relaxed);
}

/// Block sizes, 16-byte multiples growing by about a quarter.
const SIZES: [usize; 28] = [
    16, 32, 48, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 448, 512, 640, 768, 896, 1024,
    1280, 1536, 1792, 2048, 2560, 3072, 3584, 4096,
];
const MAX_SMALL: usize = 4096;
const MAX_ALIGN: usize = 16;
const CHUNK: usize = 64 * 1024;

/// Size class for each 16-byte step of request size (`(size + 15) / 16`).
const CLASS_OF: [u8; MAX_SMALL / 16 + 1] = {
    let mut table = [0u8; MAX_SMALL / 16 + 1];
    let mut step = 0;
    let mut class = 0;
    while step <= MAX_SMALL / 16 {
        while SIZES[class] < step * 16 {
            class += 1;
        }
        table[step] = class as u8;
        step += 1;
    }
    table
};

struct Cache {
    /// Per size class: the most recently freed block (each free block starts with the next).
    free: [*mut u8; SIZES.len()],
    /// Unused rest of the current chunk.
    bump: *mut u8,
    end: *mut u8,
}

thread_local! {
    static CACHE: UnsafeCell<Cache> = const {
        UnsafeCell::new(Cache {
            free: [null_mut(); SIZES.len()],
            bump: null_mut(),
            end: null_mut(),
        })
    };
}

/// Size class of a request this allocator serves, or `None` for the system allocator.
#[inline(always)]
fn class(layout: &Layout) -> Option<usize> {
    (layout.size() <= MAX_SMALL && layout.align() <= MAX_ALIGN)
        .then(|| CLASS_OF[layout.size().div_ceil(16)] as usize)
}

/// The Asili allocator (see the module documentation). Install it with
/// `#[global_allocator] static A: asili_evaluator::alloc::AsiliAlloc = AsiliAlloc;`.
pub struct AsiliAlloc;

impl AsiliAlloc {
    /// A block of size class `c` from this thread's cache.
    #[inline(always)]
    unsafe fn take(c: usize) -> *mut u8 {
        CACHE.with(|cache| {
            // SAFETY: the cache is this thread's own, and nothing re-enters the allocator
            // while it is borrowed (refills call the system allocator, not this one).
            let cache = unsafe { &mut *cache.get() };
            let head = cache.free[c];
            if !head.is_null() {
                // SAFETY: a free block's first word links to the next free block.
                cache.free[c] = unsafe { *(head as *mut *mut u8) };
                return head;
            }
            let size = SIZES[c];
            if (cache.end as usize) - (cache.bump as usize) < size {
                // SAFETY: a valid, non-zero layout.
                let chunk = unsafe { System.alloc(Layout::from_size_align_unchecked(CHUNK, 16)) };
                if chunk.is_null() {
                    return null_mut();
                }
                note(CHUNK);
                cache.bump = chunk;
                // SAFETY: within the chunk just allocated.
                cache.end = unsafe { chunk.add(CHUNK) };
            }
            let block = cache.bump;
            // SAFETY: `size` bytes remain before `end`.
            cache.bump = unsafe { block.add(size) };
            block
        })
    }

    /// Return a block of size class `c` to this thread's cache.
    #[inline(always)]
    unsafe fn give(c: usize, block: *mut u8) {
        CACHE.with(|cache| {
            // SAFETY: as in `take`; the block is at least 16 bytes and 16-byte aligned.
            let cache = unsafe { &mut *cache.get() };
            unsafe { *(block as *mut *mut u8) = cache.free[c] };
            cache.free[c] = block;
        })
    }
}

// SAFETY: every block handed out is at least `layout.size()` bytes, aligned to 16 (chunks are
// 16-byte aligned and every class size is a multiple of 16) and so to `layout.align()`, and is
// not handed out again until it is freed; `dealloc` sees the same layout as `alloc`, so both
// agree on who owns a block.
unsafe impl GlobalAlloc for AsiliAlloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match class(&layout) {
            Some(c) => unsafe { Self::take(c) },
            None => {
                let p = unsafe { System.alloc(layout) };
                if !p.is_null() {
                    note(layout.size());
                }
                p
            }
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match class(&layout) {
            Some(c) => unsafe { Self::give(c, ptr) },
            None => {
                released(layout.size());
                unsafe { System.dealloc(ptr, layout) }
            }
        }
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller guarantees `new_size` is valid for `layout.align()`.
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        match (class(&layout), class(&new_layout)) {
            // Same block size: nothing to move.
            (Some(a), Some(b)) if a == b => ptr,
            (None, None) => {
                let p = unsafe { System.realloc(ptr, layout, new_size) };
                if !p.is_null() {
                    released(layout.size());
                    note(new_size);
                }
                p
            }
            _ => {
                let new = unsafe { self.alloc(new_layout) };
                if !new.is_null() {
                    unsafe {
                        std::ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size));
                        self.dealloc(ptr, layout);
                    }
                }
                new
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_classes_fit_their_requests() {
        for size in 1..=MAX_SMALL {
            let c = CLASS_OF[size.div_ceil(16)] as usize;
            assert!(SIZES[c] >= size, "{size} -> {}", SIZES[c]);
            assert!(
                c == 0 || SIZES[c - 1] < size,
                "{size} not in the smallest class"
            );
        }
    }

    #[test]
    fn blocks_are_distinct_aligned_and_reused() {
        let a = AsiliAlloc;
        let layouts: Vec<Layout> = [1, 16, 17, 100, 4096, 4097, 100_000]
            .iter()
            .map(|&n| Layout::from_size_align(n, 8).unwrap())
            .collect();
        let mut blocks = Vec::new();
        for _ in 0..3 {
            for l in &layouts {
                let p = unsafe { a.alloc(*l) };
                assert!(!p.is_null() && (p as usize).is_multiple_of(8));
                unsafe { std::ptr::write_bytes(p, 0xAB, l.size()) };
                blocks.push((p, *l));
            }
        }
        let mut addrs: Vec<usize> = blocks.iter().map(|(p, _)| *p as usize).collect();
        addrs.sort_unstable();
        addrs.dedup();
        assert_eq!(addrs.len(), blocks.len(), "a block was handed out twice");
        let (p, l) = blocks[1];
        unsafe { a.dealloc(p, l) };
        assert_eq!(unsafe { a.alloc(l) }, p, "a freed block is reused first");
        // Growing within a class keeps the block; across classes keeps the contents.
        let l = Layout::from_size_align(20, 8).unwrap();
        let p = unsafe { a.alloc(l) };
        unsafe { std::ptr::write_bytes(p, 7, 20) };
        assert_eq!(unsafe { a.realloc(p, l, 30) }, p);
        let q = unsafe { a.realloc(p, Layout::from_size_align(30, 8).unwrap(), 5000) };
        assert!((0..20).all(|i| unsafe { *q.add(i) } == 7));
        unsafe { a.dealloc(q, Layout::from_size_align(5000, 8).unwrap()) };
    }
}
