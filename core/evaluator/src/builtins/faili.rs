//! Faili (file system): soma_faili, andika_faili, ongeza, vipo, futa, ukubwa.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::value::{self, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("soma_faili".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            match fs::read_to_string(&path) {
                Ok(s) => Ok(Value::Tokeo(Ok(Box::new(Value::Neno(s))))),
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "soma_faili: haipatikani kwenye kivinjari".into(),
        )))))
    }));
    m.insert("andika_faili".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        let data = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            match fs::write(&path, &data) {
                Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "andika_faili: haipatikani kwenye kivinjari".into(),
        )))))
    }));
    m.insert("ongeza".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        let data = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            match fs::OpenOptions::new().create(true).append(true).open(&path) {
                Ok(mut f) => match std::io::Write::write_all(&mut f, data.as_bytes()) {
                    Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                    Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
                },
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "ongeza: haipatikani kwenye kivinjari".into(),
        )))))
    }));
    m.insert("vipo".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        let exists = Path::new(&path).exists();
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        let exists = false;
        Ok(Value::Ukweli(exists))
    }));
    m.insert("futa".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            match fs::remove_file(&path) {
                Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "futa: haipatikani kwenye kivinjari".into(),
        )))))
    }));
    m.insert("ukubwa".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        let size = fs::metadata(&path)
            .map(|m| m.len() as f64)
            .unwrap_or(0.0);
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        let size = 0.0;
        Ok(Value::Namba(size))
    }));
}
