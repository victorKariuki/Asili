//! Mfumo (system): vigezo, pata_env, toka, sikiliza_ishara, rejesha_ishara.

use std::collections::HashMap;

use crate::signal;
use crate::value::{self, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("vigezo".to_string(), Box::new(|_args: &[Value]| {
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        let args_vec: Vec<Value> = std::env::args().map(Value::Neno).collect();
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        let args_vec: Vec<Value> = Vec::new();
        Ok(Value::Orodha(args_vec))
    }));
    m.insert("pata_env".to_string(), Box::new(|args: &[Value]| {
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        let val = {
            let name = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
            std::env::var(&name).ok().map(|s| Box::new(Value::Neno(s)))
        };
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        let val = None::<Box<Value>>;
        Ok(Value::Chaguo(val))
    }));
    m.insert("toka".to_string(), Box::new(|args: &[Value]| {
        let code = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0) as i32;
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        std::process::exit(code);
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        {
            let _ = code;
            Ok(Value::Tupu)
        }
    }));
    m.insert("sikiliza_ishara".to_string(), Box::new(|args: &[Value]| {
        #[cfg(unix)]
        {
            let sig_id = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0) as i32;
            let kazi_name = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
            signal::register_handler(sig_id, kazi_name);
            Ok(Value::Tokeo(Ok(Box::new(Value::Tupu))))
        }
        #[cfg(not(unix))]
        {
            Ok(Value::Tokeo(Err(Box::new(Value::Neno("sikiliza_ishara: sifa haipo kwenye jukwaa hili".to_string())))))
        }
    }));
    m.insert("rejesha_ishara".to_string(), Box::new(|args: &[Value]| {
        #[cfg(unix)]
        {
            let sig_id = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0) as i32;
            signal::clear_handler(sig_id);
            Ok(Value::Tokeo(Ok(Box::new(Value::Tupu))))
        }
        #[cfg(not(unix))]
        {
            Ok(Value::Tokeo(Err(Box::new(Value::Neno("rejesha_ishara: sifa haipo kwenye jukwaa hili".to_string())))))
        }
    }));
}
