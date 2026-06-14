//! Runtime (kitekelezi): toleo, jina_os.
//
// TODO: The spec lists additional runtime introspectives that are not implemented:
//   - `arch()` -> Neno — CPU architecture (x86_64, aarch64, wasm32, ...)
//   - `ni_debug()` -> Ukweli — true when compiled with debug profile
//   - `ni_wasm()` -> Ukweli — true when running in WASM
//   - `mazingira()` -> Kamusi<Neno,Neno> — full environment variable map
//   - `muda_wa_kuanza()` -> Wakati — process start time (for uptime/perf measurement)

use std::collections::HashMap;

use crate::value::Value;
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("toleo".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Neno(
            option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0").into(),
        ))
    }));
    m.insert("jina_os".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Neno(std::env::consts::OS.into()))
    }));
}
