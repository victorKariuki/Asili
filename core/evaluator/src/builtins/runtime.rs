//! Runtime (kitekelezi): toleo, jina_os, arch, ni_debug, ni_wasm, mazingira, muda_wa_kuanza.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::value::{Value, MapKey};
use super::BuiltinFn;

static START_TIME: OnceLock<f64> = OnceLock::new();

fn get_start_time() -> f64 {
    *START_TIME.get_or_init(|| {
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        {
            0.0
        }
    })
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("toleo".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Neno(
            option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0").into(),
        ))
    }));
    m.insert("jina_os".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Neno(std::env::consts::OS.into()))
    }));
    m.insert("arch".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Neno(std::env::consts::ARCH.into()))
    }));
    m.insert("ni_debug".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Ukweli(cfg!(debug_assertions)))
    }));
    m.insert("ni_wasm".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Ukweli(cfg!(target_arch = "wasm32")))
    }));
    m.insert("mazingira".to_string(), Box::new(|_args: &[Value]| {
        let mut map = HashMap::new();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            for (key, val) in std::env::vars() {
                map.insert(MapKey::Neno(key), Value::Neno(val));
            }
        }
        Ok(Value::Kamusi(map))
    }));
    m.insert("muda_wa_kuanza".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Wakati(get_start_time()))
    }));
}
