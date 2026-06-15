//! Sambamba (concurrency): anza_mwendo, subiri_mwendo. Stub; full impl would need Send runtime.

// TODO(Phase IV): Implement real thread spawning via std::thread or tokio::task.
// anza_mwendo must look up `kazi_name` in a shared module, spawn a thread, and return a unique handle ID.
// subiri_mwendo must join on that handle and return the thread's result as Tokeo<Value, Neno>.
// Requires a global thread-handle registry (Mutex<HashMap<u64, JoinHandle<Value>>>) and Send-safe Values.

use std::collections::HashMap;

use crate::value::{self, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    // FIXME(Phase IV): always returns dummy ID 0 — no thread is actually spawned.
    m.insert("anza_mwendo".to_string(), Box::new(|args: &[Value]| {
        let _kazi = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        let _hoja = args.get(1).cloned();
        let _ = (_kazi, _hoja);
        Ok(Value::Namba(0.0))
    }));
    // FIXME(Phase IV): always returns Ok(Tupu) — no join actually happens.
    m.insert("subiri_mwendo".to_string(), Box::new(|args: &[Value]| {
        let _id = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0);
        let _ = _id;
        Ok(Value::Tokeo(Ok(Box::new(Value::Tupu))))
    }));
}
