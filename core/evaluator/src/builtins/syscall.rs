//! Syscall: raw system call wrapper. Stub returns 0; full impl would use libc or asm.

// TODO(Phase IV): Implement raw syscall dispatch using libc::syscall(nr, a, b, c) on Linux/macOS.
// The syscall number `nr` maps to platform-specific constants (e.g. SYS_read, SYS_write).
// Return value is the OS return code as Anuani (u64). Negative values signal errno.
// Must be gated on `#[cfg(unix)]` and disabled (returns Err) on WASM and Windows.

use std::collections::HashMap;

use crate::value::{self, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    // FIXME(Phase IV): always returns Anuani(0) — no actual syscall is issued.
    m.insert("syscall".to_string(), Box::new(|args: &[Value]| {
        let _nr = value::as_f64(args.get(0).unwrap_or(&Value::Hamna)).unwrap_or(0.0);
        let _a = value::as_u64(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or(0);
        let _b = value::as_u64(args.get(2).unwrap_or(&Value::Hamna)).unwrap_or(0);
        let _c = value::as_u64(args.get(3).unwrap_or(&Value::Hamna)).unwrap_or(0);
        let _ = (_nr, _a, _b, _c);
        Ok(Value::Anuani(0))
    }));
}
