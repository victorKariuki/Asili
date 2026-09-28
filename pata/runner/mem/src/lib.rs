//! Memory primitives for the statically linked runner.
//!
//! musl's x86-64 `memcpy` copies byte by byte up to alignment and then with `rep movsq`, and
//! its `memcmp` compares one byte at a time — fine for large blocks, slow for the short strings
//! and values Asili programs copy constantly (a string-building loop ran twice as slow as on
//! glibc). These versions handle up to 64 bytes with a few overlapping unaligned loads and
//! stores and leave longer copies to `rep movsb`/`rep stosb`, which modern x86 CPUs run at
//! full speed. On x86-64 musl the runner links these in place of musl's (a static link never
//! pulls a symbol from `libc.a` that the program already defines); elsewhere they are unused
//! and the platform's own apply.
//!
//! `no_builtins` keeps the compiler from recognizing these loops as `memcpy` calls — which
//! would be calls to themselves.
#![no_builtins]
#![allow(clippy::missing_safety_doc)]

use core::ptr::{read_unaligned as ld, write_unaligned as st};

/// Copy `n` bytes from `src` to `dst`. Every load happens before the first store for `n <= 64`,
/// so short copies are also correct when the ranges overlap.
///
/// # Safety
/// `src` must be readable and `dst` writable for `n` bytes.
#[inline(always)]
unsafe fn copy_small(dst: *mut u8, src: *const u8, n: usize) {
    unsafe {
        if n >= 32 {
            // 32..=64: two 16-byte pairs from each end.
            let a: u128 = ld(src as *const u128);
            let b: u128 = ld(src.add(16) as *const u128);
            let c: u128 = ld(src.add(n - 32) as *const u128);
            let d: u128 = ld(src.add(n - 16) as *const u128);
            st(dst as *mut u128, a);
            st(dst.add(16) as *mut u128, b);
            st(dst.add(n - 32) as *mut u128, c);
            st(dst.add(n - 16) as *mut u128, d);
        } else if n >= 16 {
            let a: u128 = ld(src as *const u128);
            let b: u128 = ld(src.add(n - 16) as *const u128);
            st(dst as *mut u128, a);
            st(dst.add(n - 16) as *mut u128, b);
        } else if n >= 8 {
            let a: u64 = ld(src as *const u64);
            let b: u64 = ld(src.add(n - 8) as *const u64);
            st(dst as *mut u64, a);
            st(dst.add(n - 8) as *mut u64, b);
        } else if n >= 4 {
            let a: u32 = ld(src as *const u32);
            let b: u32 = ld(src.add(n - 4) as *const u32);
            st(dst as *mut u32, a);
            st(dst.add(n - 4) as *mut u32, b);
        } else if n > 0 {
            let a = *src;
            let b = *src.add(n / 2);
            let c = *src.add(n - 1);
            *dst = a;
            *dst.add(n / 2) = b;
            *dst.add(n - 1) = c;
        }
    }
}

/// Forward copy, safe when `dst <= src` even if the ranges overlap.
///
/// # Safety
/// As for `memcpy`.
#[inline(always)]
unsafe fn copy_forward(dst: *mut u8, src: *const u8, n: usize) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
            "rep movsb",
            inout("rcx") n => _,
            inout("rdi") dst => _,
            inout("rsi") src => _,
            options(nostack, preserves_flags)
        );
    }
    #[cfg(not(target_arch = "x86_64"))]
    unsafe {
        for i in 0..n {
            *dst.add(i) = *src.add(i);
        }
    }
}

/// Backward copy, for overlapping ranges with `dst > src`.
///
/// # Safety
/// As for `memmove`.
#[inline(always)]
unsafe fn copy_backward(dst: *mut u8, src: *const u8, n: usize) {
    unsafe {
        let mut i = n;
        while i >= 8 {
            i -= 8;
            let v: u64 = ld(src.add(i) as *const u64);
            st(dst.add(i) as *mut u64, v);
        }
        while i > 0 {
            i -= 1;
            *dst.add(i) = *src.add(i);
        }
    }
}

/// # Safety
/// As for C's `memcpy`.
#[inline]
pub unsafe fn copy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe {
        if n <= 64 {
            copy_small(dst, src, n);
        } else {
            copy_forward(dst, src, n);
        }
    }
    dst
}

/// # Safety
/// As for C's `memmove`.
#[inline]
pub unsafe fn moved(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe {
        if n <= 64 {
            copy_small(dst, src, n);
        } else if (dst as usize).wrapping_sub(src as usize) >= n {
            // `dst` before `src`, or no overlap: forward is safe.
            copy_forward(dst, src, n);
        } else {
            copy_backward(dst, src, n);
        }
    }
    dst
}

/// # Safety
/// As for C's `memset`.
#[inline]
pub unsafe fn set(dst: *mut u8, c: i32, n: usize) -> *mut u8 {
    let b = c as u8;
    unsafe {
        if n <= 32 {
            let w = u64::from_ne_bytes([b; 8]);
            if n >= 16 {
                st(dst as *mut u64, w);
                st(dst.add(8) as *mut u64, w);
                st(dst.add(n - 16) as *mut u64, w);
                st(dst.add(n - 8) as *mut u64, w);
            } else if n >= 8 {
                st(dst as *mut u64, w);
                st(dst.add(n - 8) as *mut u64, w);
            } else if n >= 4 {
                st(dst as *mut u32, w as u32);
                st(dst.add(n - 4) as *mut u32, w as u32);
            } else if n > 0 {
                *dst = b;
                *dst.add(n / 2) = b;
                *dst.add(n - 1) = b;
            }
        } else {
            #[cfg(target_arch = "x86_64")]
            core::arch::asm!(
                "rep stosb",
                inout("rcx") n => _,
                inout("rdi") dst => _,
                in("al") b,
                options(nostack, preserves_flags)
            );
            #[cfg(not(target_arch = "x86_64"))]
            for i in 0..n {
                *dst.add(i) = b;
            }
        }
    }
    dst
}

/// # Safety
/// As for C's `memcmp`.
#[inline]
pub unsafe fn compare(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0;
    unsafe {
        while i + 8 <= n {
            let x: u64 = ld(a.add(i) as *const u64);
            let y: u64 = ld(b.add(i) as *const u64);
            if x != y {
                // Read as big-endian numbers, the words order like their bytes do.
                let x = u64::from_be_bytes(x.to_ne_bytes());
                let y = u64::from_be_bytes(y.to_ne_bytes());
                return if x < y { -1 } else { 1 };
            }
            i += 8;
        }
        while i < n {
            let (x, y) = (*a.add(i), *b.add(i));
            if x != y {
                return x as i32 - y as i32;
            }
            i += 1;
        }
    }
    0
}

#[cfg(all(target_env = "musl", target_arch = "x86_64", not(test)))]
mod exports {
    #[no_mangle]
    pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
        unsafe { super::copy(dst, src, n) }
    }

    #[no_mangle]
    pub unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
        unsafe { super::moved(dst, src, n) }
    }

    #[no_mangle]
    pub unsafe extern "C" fn memset(dst: *mut u8, c: i32, n: usize) -> *mut u8 {
        unsafe { super::set(dst, c, n) }
    }

    #[no_mangle]
    pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
        unsafe { super::compare(a, b, n) }
    }

    #[no_mangle]
    pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
        unsafe { super::compare(a, b, n) }
    }
}

/// Link anchor: referencing it keeps this crate (and its exported symbols) in the binary.
pub fn linked() {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(n: usize, seed: u8) -> Vec<u8> {
        (0..n)
            .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
            .collect()
    }

    #[test]
    fn copies_and_sets_match_the_reference() {
        for n in 0..300 {
            for off in 0..9 {
                let src = pattern(n + 16, 7);
                let mut dst = vec![0u8; n + 16];
                unsafe { copy(dst.as_mut_ptr().add(off), src.as_ptr().add(3), n) };
                assert_eq!(&dst[off..off + n], &src[3..3 + n], "copy {n} at {off}");
                assert!(
                    dst[..off].iter().all(|&b| b == 0) && dst[off + n..].iter().all(|&b| b == 0)
                );
                let mut m = vec![1u8; n + 16];
                unsafe { set(m.as_mut_ptr().add(off), 0xAB, n) };
                assert!(m[off..off + n].iter().all(|&b| b == 0xAB), "set {n}");
                assert!(m[..off].iter().all(|&b| b == 1) && m[off + n..].iter().all(|&b| b == 1));
            }
        }
    }

    #[test]
    fn overlapping_moves_match_the_reference() {
        for n in 0..300 {
            for shift in [1usize, 3, 8, 17, 64, 65] {
                // dst after src and dst before src, overlapping.
                for forward in [false, true] {
                    let mut buf = pattern(n + 80, 5);
                    let mut want = buf.clone();
                    let (d, s) = if forward { (0, shift) } else { (shift, 0) };
                    want.copy_within(s..s + n, d);
                    unsafe { moved(buf.as_mut_ptr().add(d), buf.as_ptr().add(s), n) };
                    assert_eq!(buf, want, "move {n} shift {shift} forward {forward}");
                }
            }
        }
    }

    #[test]
    fn comparisons_match_the_reference() {
        for n in 0..40 {
            let a = pattern(n, 1);
            for k in 0..n {
                for delta in [1u8, 128, 255] {
                    let mut b = a.clone();
                    b[k] = b[k].wrapping_add(delta);
                    let want = a.cmp(&b) as i32;
                    let got = unsafe { compare(a.as_ptr(), b.as_ptr(), n) }.signum();
                    assert_eq!(got, want, "n {n} k {k} delta {delta}");
                }
            }
            assert_eq!(unsafe { compare(a.as_ptr(), a.as_ptr(), n) }, 0);
        }
    }
}
