//! Faili (file system): soma_faili, andika_faili, ongeza, vipo, futa, ukubwa.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::value::{self, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("soma_faili".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(not(target_arch = "wasm32"))]
        {
            match fs::read_to_string(&path) {
                Ok(s) => Ok(Value::Tokeo(Ok(Box::new(Value::Neno(s))))),
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(target_arch = "wasm32")]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "soma_faili haijatengenezwa".into(),
        )))))
    }));
    m.insert("andika_faili".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        let data = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(not(target_arch = "wasm32"))]
        {
            match fs::write(&path, &data) {
                Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(target_arch = "wasm32")]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "andika_faili haijatengenezwa".into(),
        )))))
    }));
    m.insert("ongeza".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        let data = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(not(target_arch = "wasm32"))]
        {
            match fs::OpenOptions::new().create(true).append(true).open(&path) {
                Ok(mut f) => match std::io::Write::write_all(&mut f, data.as_bytes()) {
                    Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                    Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
                },
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(target_arch = "wasm32")]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "ongeza haijatengenezwa".into(),
        )))))
    }));
    m.insert("vipo".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(not(target_arch = "wasm32"))]
        let exists = Path::new(&path).exists();
        #[cfg(target_arch = "wasm32")]
        let exists = false;
        Ok(Value::Ukweli(exists))
    }));
    m.insert("futa".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(not(target_arch = "wasm32"))]
        {
            match fs::remove_file(&path) {
                Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(target_arch = "wasm32")]
        Ok(Value::Tokeo(Err(Box::new(Value::Neno(
            "futa haijatengenezwa".into(),
        )))))
    }));
    m.insert("ukubwa".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(not(target_arch = "wasm32"))]
        let size = fs::metadata(&path)
            .map(|m| m.len() as f64)
            .unwrap_or(0.0);
        #[cfg(target_arch = "wasm32")]
        let size = 0.0;
        Ok(Value::Namba(size))
    }));
}
