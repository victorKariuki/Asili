//! Faili (file system): soma_faili, andika_faili, ongeza, vipo, futa, ukubwa (path-based
//! one-shot helpers) plus faili_fungua (handle-based, opt-in real Faili resource type — see
//! docs/design/faili-mkondo-design.md).

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use super::BuiltinFn;
use crate::value::{FailiHandle, Value};

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "soma_faili".to_string(),
        Box::new(|args: &[Value]| {
            let path = super::arg_str(args, 0);
            #[cfg(not(target_arch = "wasm32"))]
            {
                match fs::read_to_string(&path) {
                    Ok(s) => Ok(Value::sawa(Value::neno(s))),
                    Err(e) => Ok(Value::kosa(e.to_string())),
                }
            }
            #[cfg(target_arch = "wasm32")]
            Ok(Value::kosa("soma_faili: haipatikani kwenye kivinjari"))
        }),
    );
    m.insert(
        "andika_faili".to_string(),
        Box::new(|args: &[Value]| {
            let path = super::arg_str(args, 0);
            let data = super::arg_str(args, 1);
            #[cfg(not(target_arch = "wasm32"))]
            {
                match fs::write(&path, &data) {
                    Ok(()) => Ok(Value::sawa(Value::Tupu)),
                    Err(e) => Ok(Value::kosa(e.to_string())),
                }
            }
            #[cfg(target_arch = "wasm32")]
            Ok(Value::kosa("andika_faili: haipatikani kwenye kivinjari"))
        }),
    );
    m.insert(
        "ongeza".to_string(),
        Box::new(|args: &[Value]| {
            let path = super::arg_str(args, 0);
            let data = super::arg_str(args, 1);
            #[cfg(not(target_arch = "wasm32"))]
            {
                match fs::OpenOptions::new().create(true).append(true).open(&path) {
                    Ok(mut f) => match std::io::Write::write_all(&mut f, data.as_bytes()) {
                        Ok(()) => Ok(Value::sawa(Value::Tupu)),
                        Err(e) => Ok(Value::kosa(e.to_string())),
                    },
                    Err(e) => Ok(Value::kosa(e.to_string())),
                }
            }
            #[cfg(target_arch = "wasm32")]
            Ok(Value::kosa("ongeza: haipatikani kwenye kivinjari"))
        }),
    );
    m.insert(
        "vipo".to_string(),
        Box::new(|args: &[Value]| {
            let path = super::arg_str(args, 0);
            #[cfg(not(target_arch = "wasm32"))]
            let exists = Path::new(&path).exists();
            #[cfg(target_arch = "wasm32")]
            let exists = false;
            Ok(Value::Ukweli(exists))
        }),
    );
    m.insert(
        "futa".to_string(),
        Box::new(|args: &[Value]| {
            let path = super::arg_str(args, 0);
            #[cfg(not(target_arch = "wasm32"))]
            {
                match fs::remove_file(&path) {
                    Ok(()) => Ok(Value::sawa(Value::Tupu)),
                    Err(e) => Ok(Value::kosa(e.to_string())),
                }
            }
            #[cfg(target_arch = "wasm32")]
            Ok(Value::kosa("futa: haipatikani kwenye kivinjari"))
        }),
    );
    m.insert(
        "ukubwa".to_string(),
        Box::new(|args: &[Value]| {
            let path = super::arg_str(args, 0);
            #[cfg(not(target_arch = "wasm32"))]
            let size = fs::metadata(&path).map(|m| m.len() as f64).unwrap_or(0.0);
            #[cfg(target_arch = "wasm32")]
            let size = 0.0;
            Ok(Value::Namba(size))
        }),
    );
    // Handle-based Faili: faili_fungua(njia, hali) — hali is "soma" | "andika" | "ongeza".
    // Returns Tokeo<Faili, Neno>. The returned handle closes automatically on drop (scope exit,
    // explicit tupa, or .funga()) via FailiHandle's own Drop impl.
    m.insert("faili_fungua".to_string(), Box::new(|args: &[Value]| {
        let path = super::arg_str(args, 0);
        let mode = super::arg_str(args, 1);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let opened = match mode.as_str() {
                "soma" => fs::OpenOptions::new().read(true).open(&path),
                "andika" => fs::OpenOptions::new().write(true).create(true).truncate(true).open(&path),
                "ongeza" => fs::OpenOptions::new().append(true).create(true).open(&path),
                other => {
                    return Ok(Value::kosa(format!(
                        "faili_fungua: hali isiyojulikana '{other}' (tumia \"soma\", \"andika\", au \"ongeza\")"
                    )));
                }
            };
            match opened {
                Ok(f) => Ok(Value::sawa(Value::Faili(Rc::new(RefCell::new(FailiHandle(Some(f))))))),
                Err(e) => Ok(Value::kosa(e.to_string())),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = mode;
            Ok(Value::kosa("faili_fungua: haipatikani kwenye kivinjari"))
        }
    }));
}
