//! Ahead-of-time native code for ASB bytecode: the backend-neutral pieces.
//!
//! `pata jenga` compiles typed register bytecode to machine code with the in-house backend
//! ([`crate::nguvu`]) and writes it next to the `.asb`; `pata tenda` maps it and runs its
//! functions instead of interpreting them. Native code is only used when it was built from the
//! exact bytecode being run ([`program_hash`]) for the same calling convention
//! ([`ABI_VERSION`]), so a stale build can never execute.

use crate::bytecode::BytecodeProgram;
use crate::native::NativeFn;

/// Version of the calling convention between generated code and its host (`host.rs`).
pub(crate) const ABI_VERSION: u32 = 11;

/// FNV-1a over the serialized program: identifies the exact bytecode native code was built from.
pub fn program_hash(program: &BytecodeProgram) -> u64 {
    let bytes = bincode::serialize(program).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Native code whose functions correspond 1:1 to a program's functions.
pub struct NativeLibrary {
    // Keeps the code mapped for as long as the function pointers are used.
    _owner: Box<dyn std::any::Any + Send + Sync>,
    pub(crate) funcs: Vec<NativeFn>,
}

impl NativeLibrary {
    pub(crate) fn from_parts(
        owner: Box<dyn std::any::Any + Send + Sync>,
        funcs: Vec<NativeFn>,
    ) -> Self {
        NativeLibrary {
            _owner: owner,
            funcs,
        }
    }
}

// Threads started by `tenda` and the server workers share one program and its native code.
const _: fn() = || {
    fn shareable<T: Send + Sync>() {}
    shareable::<NativeLibrary>();
    shareable::<crate::BytecodeProgram>();
};
