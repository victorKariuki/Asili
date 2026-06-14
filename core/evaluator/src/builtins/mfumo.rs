//! Mfumo (system): vigezo, pata_env, toka, sikiliza_ishara, rejesha_ishara.

use std::collections::HashMap;

use crate::signal;
use crate::value::{self, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("vigezo".to_string(), Box::new(|_args: &[Value]| {
        #[cfg(not(target_arch = "wasm32"))]
        let args_vec: Vec<Value> = std::env::args().map(Value::Neno).collect();
        #[cfg(target_arch = "wasm32")]
        let args_vec: Vec<Value> = Vec::new();
        Ok(Value::Orodha(args_vec))
    }));
    m.insert("pata_env".to_string(), Box::new(|args: &[Value]| {
        #[cfg(not(target_arch = "wasm32"))]
        let val = {
            let name = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
            std::env::var(&name).ok().map(|s| Box::new(Value::Neno(s)))
        };
        #[cfg(target_arch = "wasm32")]
        let val = None::<Box<Value>>;
        Ok(Value::Chaguo(val))
    }));
    m.insert("toka".to_string(), Box::new(|args: &[Value]| {
        let code = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0) as i32;
        #[cfg(not(target_arch = "wasm32"))]
        std::process::exit(code);
        #[cfg(target_arch = "wasm32")]
        {
            let _ = code;
            Ok(Value::Tupu)
        }
    }));
    m.insert("sikiliza_ishara".to_string(), Box::new(|args: &[Value]| {
        #[cfg(unix)]
        {
            let sig_id = value::as_f64(args.get(0).unwrap_or(&Value::Hamna)).unwrap_or(0.0) as i32;
            let kazi_name = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
            signal::register_handler(sig_id, kazi_name);
            Ok(Value::Tupu)
        }
        #[cfg(not(unix))]
        {
            let mut err_map = HashMap::new();
            err_map.insert(MapKey::Neno("ujumbe".to_string()), Value::Neno("sikiliza_ishara: sifa haipo kwenye jukwaa hili".to_string()));
            Ok(Value::Tokeo(
                Box::new(Value::Tupu),
                Box::new(Value::Kamusi(err_map))
            ))
        }
    }));
    m.insert("rejesha_ishara".to_string(), Box::new(|args: &[Value]| {
        #[cfg(unix)]
        {
            let sig_id = value::as_f64(args.get(0).unwrap_or(&Value::Hamna)).unwrap_or(0.0) as i32;
            signal::clear_handler(sig_id);
            Ok(Value::Tupu)
        }
        #[cfg(not(unix))]
        {
            let mut err_map = HashMap::new();
            err_map.insert(MapKey::Neno("ujumbe".to_string()), Value::Neno("rejesha_ishara: sifa haipo kwenye jukwaa hili".to_string()));
            Ok(Value::Tokeo(
                Box::new(Value::Tupu),
                Box::new(Value::Kamusi(err_map))
            ))
        }
    }));
}
