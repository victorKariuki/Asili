//! Kiungo (FFI bridge): load library, call symbol. Stub returns error.

// TODO(Phase IV): Implement dynamic library loading via libloading crate.
// saza_kiungo(path) -> Tokeo<Anuani, Neno>: dlopen the .so/.dylib/.dll and store handle in a
//   global registry (Mutex<HashMap<u64, libloading::Library>>). Return handle address as Anuani.
// wito_kiungo(addr, symbol, args) -> Tokeo<Value, Neno>: retrieve library by addr, dlsym the
//   symbol, and call it with the provided args using an unsafe extern "C" fn pointer.
// Requires careful memory safety: pin the library handle, validate symbol lifetimes.

use std::collections::HashMap;

use crate::value::{self, Value};
use super::BuiltinFn;

fn tokeo_err(msg: String) -> Value {
    Value::Tokeo(Err(Box::new(Value::Neno(msg))))
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    // FIXME(Phase IV): always returns Err — no library is loaded.
    m.insert("saza_kiungo".to_string(), Box::new(|args: &[Value]| {
        let _njia = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        Ok(tokeo_err("saza_kiungo haijatengenezwa".into()))
    }));
    // FIXME(Phase IV): always returns Err — no symbol is resolved or called.
    m.insert("wito_kiungo".to_string(), Box::new(|args: &[Value]| {
        let _anuani = value::as_u64(args.get(0).unwrap_or(&Value::Hamna)).unwrap_or(0);
        let _jina = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
        Ok(tokeo_err("wito_kiungo haijatengenezwa".into()))
    }));
}
