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
    #[cfg(unix)]
    pub fn new(code: &[u8]) -> Result<Self, String> {
        let len = code.len().max(1);
        // SAFETY: plain anonymous private mapping; checked for failure below.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
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
            if libc::mprotect(ptr as *mut _, len, libc::PROT_READ | libc::PROT_EXEC) != 0 {
                libc::munmap(ptr as *mut _, len);
                return Err("mprotect imeshindwa".into());
            }
        }
        Ok(ExecMem { ptr, len })
    }

    #[cfg(not(unix))]
    pub fn new(_code: &[u8]) -> Result<Self, String> {
        Err("msimbo asilia wa ndani bado haupatikani kwenye mfumo huu".into())
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
        #[cfg(unix)]
        // SAFETY: unmapping exactly the region `new` mapped.
        unsafe {
            libc::munmap(self.ptr as *mut _, self.len);
        }
    }
}
