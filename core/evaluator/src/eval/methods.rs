//! Receiver methods that need no interpreter state.
//!
//! Shared by the tree-walking evaluator and the bytecode VM so both execution paths agree on
//! behaviour and error text.

use std::rc::Rc;
use unicode_segmentation::UnicodeSegmentation;

use crate::value::{self, EvalError, MapKey, Value};

/// `Tokeo` error value produced by an out-of-bounds `Orodha` read.
pub(crate) fn out_of_bounds(idx: usize, len: usize) -> Value {
    let msg = format!("fahirisi nje ya mipaka: {} (urefu {})", idx, len);
    let kosa = Value::Struct(
        "KosaMipaka".to_string(),
        vec![("ujumbe".to_string(), Value::Neno(msg))],
    );
    Value::Tokeo(Err(Box::new(kosa)))
}

pub(crate) fn index_value(base: &Value, index: &Value) -> Result<Value, EvalError> {
    match base {
        Value::Kamusi(m) => {
            let key = MapKey::try_from_value(index)?;
            Ok(m.get(&key).cloned().unwrap_or(Value::Hamna))
        }

        Value::Orodha(v) => {
            let idx = value::as_f64(index)
                .ok_or_else(|| EvalError::TypeErr("fahirisi inahitaji Namba".into()))?;
            let idx = (idx as i64).max(0) as usize;
            if idx >= v.len() {
                Ok(out_of_bounds(idx, v.len()))
            } else {
                Ok(Value::Tokeo(Ok(Box::new(v[idx].clone()))))
            }
        }

        _ => Err(EvalError::TypeErr(
            "fahirisi inahitaji Orodha au Kamusi".into(),
        )),
    }
}

/// Whether `method` on `recv` is implemented by [`pure_method`].
pub(crate) fn is_pure_method(recv: &Value, method: &str) -> bool {
    match recv {
        Value::Neno(_) => matches!(
            method,
            "clona"
                | "urefu"
                | "herufi_kwa"
                | "biti_ngapi"
                | "unganisha"
                | "kata"
                | "tafuta"
                | "kwa_herufi_ndogo"
                | "kwa_herufi_kubwa"
                | "tupu"
                | "ina"
                | "hesabu"
                | "rudia"
                | "anza_na"
                | "maliza_na"
                | "gawanya"
                | "badilisha"
        ),
        Value::Orodha(_) => matches!(
            method,
            "clona" | "urefu" | "pata" | "unganisha" | "jiunge" | "kwa_neno" | "vipande"
        ),
        Value::Kamusi(_) => matches!(method, "clona" | "idadi" | "pata" | "funguo" | "vipo"),
        Value::Seti(_) => matches!(method, "ina" | "urefu" | "clona" | "orodha"),
        Value::Chaguo(_) => matches!(method, "angu" | "ni_tupu" | "ni_po" | "hakikisha"),
        Value::Tokeo(_) => matches!(method, "ni_kosa" | "ni_sawa" | "kosa" | "angu"),
        Value::Jozi(..) => matches!(method, "clona" | "kwanza" | "pili"),
        Value::Wakati(_) => method == "sekunde",
        Value::KashaGC(_) => matches!(method, "pata" | "weka" | "idadi" | "shirikisha"),
        Value::KashaGCDhaifu(_) => method == "imarisha",
        Value::Faili(_) => matches!(method, "soma" | "andika" | "funga"),
        Value::Mkondo(_) => matches!(method, "soma" | "andika" | "funga" | "soma_bailisi"),
        Value::Kumbukumbu(_) => method == "pata",
        Value::NjiaTx(_) | Value::NjiaTxBounded(_) => method == "tuma",
        Value::NjiaRx(_) | Value::NjiaRxBounded(_) => method == "pokea",
        Value::Fungo(_) => matches!(method, "funga" | "fungua" | "pata" | "weka"),
        Value::Enum(en, _, _) if en == "Tokeo" => {
            matches!(method, "ni_kosa" | "ni_sawa" | "kosa" | "angu")
        }
        Value::Enum(en, _, _) if en == "Chaguo" => {
            matches!(method, "ni_po" | "ni_tupu" | "angu" | "hakikisha")
        }
        _ => false,
    }
}

/// Evaluate a state-free method. Callers must check [`is_pure_method`] first.
pub(crate) fn pure_method(
    recv: &Value,
    method: &str,
    args_val: &[Value],
) -> Result<Value, EvalError> {
    match (recv, method) {
        (Value::Neno(s), "clona") => Ok(Value::Neno(s.clone())),
        (Value::Neno(s), "urefu") => Ok(Value::Namba(s.graphemes(true).count() as f64)),
        (Value::Neno(s), "herufi_kwa") => {
            // Grapheme-indexed, matching .urefu()'s existing counting convention (not
            // byte or codepoint index) — a tokenizer walking "what's at position N"
            // wants the same units .urefu() reports N in.
            let idx = args_val
                .first()
                .and_then(value::as_f64)
                .map(|n| n as i64)
                .unwrap_or(-1);
            let ch = if idx >= 0 {
                s.graphemes(true)
                    .nth(idx as usize)
                    .and_then(|g| g.chars().next())
            } else {
                None
            };
            Ok(Value::Chaguo(ch.map(|c| Box::new(Value::Herufi(c)))))
        }
        (Value::Neno(s), "biti_ngapi") => Ok(Value::Namba(s.len() as f64)),
        (Value::Neno(s), "unganisha") => {
            let out = match args_val.first() {
                Some(Value::Neno(sep)) => format!("{s}{sep}"),
                _ => s.clone(),
            };
            Ok(Value::Neno(out))
        }
        (Value::Neno(s), "kata") => {
            let start = args_val
                .first()
                .and_then(value::as_f64)
                .map(|n| n as usize)
                .unwrap_or(0);
            let end = args_val
                .get(1)
                .and_then(value::as_f64)
                .map(|n| n as usize)
                .unwrap_or_else(|| s.len());
            let start = start.min(s.len());
            let end = end.min(s.len()).max(start);
            let sub = String::from_utf8_lossy(s.as_bytes()[start..end].into()).into_owned();
            Ok(Value::Neno(sub))
        }
        (Value::Neno(s), "tafuta") => {
            let sub = args_val
                .first()
                .and_then(value::as_string)
                .unwrap_or_default();
            match s.find(&sub) {
                Some(i) => Ok(Value::Chaguo(Some(Box::new(Value::Namba(i as f64))))),
                None => Ok(Value::Chaguo(None)),
            }
        }
        (Value::Neno(s), "kwa_herufi_ndogo") => Ok(Value::Neno(s.to_lowercase())),
        (Value::Neno(s), "kwa_herufi_kubwa") => Ok(Value::Neno(s.to_uppercase())),
        (Value::Neno(s), "tupu") => Ok(Value::Ukweli(s.is_empty())),
        (Value::Neno(s), "ina") => {
            let sub = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("ina inahitaji Neno".into()))?;
            Ok(Value::Ukweli(s.contains(&sub)))
        }
        (Value::Neno(s), "hesabu") => {
            let sub = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("hesabu inahitaji Neno".into()))?;
            if sub.is_empty() {
                return Ok(Value::Namba(0.0));
            }
            Ok(Value::Namba(s.matches(&sub).count() as f64))
        }
        (Value::Neno(s), "rudia") => {
            let count = args_val
                .first()
                .and_then(value::as_f64)
                .filter(|n| n.is_finite() && *n >= 0.0 && n.fract() == 0.0)
                .ok_or_else(|| {
                    EvalError::TypeErr("rudia inahitaji idadi ya Namba kamili isiyo hasi".into())
                })?;
            Ok(Value::Neno(s.repeat(count as usize)))
        }
        (Value::Neno(s), "anza_na") => {
            let prefix = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("anza_na inahitaji Neno".into()))?;
            Ok(Value::Ukweli(s.starts_with(&prefix)))
        }
        (Value::Neno(s), "maliza_na") => {
            let suffix = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("maliza_na inahitaji Neno".into()))?;
            Ok(Value::Ukweli(s.ends_with(&suffix)))
        }
        (Value::Neno(s), "gawanya") => {
            let sep = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("gawanya inahitaji Neno".into()))?;
            let parts: Vec<Value> = s.split(&sep).map(|p| Value::Neno(p.to_string())).collect();
            Ok(Value::Orodha(parts))
        }
        (Value::Neno(s), "badilisha") => {
            let from = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("badilisha inahitaji from na to".into()))?;
            let to = value::as_string(args_val.get(1).unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("badilisha inahitaji to".into()))?;
            Ok(Value::Neno(s.replace(&from, &to)))
        }
        (Value::Orodha(l), "clona") => Ok(Value::Orodha(l.clone())),
        (Value::Orodha(l), "urefu") => Ok(Value::Namba(l.len() as f64)),
        (Value::Orodha(l), "pata") => {
            let idx = args_val
                .first()
                .and_then(value::as_f64)
                .filter(|n| n.is_finite() && *n >= 0.0 && n.fract() == 0.0)
                .map(|n| n as usize);
            Ok(Value::Chaguo(
                idx.and_then(|i| l.get(i).cloned()).map(Box::new),
            ))
        }
        (Value::Orodha(l), "unganisha") => {
            let sep = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("unganisha inahitaji Neno".into()))?;
            let mut parts = Vec::with_capacity(l.len());
            for item in l {
                parts.push(value::as_string(item).ok_or_else(|| {
                    EvalError::TypeErr("unganisha inahitaji Orodha ya Neno".into())
                })?);
            }
            Ok(Value::Neno(parts.join(&sep)))
        }
        (Value::Orodha(l), "jiunge") => {
            let sep = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("jiunge inahitaji Neno".into()))?;
            let parts: Vec<String> = l
                .iter()
                .map(|item| {
                    value::to_display_string(item).ok_or_else(|| {
                        EvalError::TypeErr(
                            "jiunge inahitaji Orodha yenye thamani zinazoweza kuwa Neno".into(),
                        )
                    })
                })
                .collect::<Result<_, _>>()?;
            Ok(Value::Neno(parts.join(&sep)))
        }
        (Value::Orodha(l), "kwa_neno") => {
            let strings: Vec<Value> = l
                .iter()
                .map(|item| {
                    value::to_display_string(item)
                        .map(Value::Neno)
                        .ok_or_else(|| {
                            EvalError::TypeErr(
                                "kwa_neno inahitaji Orodha yenye thamani zinazoweza kuwa Neno"
                                    .into(),
                            )
                        })
                })
                .collect::<Result<_, _>>()?;
            Ok(Value::Orodha(strings))
        }
        (Value::Orodha(l), "vipande") => {
            let size = args_val
                .first()
                .and_then(value::as_f64)
                .filter(|n| n.is_finite() && *n > 0.0 && n.fract() == 0.0)
                .map(|n| n as usize)
                .ok_or_else(|| {
                    EvalError::TypeErr("vipande inahitaji ukubwa chanya wa Namba kamili".into())
                })?;
            Ok(Value::Orodha(
                l.chunks(size)
                    .map(|chunk| Value::Orodha(chunk.to_vec()))
                    .collect(),
            ))
        }
        (Value::Kamusi(m), "clona") => Ok(Value::Kamusi(m.clone())),
        (Value::Kamusi(m), "idadi") => Ok(Value::Namba(m.len() as f64)),
        (Value::Kamusi(m), "pata") => {
            let key_val = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("pata inahitaji ufunguo".into()))?;
            let key = MapKey::try_from_value(key_val)?;
            Ok(Value::Chaguo(m.get(&key).cloned().map(Box::new)))
        }
        (Value::Kamusi(m), "funguo") => Ok(Value::Orodha(m.keys().map(MapKey::to_value).collect())),
        (Value::Kamusi(m), "vipo") => {
            let key_val = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("vipo inahitaji ufunguo".into()))?;
            let key = MapKey::try_from_value(key_val)?;
            Ok(Value::Ukweli(m.contains_key(&key)))
        }
        (Value::Seti(s), "ina") => {
            let v = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("ina inahitaji thamani".into()))?;
            let key = MapKey::try_from_value(v)?;
            Ok(Value::Ukweli(s.contains(&key)))
        }
        (Value::Seti(s), "urefu") => Ok(Value::Namba(s.len() as f64)),
        (Value::Seti(s), "clona") => Ok(Value::Seti(s.clone())),
        (Value::Seti(s), "orodha") => Ok(Value::Orodha(s.iter().map(MapKey::to_value).collect())),
        (Value::Chaguo(opt), "angu") => {
            let mbadala = args_val.first().cloned().unwrap_or(Value::Hamna);
            Ok(match opt {
                Some(v) => (**v).clone(),
                None => mbadala,
            })
        }
        (Value::Chaguo(opt), "ni_tupu") => Ok(Value::Ukweli(opt.is_none())),
        (Value::Chaguo(opt), "ni_po") => Ok(Value::Ukweli(opt.is_some())),
        (Value::Chaguo(opt), "hakikisha") => {
            let msg = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .unwrap_or_else(|| "Chaguo: Hamna".into());
            match opt {
                Some(v) => Ok((**v).clone()),
                None => Err(EvalError::Panic(msg)),
            }
        }
        (Value::Tokeo(res), "ni_kosa") => Ok(Value::Ukweli(res.is_err())),
        (Value::Tokeo(res), "ni_sawa") => Ok(Value::Ukweli(res.is_ok())),
        (Value::Tokeo(res), "kosa") => match res {
            Ok(_) => Err(EvalError::TypeErr("kosa() inahitaji Tokeo(Kosa)".into())),
            Err(e) => Ok((**e).clone()),
        },
        (Value::Tokeo(res), "angu") => {
            let mbadala = args_val.first().cloned().unwrap_or(Value::Hamna);
            Ok(match res {
                Ok(v) => (**v).clone(),
                Err(_) => mbadala,
            })
        }
        (Value::Jozi(a, b), "clona") => Ok(Value::Jozi(a.clone(), b.clone())),
        (Value::Jozi(a, _), "kwanza") => Ok((**a).clone()),
        (Value::Jozi(_, b), "pili") => Ok((**b).clone()),
        (Value::Wakati(secs), "sekunde") => Ok(Value::Namba(*secs)),
        // Handle types: shared cells, files, streams, channels, locks.
        (Value::KashaGC(cell), "pata") => cell.try_borrow().map(|v| v.clone()).map_err(|_| {
            EvalError::Panic("kasha_gc: pata: tayari inatumika (weka ndani ya .weka)".into())
        }),
        (Value::KashaGC(cell), "weka") => {
            let new_val = args_val.first().cloned().unwrap_or(Value::Hamna);
            match cell.try_borrow_mut() {
                Ok(mut slot) => {
                    *slot = new_val;
                    Ok(Value::Tupu)
                }
                Err(_) => Err(EvalError::Panic(
                    "kasha_gc: weka: tayari inatumika (borrow nyingine iko wazi)".into(),
                )),
            }
        }
        // -1: `recv` above is itself a temporary clone of the receiver's Rc (from
        // evaluating the receiver expression), so strong_count includes one reference
        // that isn't a real, independent handle — subtract it to report the count a
        // caller would actually observe (e.g. via other live `weka` bindings).
        (Value::KashaGC(cell), "idadi") => Ok(Value::Namba((Rc::strong_count(cell) - 1) as f64)),
        (Value::KashaGC(cell), "shirikisha") => Ok(Value::KashaGC(Rc::clone(cell))),
        // Upgrade: Hamna if the strong count already hit zero (every KashaGC handle
        // dropped), Kuna(KashaGC) otherwise — a fresh strong handle sharing the same
        // allocation, exactly like .shirikisha() produces from a live KashaGC.
        (Value::KashaGCDhaifu(weak), "imarisha") => Ok(Value::Chaguo(
            weak.upgrade().map(Value::KashaGC).map(Box::new),
        )),
        (Value::Faili(cell), "soma") => {
            use std::io::Read;
            let mut guard = cell.borrow_mut();
            match guard.0.as_mut() {
                Some(f) => {
                    let mut s = String::new();
                    match f.read_to_string(&mut s) {
                        Ok(_) => Ok(Value::Tokeo(Ok(Box::new(Value::Neno(s))))),
                        Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
                    }
                }
                None => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "faili: imefungwa tayari".into(),
                ))))),
            }
        }
        (Value::Faili(cell), "andika") => {
            use std::io::Write;
            let data =
                value::as_string(args_val.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
            let mut guard = cell.borrow_mut();
            match guard.0.as_mut() {
                Some(f) => match f.write_all(data.as_bytes()) {
                    Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                    Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
                },
                None => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "faili: imefungwa tayari".into(),
                ))))),
            }
        }
        (Value::Faili(cell), "funga") => {
            cell.borrow_mut().0.take();
            Ok(Value::Tupu)
        }
        (Value::Mkondo(cell), "soma") => {
            use std::io::Read;
            let mut guard = cell.borrow_mut();
            match guard.0.as_mut() {
                Some(s) => {
                    let mut buf = String::new();
                    match s.read_to_string(&mut buf) {
                        Ok(_) => Ok(Value::Tokeo(Ok(Box::new(Value::Neno(buf))))),
                        Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
                    }
                }
                None => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "mkondo: imefungwa tayari".into(),
                ))))),
            }
        }
        (Value::Mkondo(cell), "andika") => {
            use std::io::Write;
            let data =
                value::as_string(args_val.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
            let mut guard = cell.borrow_mut();
            match guard.0.as_mut() {
                Some(s) => match s.write_all(data.as_bytes()) {
                    Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                    Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
                },
                None => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "mkondo: imefungwa tayari".into(),
                ))))),
            }
        }
        (Value::Mkondo(cell), "funga") => {
            cell.borrow_mut().0.take();
            Ok(Value::Tupu)
        }
        // `.soma()` (read_to_string) reads until EOF — structurally incompatible with
        // HTTP/1.1 keep-alive, which must read exactly one request's bytes and then be
        // able to read a *second* request over the same connection. `.soma_bailisi`
        // (bounded read) is the primitive an HTTP parser loop actually needs: one
        // Read::read() call, not read-to-EOF, returning whatever bytes were actually
        // available (possibly fewer than kikomo, possibly zero on a timeout with nothing
        // sent — surfaced as an empty Neno, not an error, since a zero-byte read isn't
        // itself a failure). Additive alongside `.soma()`, which keeps its existing
        // behavior for every current caller (mkondo_unganisha's tests, examples/
        // mkondo_server/'s one-request-per-connection contract).
        (Value::Mkondo(cell), "soma_bailisi") => {
            use std::io::Read;
            let kikomo = value::as_f64(args_val.first().unwrap_or(&Value::Hamna))
                .unwrap_or(0.0)
                .max(0.0) as usize;
            let mut guard = cell.borrow_mut();
            match guard.0.as_mut() {
                Some(s) => {
                    let mut buf = vec![0u8; kikomo];
                    match s.read(&mut buf) {
                        Ok(n) => {
                            // Lossy UTF-8: Neno is a Rust String throughout this
                            // interpreter (no Value::Bytes variant exists) — a raw
                            // binary body isn't safely round-trippable through this
                            // method today, a known, documented limitation of this
                            // minimal framing pass (see docs/design/http-framing-design.md).
                            let text = String::from_utf8_lossy(&buf[..n]).into_owned();
                            Ok(Value::Tokeo(Ok(Box::new(Value::Neno(text)))))
                        }
                        Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
                    }
                }
                None => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "mkondo: imefungwa tayari".into(),
                ))))),
            }
        }
        // Kumbukumbu<T> is a plain owning Box, not a shared/interior-mutable cell like
        // Kasha_GC<T> — `.pata()` reads a clone of the boxed value; there is no `.weka()`
        // (in-place mutation) since `recv` here is already a clone of the binding, and
        // mutating that clone's Box would not affect the original `weka`-bound value.
        // Reassign the whole Kumbukumbu (`weka k = kumbukumbu_unda(newval)`) instead.
        (Value::Kumbukumbu(v), "pata") => Ok((**v).clone()),
        (Value::NjiaTx(tx), "tuma") => {
            let v = args_val.first().cloned().unwrap_or(Value::Hamna);
            match v.try_into_send() {
                Some(sv) => {
                    let sent = tx.lock().unwrap().send(sv).is_ok();
                    Ok(Value::Tokeo(if sent {
                        Ok(Box::new(Value::Tupu))
                    } else {
                        Err(Box::new(Value::Neno(
                            "njia: upande wa pili umefungwa".into(),
                        )))
                    }))
                }
                None => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "tuma: thamani haiwezi kuvuka nyuzi (Kasha_GC/Faili/Mkondo)".into(),
                ))))),
            }
        }
        (Value::NjiaRx(rx), "pokea") => {
            let guard = rx.lock().unwrap();
            match guard.recv() {
                Ok(sv) => Ok(Value::Tokeo(Ok(Box::new(sv.into_value())))),
                Err(_) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "pokea: upande wa kutuma umefungwa".into(),
                ))))),
            }
        }
        // Bounded njia_na_kikomo: same .tuma()/.pokea() contract as the unbounded njia()
        // above, except `.tuma()` blocks the caller once the bound is full instead of
        // growing memory without limit — that backpressure is `SyncSender::send`'s own
        // behavior, transparent to this dispatch code.
        (Value::NjiaTxBounded(tx), "tuma") => {
            let v = args_val.first().cloned().unwrap_or(Value::Hamna);
            match v.try_into_send() {
                Some(sv) => {
                    let sent = tx.lock().unwrap().send(sv).is_ok();
                    Ok(Value::Tokeo(if sent {
                        Ok(Box::new(Value::Tupu))
                    } else {
                        Err(Box::new(Value::Neno(
                            "njia: upande wa pili umefungwa".into(),
                        )))
                    }))
                }
                None => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "tuma: thamani haiwezi kuvuka nyuzi (Kasha_GC/Faili/Mkondo)".into(),
                ))))),
            }
        }
        (Value::NjiaRxBounded(rx), "pokea") => {
            let guard = rx.lock().unwrap();
            match guard.recv() {
                Ok(sv) => Ok(Value::Tokeo(Ok(Box::new(sv.into_value())))),
                Err(_) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(
                    "pokea: upande wa kutuma umefungwa".into(),
                ))))),
            }
        }
        // .funga()/.fungua() are an explicit, best-effort lock/unlock pair for holding
        // the lock across several operations — Asili has no closures to scope a critical
        // section with, so unlike .pata()/.weka() below (self-contained, atomic, and the
        // usual way to use a Fungo), there is no way to statically verify a .fungua()
        // call is paired with a prior .funga() on the same logical "holder." Calling
        // .fungua() without holding the lock is a genuine Asili-level programming error,
        // reported as a panic (not silently ignored, not undefined behavior at the Rust
        // level — see FungoCell's `unlock()` safety contract in core/evaluator/src/
        // value/mod.rs, which this dispatch arm is responsible for upholding).
        (Value::Fungo(cell), "funga") => {
            cell.lock();
            Ok(Value::Tupu)
        }
        (Value::Fungo(cell), "fungua") => {
            if cell.try_lock() {
                // try_lock() just acquired a lock nothing was holding — .fungua() was
                // called without a matching .funga(). Release what we just took (this
                // call's own successful try_lock), then report the misuse.
                unsafe { cell.unlock() };
                Err(EvalError::Panic(
                    "fungua: haikuwa imefungwa (hakuna .funga() iliyotangulia)".into(),
                ))
            } else {
                // Locked by someone — assume it's this call's own prior .funga() (the
                // only sound assumption available without per-holder tracking) and
                // release it.
                unsafe { cell.unlock() };
                Ok(Value::Tupu)
            }
        }
        // .pata()/.weka() are self-contained: lock, act, unlock, all in one call — the
        // safe, usual way to use a Fungo, not requiring .funga()/.fungua() at all.
        (Value::Fungo(cell), "pata") => {
            cell.lock();
            let v = unsafe { cell.read() };
            unsafe { cell.unlock() };
            Ok(v.into_value())
        }
        (Value::Fungo(cell), "weka") => {
            let new_val = args_val.first().cloned().unwrap_or(Value::Hamna);
            let Some(sv) = new_val.try_into_send() else {
                return Err(EvalError::TypeErr(
                    "fungo: weka: thamani haiwezi kuvuka nyuzi (Kasha_GC/Faili/Mkondo)".into(),
                ));
            };
            cell.lock();
            unsafe { cell.write(sv) };
            unsafe { cell.unlock() };
            Ok(Value::Tupu)
        }
        // `Tokeo`/`Chaguo` written as enum values (e.g. `Chaguo::Kuna(x).ni_po()`).
        (Value::Enum(en, vn, data), _) if en == "Tokeo" || en == "Chaguo" => {
            let inner = || data.as_ref().map(|v| (**v).clone()).unwrap_or(Value::Hamna);
            let mbadala = || args_val.first().cloned().unwrap_or(Value::Hamna);
            match (en.as_str(), method) {
                ("Tokeo", "ni_kosa") => Ok(Value::Ukweli(vn == "Kosa")),
                ("Tokeo", "ni_sawa") => Ok(Value::Ukweli(vn == "Sawa")),
                ("Tokeo", "kosa") if vn == "Kosa" => Ok(inner()),
                ("Tokeo", "kosa") => Err(EvalError::TypeErr("kosa() inahitaji Tokeo(Kosa)".into())),
                ("Tokeo", "angu") => Ok(if vn == "Sawa" { inner() } else { mbadala() }),
                ("Chaguo", "ni_po") => Ok(Value::Ukweli(vn == "Kuna")),
                ("Chaguo", "ni_tupu") => Ok(Value::Ukweli(vn == "Hamna")),
                ("Chaguo", "angu") => Ok(if vn == "Kuna" { inner() } else { mbadala() }),
                ("Chaguo", "hakikisha") if vn == "Kuna" => Ok(inner()),
                ("Chaguo", "hakikisha") => Err(EvalError::Panic(
                    value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                        .unwrap_or_else(|| "Chaguo: Hamna".into()),
                )),
                _ => Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
            }
        }
        _ => Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
    }
}

/// Whether `method` mutates `recv` in place (`ongeza`, `ingiza`, `ondoa`, `weka_key`,
/// `badilisha` on the collection types that have them).
pub(crate) fn is_mutating(recv: &Value, method: &str) -> bool {
    match recv {
        Value::Orodha(_) => matches!(method, "ongeza" | "ingiza" | "ondoa" | "badilisha"),
        Value::Kamusi(_) => matches!(method, "ingiza" | "weka_key"),
        Value::Seti(_) => matches!(method, "ongeza" | "ondoa"),
        _ => false,
    }
}

fn index_arg(args_val: &[Value]) -> Option<usize> {
    args_val.first().and_then(value::as_f64).map(|n| n as usize)
}

/// Apply a mutating method to a receiver in place (a named local). Callers check
/// [`is_mutating`] first.
pub(crate) fn mutate(
    recv: &mut Value,
    method: &str,
    args_val: &[Value],
) -> Result<Value, EvalError> {
    let arg = |i: usize| args_val.get(i).cloned().unwrap_or(Value::Hamna);
    match (recv, method) {
        (Value::Orodha(values), "ongeza") => {
            values.push(arg(0));
            Ok(Value::Tupu)
        }
        (Value::Orodha(values), "ingiza") => {
            let idx = index_arg(args_val)
                .ok_or_else(|| EvalError::TypeErr("ingiza inahitaji index na thamani".into()))?;
            match values.get_mut(idx) {
                Some(slot) => {
                    *slot = arg(1);
                    Ok(Value::Tupu)
                }
                None => Err(EvalError::TypeErr("ingiza: index nje ya mipaka".into())),
            }
        }
        (Value::Orodha(values), "ondoa") => {
            let idx = index_arg(args_val).unwrap_or(0);
            Ok(if idx < values.len() {
                Value::Chaguo(Some(Box::new(values.remove(idx))))
            } else {
                Value::Chaguo(None)
            })
        }
        (Value::Orodha(values), "badilisha") => {
            let idx = args_val
                .first()
                .and_then(value::as_f64)
                .filter(|n| n.is_finite() && *n >= 0.0 && n.fract() == 0.0)
                .map(|n| n as usize)
                .ok_or_else(|| EvalError::TypeErr("badilisha inahitaji fahirisi".into()))?;
            match values.get_mut(idx) {
                Some(slot) => {
                    *slot = arg(1);
                    Ok(Value::Tupu)
                }
                None => Err(EvalError::TypeErr(
                    "badilisha: fahirisi nje ya mipaka".into(),
                )),
            }
        }
        (Value::Kamusi(map), "ingiza" | "weka_key") => {
            let key_val = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("ingiza inahitaji ufunguo na thamani".into()))?;
            map.insert(MapKey::try_from_value(key_val)?, arg(1));
            Ok(Value::Tupu)
        }
        (Value::Seti(set), "ongeza") => {
            let v = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("ongeza inahitaji thamani".into()))?;
            set.insert(MapKey::try_from_value(v)?);
            Ok(Value::Tupu)
        }
        (Value::Seti(set), "ondoa") => {
            let v = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("ondoa inahitaji thamani".into()))?;
            Ok(Value::Ukweli(set.remove(&MapKey::try_from_value(v)?)))
        }
        _ => Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
    }
}

/// A mutating method on a temporary receiver (e.g. `f().ongeza(1)`): the change is not
/// observable, except that `Orodha.badilisha` returns the updated list.
pub(crate) fn mutate_temporary(
    mut recv: Value,
    method: &str,
    args_val: &[Value],
) -> Result<Value, EvalError> {
    let result = mutate(&mut recv, method, args_val)?;
    Ok(match (&recv, method) {
        (Value::Orodha(_), "badilisha") => recv,
        _ => result,
    })
}

/// How an engine calls a builtin or `kazi` by name for callback methods.
pub(crate) type CallByName<'a> = dyn FnMut(&str, &[Value]) -> Result<Value, EvalError> + 'a;

/// `Orodha` methods that take the name of a `kazi` (or builtin) to call per element.
pub(crate) fn is_callback_method(recv: &Value, method: &str) -> bool {
    matches!(recv, Value::Orodha(_))
        && matches!(
            method,
            "ramani" | "chuja" | "hesabu" | "chunguza" | "kila_na_fahirisi" | "kila_mmoja"
        )
}

/// Run a callback method; `call(name, args)` invokes a builtin or `kazi` by name the way the
/// executing engine does. Callers check [`is_callback_method`] first.
pub(crate) fn callback_method(
    recv: &Value,
    method: &str,
    args_val: &[Value],
    call: &mut CallByName<'_>,
) -> Result<Value, EvalError> {
    let Value::Orodha(items) = recv else {
        return Err(EvalError::Unknown(format!("njia '{method}' haijulikani")));
    };
    let cb_name = args_val.first().and_then(value::as_string);
    if method == "kila_mmoja" {
        // No callback: a no-op.
        if let Some(name) = cb_name {
            for item in items {
                call(&name, std::slice::from_ref(item))?;
            }
        }
        return Ok(Value::Tupu);
    }
    let name = cb_name.ok_or_else(|| EvalError::TypeErr("njia inahitaji jina la kazi".into()))?;
    let mut test = |item: &Value| -> Result<bool, EvalError> {
        Ok(matches!(
            call(&name, std::slice::from_ref(item))?,
            Value::Ukweli(true)
        ))
    };
    match method {
        "ramani" => {
            let mut mapped = Vec::with_capacity(items.len());
            for item in items {
                mapped.push(call(&name, std::slice::from_ref(item))?);
            }
            Ok(Value::Orodha(mapped))
        }
        "chuja" => {
            let mut kept = Vec::new();
            for item in items {
                if test(item)? {
                    kept.push(item.clone());
                }
            }
            Ok(Value::Orodha(kept))
        }
        "hesabu" => {
            let mut count = 0.0;
            for item in items {
                if test(item)? {
                    count += 1.0;
                }
            }
            Ok(Value::Namba(count))
        }
        "chunguza" => {
            for item in items {
                if test(item)? {
                    return Ok(Value::Ukweli(true));
                }
            }
            Ok(Value::Ukweli(false))
        }
        // `kila_na_fahirisi`: the callback receives (element, index).
        _ => {
            for (idx, item) in items.iter().enumerate() {
                call(&name, &[item.clone(), Value::Namba(idx as f64)])?;
            }
            Ok(Value::Tupu)
        }
    }
}

/// `expr kama ty` conversion, shared by the evaluator and the bytecode VM.
pub(crate) fn cast_value(v: Value, ty: &str) -> Result<Value, EvalError> {
    let t = ty.replace(' ', "");
    if t == "Namba" && !matches!(v, Value::NambaKuu(_) | Value::NambaSahihi(_)) {
        let n = value::as_f64(&v)
            .or_else(|| match &v {
                Value::Ukweli(b) => Some(if *b { 1.0 } else { 0.0 }),
                _ => None,
            })
            .or_else(|| value::as_string(&v).and_then(|s| s.parse::<f64>().ok()))
            .or_else(|| value::as_char(&v).map(|c| c as u32 as f64))
            .or_else(|| match &v {
                Value::Chaguo(Some(inner)) => value::as_f64(inner),
                _ => None,
            });
        Ok(Value::Namba(n.unwrap_or(0.0)))
    } else if t == "Namba" {
        // Namba_Kuu/Namba_Sahihi -> Namba is fallible (may not fit in f64's precision or
        // range) — Chaguo<Namba>, matching the Biti8-style fallible-cast precedent below.
        use num_traits::ToPrimitive;
        let n = match &v {
            Value::NambaKuu(b) => b.to_f64(),
            Value::NambaSahihi(b) => {
                use std::str::FromStr;
                f64::from_str(&b.to_string()).ok()
            }
            _ => None,
        };
        Ok(match n {
            Some(n) if n.is_finite() => Value::Chaguo(Some(Box::new(Value::Namba(n)))),
            _ => Value::Chaguo(None),
        })
    } else if t == "Namba_Kuu" {
        // Namba -> Namba_Kuu is infallible (widening); truncates toward zero, matching
        // BinaryOp's own Namba-widening-into-Namba_Kuu behavior in big_numeric_binary_op.
        use crate::value::BigInt;
        let big = match &v {
            Value::NambaKuu(b) => b.clone(),
            Value::Namba(n) => BigInt::from(*n as i64),
            _ => value::as_f64(&v)
                .map(|n| BigInt::from(n as i64))
                .unwrap_or_default(),
        };
        Ok(Value::NambaKuu(big))
    } else if t == "Namba_Sahihi" {
        use crate::value::BigDecimal;
        use num_traits::FromPrimitive;
        let dec = match &v {
            Value::NambaSahihi(b) => b.clone(),
            Value::NambaKuu(b) => BigDecimal::from(b.clone()),
            _ => value::as_f64(&v)
                .and_then(BigDecimal::from_f64)
                .unwrap_or_default(),
        };
        Ok(Value::NambaSahihi(dec))
    } else if t == "Neno" {
        Ok(Value::Neno(match &v {
            Value::Namba(n) => value::format_namba(*n),
            Value::Ukweli(true) => "kweli".into(),
            Value::Ukweli(false) => "si_kweli".into(),
            Value::Neno(s) => s.clone(),
            Value::Herufi(c) => c.to_string(),
            Value::Wakati(secs) => secs.to_string(),
            Value::Anuani(a) => a.to_string(),
            Value::NambaKuu(b) => b.to_string(),
            Value::NambaSahihi(b) => b.to_string(),
            _ => format!("{v:?}"),
        }))
    } else if t == "Herufi" {
        let ch = value::as_char(&v)
            .or_else(|| value::as_f64(&v).and_then(|n| std::char::from_u32(n as u32)))
            .or_else(|| value::as_string(&v).and_then(|s| s.chars().next()));
        Ok(Value::Herufi(ch.unwrap_or('\0')))
    } else if t == "Ukweli" {
        Ok(Value::Ukweli(match &v {
            Value::Ukweli(b) => *b,
            Value::Namba(n) => *n != 0.0,
            _ => true,
        }))
    } else if t == "Wakati" {
        let n = value::as_f64(&v).unwrap_or(0.0);
        Ok(Value::Wakati(n))
    } else if t == "Anuani" {
        let n = value::as_f64(&v).unwrap_or(0.0);
        Ok(Value::Anuani(n as u64))
    // TODO(Phase II): Biti8/uBiti8/Biti32 etc. should map to distinct runtime types,
    // not just range-checked f64. Fixed-width integer arithmetic currently overflows silently.
    } else if t.starts_with("Biti") || t.starts_with("uBiti") {
        let n = value::as_f64(&v).unwrap_or(0.0);
        let n_i = n as i64;
        let fits = match t.as_str() {
            "Biti8" => n_i >= i8::MIN as i64 && n_i <= i8::MAX as i64,
            "Biti16" => n_i >= i16::MIN as i64 && n_i <= i16::MAX as i64,
            "Biti32" => n_i >= i32::MIN as i64 && n_i <= i32::MAX as i64,
            "Biti64" => true,
            "uBiti8" => n_i >= 0 && n_i <= u8::MAX as i64,
            "uBiti16" => n_i >= 0 && n_i <= u16::MAX as i64,
            "uBiti32" => n_i >= 0 && n_i <= u32::MAX as i64,
            "uBiti64" => n_i >= 0,
            _ => true,
        };
        if fits {
            Ok(Value::Chaguo(Some(Box::new(Value::Namba(n)))))
        } else {
            Ok(Value::Chaguo(None))
        }
    } else {
        Ok(v)
    }
}

/// The elements `kwa x katika v` visits: an `Orodha`'s items, or a `Kamusi`'s entries as
/// `Jozi(ufunguo, thamani)`. The loop iterates this snapshot, so the body may mutate `v`.
pub(crate) fn iter_items(v: Value) -> Result<Vec<Value>, EvalError> {
    match v {
        Value::Orodha(items) => Ok(items),
        Value::Kamusi(map) => Ok(map
            .into_iter()
            .map(|(k, v)| Value::Jozi(Box::new(k.to_value()), Box::new(v)))
            .collect()),
        _ => Err(EvalError::TypeErr(
            "kwa...katika inashughulikia Orodha na Kamusi tu".to_string(),
        )),
    }
}

/// Whether any receiver type has a shared method called `method` (used by the bytecode VM to
/// dispatch at run time when a receiver's type is not known statically).
pub(crate) fn is_shared_method_name(method: &str) -> bool {
    use std::collections::{HashMap, HashSet};
    let probes = [
        Value::Neno(String::new()),
        Value::Orodha(Vec::new()),
        Value::Kamusi(HashMap::new()),
        Value::Seti(HashSet::new()),
        Value::Chaguo(None),
        Value::Tokeo(Ok(Box::new(Value::Tupu))),
        Value::Jozi(Box::new(Value::Tupu), Box::new(Value::Tupu)),
        Value::Wakati(0.0),
        Value::Kumbukumbu(Box::new(Value::Tupu)),
    ];
    probes.iter().any(|p| {
        is_pure_method(p, method) || is_mutating(p, method) || is_callback_method(p, method)
    }) || matches!(
        method,
        "shirikisha" | "imarisha" | "soma_bailisi" | "tuma" | "pokea" | "funga" | "fungua" | "weka"
    )
}
