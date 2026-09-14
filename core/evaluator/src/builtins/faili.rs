//! Faili (file system): soma_faili, andika_faili, ongeza, vipo, futa, ukubwa (path-based
//! one-shot helpers) plus faili_fungua (handle-based, opt-in real Faili resource type — see
//! docs/design/faili-mkondo-design.md).

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use crate::value::{self, FailiHandle, Value};
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
    // Handle-based Faili: faili_fungua(njia, hali) — hali is "soma" | "andika" | "ongeza".
    // Returns Tokeo<Faili, Neno>. The returned handle closes automatically on drop (scope exit,
    // explicit tupa, or .funga()) via FailiHandle's own Drop impl.
    m.insert("faili_fungua".to_string(), Box::new(|args: &[Value]| {
        let path = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        let mode = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
        {
            let opened = match mode.as_str() {
                "soma" => fs::OpenOptions::new().read(true).open(&path),
                "andika" => fs::OpenOptions::new().write(true).create(true).truncate(true).open(&path),
                "ongeza" => fs::OpenOptions::new().append(true).create(true).open(&path),
                other => {
                    return Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!(
                        "faili_fungua: hali isiyojulikana '{other}' (tumia \"soma\", \"andika\", au \"ongeza\")"
                    ))))));
                }
            };
            match opened {
                Ok(f) => Ok(Value::Tokeo(Ok(Box::new(Value::Faili(Rc::new(RefCell::new(FailiHandle(Some(f))))))))),
                Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
            }
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-wasi")))]
        {
            let _ = mode;
            Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                "faili_fungua: haipatikani kwenye kivinjari".into(),
            )))))
        }
    }));
}
