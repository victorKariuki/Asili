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
    // Bytes: the same three as text, without UTF-8 in between.
    m.insert(
        "soma_baiti".to_string(),
        Box::new(|args: &[Value]| {
            let path = super::arg_str(args, 0);
            #[cfg(not(target_arch = "wasm32"))]
            {
                Ok(match fs::read(&path) {
                    Ok(b) => Value::sawa(Value::Baiti(b.into())),
                    Err(e) => Value::kosa(e.to_string()),
                })
            }
            #[cfg(target_arch = "wasm32")]
            Ok(Value::kosa("soma_baiti: haipatikani kwenye kivinjari"))
        }),
    );
    for (name, append) in [("andika_baiti", false), ("ongeza_baiti", true)] {
        m.insert(
            name.to_string(),
            Box::new(move |args: &[Value]| {
                let path = super::arg_str(args, 0);
                let data = args
                    .get(1)
                    .and_then(crate::eval::methods::bytes_of)
                    .unwrap_or_default();
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let written = fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .append(append)
                        .truncate(!append)
                        .open(&path)
                        .and_then(|mut f| std::io::Write::write_all(&mut f, &data));
                    Ok(match written {
                        Ok(()) => Value::sawa(Value::Tupu),
                        Err(e) => Value::kosa(e.to_string()),
                    })
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = (append, data);
                    Ok(Value::kosa(format!("{name}: haipatikani kwenye kivinjari")))
                }
            }),
        );
    }
    // Handle-based Faili: faili_fungua(njia, hali) — hali is "soma" | "andika" | "ongeza".
    // Returns Tokeo<Faili, Neno>. The returned handle closes automatically on drop (scope exit,
    // explicit tupa, or .funga()) via FailiHandle's own Drop impl.
    // Directories.
    m.insert(
        "orodha_saraka".to_string(),
        Box::new(|args: &[Value]| {
            let path = super::arg_str(args, 0);
            #[cfg(not(target_arch = "wasm32"))]
            {
                let read = fs::read_dir(&path).and_then(|entries| {
                    let mut names = Vec::new();
                    for entry in entries {
                        names.push(entry?.file_name().to_string_lossy().into_owned());
                    }
                    Ok(names)
                });
                Ok(match read {
                    Ok(mut names) => {
                        // A stable order whatever the filesystem returns.
                        names.sort_unstable();
                        Value::sawa(Value::list(names.into_iter().map(Value::neno).collect()))
                    }
                    Err(e) => Value::kosa(format!("{path}: {e}")),
                })
            }
            #[cfg(target_arch = "wasm32")]
            Ok(Value::kosa("orodha_saraka: haipatikani kwenye kivinjari"))
        }),
    );
    io_unit(m, "unda_saraka", |a| fs::create_dir_all(&a[0]));
    io_unit(m, "futa_saraka", |a| fs::remove_dir_all(&a[0]));
    io_unit(m, "badili_jina", |a| fs::rename(&a[0], &a[1]));
    m.insert(
        "nakili".to_string(),
        Box::new(|args: &[Value]| {
            let (from, to) = (super::arg_str(args, 0), super::arg_str(args, 1));
            #[cfg(not(target_arch = "wasm32"))]
            {
                Ok(match fs::copy(&from, &to) {
                    Ok(bytes) => Value::sawa(Value::Namba(bytes as f64)),
                    Err(e) => Value::kosa(format!("{from} → {to}: {e}")),
                })
            }
            #[cfg(target_arch = "wasm32")]
            Ok(Value::kosa("nakili: haipatikani kwenye kivinjari"))
        }),
    );
    m.insert(
        "ni_saraka".to_string(),
        Box::new(|args: &[Value]| {
            #[cfg(not(target_arch = "wasm32"))]
            let yes = Path::new(&super::arg_str(args, 0)).is_dir();
            #[cfg(target_arch = "wasm32")]
            let yes = {
                let _ = args;
                false
            };
            Ok(Value::Ukweli(yes))
        }),
    );
    m.insert(
        "ni_faili".to_string(),
        Box::new(|args: &[Value]| {
            #[cfg(not(target_arch = "wasm32"))]
            let yes = Path::new(&super::arg_str(args, 0)).is_file();
            #[cfg(target_arch = "wasm32")]
            let yes = {
                let _ = args;
                false
            };
            Ok(Value::Ukweli(yes))
        }),
    );
    // Paths: text only, nothing touches the filesystem except `njia_kamili`.
    m.insert(
        "njia_unganisha".to_string(),
        Box::new(|args: &[Value]| {
            let mut path = std::path::PathBuf::from(super::arg_str(args, 0));
            path.push(super::arg_str(args, 1));
            Ok(Value::neno(path.to_string_lossy().into_owned()))
        }),
    );
    path_part(m, "njia_mzazi", |p| {
        p.parent()
            .map(|q| q.to_string_lossy().into_owned())
            .filter(|q| !q.is_empty())
    });
    path_part(m, "njia_jina", |p| {
        p.file_name().map(|q| q.to_string_lossy().into_owned())
    });
    path_part(m, "njia_kiendelezi", |p| {
        p.extension().map(|q| q.to_string_lossy().into_owned())
    });
    m.insert(
        "njia_kamili".to_string(),
        Box::new(|args: &[Value]| {
            let path = super::arg_str(args, 0);
            #[cfg(not(target_arch = "wasm32"))]
            {
                Ok(match fs::canonicalize(&path) {
                    Ok(full) => Value::sawa(Value::neno(full.to_string_lossy().into_owned())),
                    Err(e) => Value::kosa(format!("{path}: {e}")),
                })
            }
            #[cfg(target_arch = "wasm32")]
            Ok(Value::kosa("njia_kamili: haipatikani kwenye kivinjari"))
        }),
    );
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

/// A filesystem operation on text arguments giving `Tokeo<Tupu, Neno>`.
fn io_unit(
    m: &mut HashMap<String, BuiltinFn>,
    name: &'static str,
    op: fn(&[String]) -> std::io::Result<()>,
) {
    m.insert(
        name.to_string(),
        Box::new(move |args: &[Value]| {
            let texts: Vec<String> = (0..args.len()).map(|i| super::arg_str(args, i)).collect();
            #[cfg(not(target_arch = "wasm32"))]
            {
                if texts.is_empty() {
                    return Ok(Value::kosa(format!("{name}: njia haipo")));
                }
                Ok(match op(&texts) {
                    Ok(()) => Value::sawa(Value::Tupu),
                    Err(e) => Value::kosa(format!("{}: {e}", texts[0])),
                })
            }
            #[cfg(target_arch = "wasm32")]
            {
                let _ = (texts, op);
                Ok(Value::kosa(format!("{name}: haipatikani kwenye kivinjari")))
            }
        }),
    );
}

/// A part of a path as `Chaguo<Neno>` (`Hamna` when the path has none).
fn path_part(
    m: &mut HashMap<String, BuiltinFn>,
    name: &'static str,
    part: fn(&Path) -> Option<String>,
) {
    m.insert(
        name.to_string(),
        Box::new(move |args: &[Value]| {
            let path = super::arg_str(args, 0);
            Ok(Value::Chaguo(
                part(Path::new(&path)).map(|p| Box::new(Value::neno(p))),
            ))
        }),
    );
}
