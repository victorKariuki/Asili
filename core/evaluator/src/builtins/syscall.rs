//! Syscall: raw system call wrapper. Stub returns 0; full impl would use libc or asm.

use std::collections::HashMap;

use crate::value::{self, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("syscall".to_string(), Box::new(|args: &[Value]| {
        let _nr = value::as_f64(args.get(0).unwrap_or(&Value::Hamna)).unwrap_or(0.0);
        let _a = value::as_u64(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or(0);
        let _b = value::as_u64(args.get(2).unwrap_or(&Value::Hamna)).unwrap_or(0);
        let _c = value::as_u64(args.get(3).unwrap_or(&Value::Hamna)).unwrap_or(0);
        let _ = (_nr, _a, _b, _c);
        Ok(Value::Anuani(0))
    }));
}
