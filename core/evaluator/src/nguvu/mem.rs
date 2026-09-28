//! Executable memory for generated code: mapped writable, filled, then flipped to read+execute
//! (never writable and executable at once).

pub struct ExecMem {
    ptr: *mut u8,
    len: usize,
}

// SAFETY: the mapping is immutable (read+execute) once constructed and owned by this value.
unsafe impl Send for ExecMem {}
unsafe impl Sync for ExecMem {}

impl ExecMem {
    /// Map `code` executable. Linux/BSD: an anonymous read-write mapping flipped to
    /// read-execute. macOS: a `MAP_JIT` mapping (what the hardened runtime and Apple silicon's
    /// W^X require), written with JIT write protection lifted for this thread, falling back to
    /// the plain path. Windows: `VirtualAlloc` read-write, then `VirtualProtect` to
    /// execute-read.
    pub fn new(code: &[u8]) -> Result<Self, String> {
        let len = code.len().max(1);
        #[cfg(target_os = "macos")]
        if let Some(mem) = apple::map_jit(code, len) {
            return Ok(mem);
        }
        #[cfg(unix)]
        {
            // SAFETY: plain anonymous private mapping; checked for failure below.
            let ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    len,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANON,
                    -1,
                    0,
                )
            };
            if ptr == libc::MAP_FAILED {
                return Err("mmap imeshindwa".into());
            }
            let ptr = ptr as *mut u8;
            // SAFETY: `ptr` is a fresh writable mapping of at least `len >= code.len()` bytes.
            unsafe {
                std::ptr::copy_nonoverlapping(code.as_ptr(), ptr, code.len());
                sync_instruction_cache(ptr, code.len());
                if libc::mprotect(ptr as *mut _, len, libc::PROT_READ | libc::PROT_EXEC) != 0 {
                    libc::munmap(ptr as *mut _, len);
                    return Err("mprotect imeshindwa".into());
                }
            }
            Ok(ExecMem { ptr, len })
        }
        #[cfg(windows)]
        {
            // SAFETY: a fresh private allocation, checked for failure; the copy stays within it.
            unsafe {
                let ptr = windows::VirtualAlloc(
                    std::ptr::null_mut(),
                    len,
                    windows::MEM_COMMIT | windows::MEM_RESERVE,
                    windows::PAGE_READWRITE,
                ) as *mut u8;
                if ptr.is_null() {
                    return Err("VirtualAlloc imeshindwa".into());
                }
                std::ptr::copy_nonoverlapping(code.as_ptr(), ptr, code.len());
                let mut old = 0u32;
                if windows::VirtualProtect(ptr as *mut _, len, windows::PAGE_EXECUTE_READ, &mut old)
                    == 0
                {
                    windows::VirtualFree(ptr as *mut _, 0, windows::MEM_RELEASE);
                    return Err("VirtualProtect imeshindwa".into());
                }
                windows::FlushInstructionCache(windows::GetCurrentProcess(), ptr as *const _, len);
                Ok(ExecMem { ptr, len })
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (code, len);
            Err("msimbo asilia wa ndani bado haupatikani kwenye mfumo huu".into())
        }
    }

    /// Address of byte `offset` of the code.
    pub fn at(&self, offset: usize) -> *const u8 {
        debug_assert!(offset < self.len);
        // SAFETY: in bounds of the mapping.
        unsafe { self.ptr.add(offset) }
    }
}

impl Drop for ExecMem {
    fn drop(&mut self) {
        // SAFETY: releasing exactly the region `new` mapped.
        #[cfg(unix)]
        unsafe {
            libc::munmap(self.ptr as *mut _, self.len);
        }
        #[cfg(windows)]
        unsafe {
            windows::VirtualFree(self.ptr as *mut _, 0, windows::MEM_RELEASE);
        }
    }
}

#[cfg(target_os = "macos")]
mod apple {
    use super::ExecMem;
    use std::ffi::{c_int, c_void};

    extern "C" {
        /// Per-thread switch between writing and executing `MAP_JIT` memory (Apple silicon).
        fn pthread_jit_write_protect_np(enabled: c_int);
        fn sys_icache_invalidate(start: *mut c_void, len: usize);
    }

    const MAP_JIT: c_int = 0x0800;

    pub(super) fn map_jit(code: &[u8], len: usize) -> Option<ExecMem> {
        // SAFETY: anonymous private mapping checked for failure; written only while this
        // thread has JIT write protection lifted, then made executable again.
        unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                libc::MAP_PRIVATE | libc::MAP_ANON | MAP_JIT,
                -1,
                0,
            );
            if ptr == libc::MAP_FAILED {
                return None; // no JIT entitlement under a hardened runtime: try the plain path
            }
            let ptr = ptr as *mut u8;
            if cfg!(target_arch = "aarch64") {
                pthread_jit_write_protect_np(0);
            }
            std::ptr::copy_nonoverlapping(code.as_ptr(), ptr, code.len());
            if cfg!(target_arch = "aarch64") {
                pthread_jit_write_protect_np(1);
            }
            sys_icache_invalidate(ptr as *mut c_void, code.len());
            Some(ExecMem { ptr, len })
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;

    pub const MEM_COMMIT: u32 = 0x1000;
    pub const MEM_RESERVE: u32 = 0x2000;
    pub const MEM_RELEASE: u32 = 0x8000;
    pub const PAGE_READWRITE: u32 = 0x04;
    pub const PAGE_EXECUTE_READ: u32 = 0x20;

    #[link(name = "kernel32")]
    extern "system" {
        pub fn VirtualAlloc(addr: *mut c_void, size: usize, kind: u32, protect: u32)
            -> *mut c_void;
        pub fn VirtualProtect(addr: *mut c_void, size: usize, protect: u32, old: *mut u32) -> i32;
        pub fn VirtualFree(addr: *mut c_void, size: usize, kind: u32) -> i32;
        pub fn GetCurrentProcess() -> *mut c_void;
        pub fn FlushInstructionCache(process: *mut c_void, addr: *const c_void, size: usize)
            -> i32;
    }
}

/// Make freshly written code visible to instruction fetch. x86-64 keeps its caches coherent;
/// AArch64 does not: clean the data cache to the point of unification, invalidate the
/// instruction cache, then synchronize.
///
/// # Safety
/// `ptr..ptr + len` must be mapped.
#[cfg(all(unix, target_arch = "aarch64"))]
unsafe fn sync_instruction_cache(ptr: *mut u8, len: usize) {
    use std::arch::asm;
    let ctr: u64;
    asm!("mrs {}, ctr_el0", out(reg) ctr, options(nomem, nostack));
    let dline = 4usize << ((ctr >> 16) & 0xF);
    let iline = 4usize << (ctr & 0xF);
    let (start, end) = (ptr as usize, ptr as usize + len);
    let mut a = start & !(dline - 1);
    while a < end {
        asm!("dc cvau, {}", in(reg) a, options(nostack));
        a += dline;
    }
    asm!("dsb ish", options(nostack));
    let mut a = start & !(iline - 1);
    while a < end {
        asm!("ic ivau, {}", in(reg) a, options(nostack));
        a += iline;
    }
    asm!("dsb ish", "isb", options(nostack));
}

#[cfg(all(unix, not(target_arch = "aarch64")))]
unsafe fn sync_instruction_cache(_ptr: *mut u8, _len: usize) {}
