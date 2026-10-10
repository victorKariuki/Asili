//! Receiver methods that need no interpreter state.
//!
//! The one definition of each method's behaviour and error text, called by native code's host.

use std::rc::Rc;
use unicode_segmentation::UnicodeSegmentation;

use crate::numlist::NumList;
use crate::value::{self, EvalError, MapKey, Value};

fn out_of_bounds_message(idx: usize, len: usize) -> String {
    format!("fahirisi nje ya mipaka: {} (urefu {})", idx, len)
}

/// `Tokeo` error value of an out-of-bounds `b[i]?` / `jaribu b[i]`.
pub(crate) fn out_of_bounds(idx: usize, len: usize) -> Value {
    let kosa = Value::Struct(
        "KosaMipaka".into(),
        vec![(
            "ujumbe".into(),
            Value::neno(out_of_bounds_message(idx, len)),
        )]
        .into(),
    );
    Value::Tokeo(Err(Box::new(kosa)))
}

/// Runtime error of an out-of-bounds plain `b[i]`.
pub(crate) fn out_of_bounds_error(idx: usize, len: usize) -> EvalError {
    EvalError::Panic(out_of_bounds_message(idx, len))
}

/// `base[index]`: an `Orodha` element (out of range is an error) or a `Kamusi` value.
pub(crate) fn index_element(base: &Value, index: &Value) -> Result<Value, EvalError> {
    match base {
        Value::Orodha(v) => {
            let idx = list_index(index)?;
            v.get(idx)
                .cloned()
                .ok_or_else(|| out_of_bounds_error(idx, v.len()))
        }
        Value::Baiti(b) => {
            let idx = list_index(index)?;
            b.get(idx)
                .map(|&x| Value::Namba(x as f64))
                .ok_or_else(|| out_of_bounds_error(idx, b.len()))
        }
        _ => index_value(base, index),
    }
}

fn list_index(index: &Value) -> Result<usize, EvalError> {
    let idx = value::as_f64(index)
        .ok_or_else(|| EvalError::TypeErr("fahirisi inahitaji Namba".into()))?;
    Ok((idx as i64).max(0) as usize)
}

/// `base[index]` as a `Tokeo` (the form `b[i]?` and `jaribu b[i]` unwrap): an `Orodha` element
/// or a `KosaMipaka` error, or a `Kamusi` value.
pub(crate) fn index_value(base: &Value, index: &Value) -> Result<Value, EvalError> {
    match base {
        Value::Kamusi(m) => {
            let key = MapKey::try_from_value(index)?;
            Ok(m.get(&key).cloned().unwrap_or(Value::Hamna))
        }

        Value::Orodha(v) => {
            let idx = list_index(index)?;
            if idx >= v.len() {
                Ok(out_of_bounds(idx, v.len()))
            } else {
                Ok(Value::sawa(v[idx].clone()))
            }
        }

        Value::Baiti(b) => {
            let idx = list_index(index)?;
            Ok(match b.get(idx) {
                Some(&x) => Value::sawa(Value::Namba(x as f64)),
                None => out_of_bounds(idx, b.len()),
            })
        }

        _ => Err(EvalError::TypeErr(
            "fahirisi inahitaji Orodha, Baiti au Kamusi".into(),
        )),
    }
}

/// User-perceived characters in `s`, as `urefu` counts them. In ASCII every character is one
/// except CR LF, which is a single grapheme.
pub(crate) fn grapheme_count(s: &str) -> usize {
    let b = s.as_bytes();
    // The common case — ASCII without '\r', where every byte is one grapheme — checked a word
    // at a time.
    if plain_ascii(b) {
        return b.len();
    }
    if s.is_ascii() {
        b.len() - b.windows(2).filter(|w| w == b"\r\n").count()
    } else {
        s.graphemes(true).count()
    }
}

/// Whether `b` is ASCII without `'\r'`: eight bytes per step (a byte is flagged if its top bit is
/// set or it equals `'\r'`, by the classic zero-byte test on `word ^ 0x0d0d…`).
#[inline]
fn plain_ascii(b: &[u8]) -> bool {
    const ONES: u64 = 0x0101_0101_0101_0101;
    const HIGH: u64 = 0x8080_8080_8080_8080;
    const CR: u64 = 0x0d0d_0d0d_0d0d_0d0d;
    let flagged = |w: u64| {
        let cr = w ^ CR;
        (w | (cr.wrapping_sub(ONES) & !cr)) & HIGH != 0
    };
    let mut chunks = b.chunks_exact(8);
    for c in &mut chunks {
        // `chunks_exact(8)`: always eight bytes.
        let Ok(word) = <[u8; 8]>::try_from(c) else {
            return false;
        };
        if flagged(u64::from_le_bytes(word)) {
            return false;
        }
    }
    let rest = chunks.remainder();
    let mut tail = [0u8; 8];
    tail[..rest.len()].copy_from_slice(rest);
    !flagged(u64::from_le_bytes(tail))
}
/// [`field_ref`], trying position `slot` first (where the field is declared).
#[inline]
pub(crate) fn field_at<'v>(
    recv: &'v Value,
    field: &str,
    slot: u32,
) -> Result<&'v Value, EvalError> {
    if let Value::Struct(_, flds) = recv {
        if let Some((n, v)) = flds.get(slot as usize) {
            if **n == *field {
                return Ok(v);
            }
        }
    }
    field_ref(recv, field)
}

/// [`field_of`] without the copy.
pub(crate) fn field_ref<'v>(recv: &'v Value, field: &str) -> Result<&'v Value, EvalError> {
    match recv {
        Value::Struct(_, flds) => flds
            .iter()
            .find(|(n, _)| &**n == field)
            .map(|(_, v)| v)
            .ok_or_else(|| EvalError::TypeErr(format!("uga haijulikani: {}", field))),
        _ => Err(EvalError::TypeErr(
            "uga unahitaji kitu cha aina ya umbo".into(),
        )),
    }
}

/// Whether `method` on `recv` is implemented by [`pure_method`].
pub(crate) fn is_pure_method(recv: &Value, method: &str) -> bool {
    receiver_kind(recv).is_some_and(|kind| PURE_METHODS[kind as usize].contains(&method))
}

use asili_parser::builtins::{
    MethodReceiver as Kind, CALLBACK_METHODS, MUTATING_METHODS, PURE_METHODS,
};
use asili_parser::Name;

/// [`PURE_METHODS`] and [`MUTATING_METHODS`] as interned names, built once: bytecode names its
/// methods with `Name`s, so classifying a call there compares pointers, not strings.
struct MethodNames {
    pure: Vec<Vec<Name>>,
    mutating: Vec<Vec<Name>>,
}

fn method_names() -> &'static MethodNames {
    static NAMES: std::sync::OnceLock<MethodNames> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        let intern = |table: &[&[&str]]| -> Vec<Vec<Name>> {
            table
                .iter()
                .map(|names| names.iter().map(|n| Name::new(n)).collect())
                .collect()
        };
        MethodNames {
            pure: intern(PURE_METHODS),
            mutating: intern(MUTATING_METHODS),
        }
    })
}

/// [`is_pure_method`] for an interned name.
pub(crate) fn is_pure_name(recv: &Value, method: Name) -> bool {
    receiver_kind(recv).is_some_and(|kind| method_names().pure[kind as usize].contains(&method))
}

/// [`is_mutating`] for an interned name.
pub(crate) fn is_mutating_name(recv: &Value, method: Name) -> bool {
    receiver_kind(recv).is_some_and(|kind| method_names().mutating[kind as usize].contains(&method))
}

fn receiver_kind(recv: &Value) -> Option<Kind> {
    Some(match recv {
        Value::Neno(_) => Kind::Neno,
        Value::Baiti(_) => Kind::Baiti,
        Value::Ahadi(_) => Kind::Ahadi,
        Value::Orodha(_) => Kind::Orodha,
        Value::Kamusi(_) => Kind::Kamusi,
        Value::Seti(_) => Kind::Seti,
        Value::Chaguo(_) => Kind::Chaguo,
        Value::Enum(en, _, _) if en == "Chaguo" => Kind::Chaguo,
        Value::Tokeo(_) => Kind::Tokeo,
        Value::Enum(en, _, _) if en == "Tokeo" => Kind::Tokeo,
        Value::Jozi(..) => Kind::Jozi,
        Value::Wakati(_) => Kind::Wakati,
        Value::KashaGC(_) => Kind::KashaGC,
        Value::KashaGCDhaifu(_) => Kind::KashaGCDhaifu,
        Value::Faili(_) => Kind::Faili,
        Value::Mkondo(_) => Kind::Mkondo,
        Value::MkondoSikilizaji(_) => Kind::MkondoSikilizaji,
        Value::MkondoUdp(_) => Kind::MkondoUdp,
        Value::Kumbukumbu(_) => Kind::Kumbukumbu,
        Value::NjiaTx(_) | Value::NjiaTxBounded(_) => Kind::NjiaTx,
        Value::NjiaRx(_) | Value::NjiaRxBounded(_) => Kind::NjiaRx,
        Value::Fungo(_) => Kind::Fungo,
        _ => return None,
    })
}

/// Evaluate a state-free method. Callers must check [`is_pure_method`] first.
pub(crate) fn pure_method(
    recv: &Value,
    method: &str,
    args_val: &[Value],
) -> Result<Value, EvalError> {
    match (recv, method) {
        (Value::Neno(s), "clona") => Ok(Value::Neno(s.clone())),
        (Value::Neno(s), "urefu") => Ok(Value::Namba(grapheme_count(s) as f64)),
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
                _ => s.to_string(),
            };
            Ok(Value::neno(out))
        }
        (Value::Neno(s), "kata") => {
            // Characters `mwanzo..mwisho`, counted as `urefu` counts them.
            Ok(Value::neno(char_slice(
                s,
                arg_index(args_val, 0).unwrap_or(0),
                arg_index(args_val, 1),
            )))
        }
        (Value::Neno(s), "tafuta") => {
            let sub = args_val
                .first()
                .and_then(value::as_string)
                .unwrap_or_default();
            // The character position (as `urefu` and `kata` count), not the byte offset.
            Ok(Value::Chaguo(s.find(&sub).map(|at| {
                Box::new(Value::Namba(grapheme_count(&s[..at]) as f64))
            })))
        }
        (Value::Neno(s), "safisha") => Ok(trimmed(s, s.trim())),
        (Value::Neno(s), "safisha_mwanzo") => Ok(trimmed(s, s.trim_start())),
        (Value::Neno(s), "safisha_mwisho") => Ok(trimmed(s, s.trim_end())),
        (Value::Neno(s), "jaza_kushoto" | "jaza_kulia") => {
            let width = arg_index(args_val, 0).ok_or_else(|| {
                EvalError::TypeErr(format!("{method} inahitaji urefu wa Namba kamili"))
            })?;
            let fill = match args_val.get(1) {
                None => " ".to_string(),
                Some(v) => value::as_string(v)
                    .or_else(|| value::as_char(v).map(String::from))
                    .filter(|f| grapheme_count(f) == 1)
                    .ok_or_else(|| {
                        EvalError::TypeErr(format!("{method} inahitaji herufi moja ya kujaza"))
                    })?,
            };
            let missing = width.saturating_sub(grapheme_count(s));
            if missing == 0 {
                return Ok(Value::Neno(s.clone()));
            }
            let mut out = String::with_capacity(s.len() + missing * fill.len());
            if method == "jaza_kulia" {
                out.push_str(s);
            }
            for _ in 0..missing {
                out.push_str(&fill);
            }
            if method == "jaza_kushoto" {
                out.push_str(s);
            }
            Ok(Value::neno(out))
        }
        (Value::Neno(s), "jaza") => match args_val.first() {
            Some(Value::Orodha(items)) => fill_template(s, items).map(Value::neno),
            _ => Err(EvalError::TypeErr(
                "jaza inahitaji Orodha ya thamani".into(),
            )),
        },
        (Value::Neno(s), "herufi") => Ok(Value::list(if plain_ascii(s.as_bytes()) {
            s.bytes()
                .map(|b| Value::neno((b as char).to_string()))
                .collect()
        } else {
            s.graphemes(true)
                .map(|g| Value::neno(g.to_string()))
                .collect()
        })),
        (Value::Neno(s), "mistari") => Ok(Value::list(
            s.lines().map(|l| Value::neno(l.to_string())).collect(),
        )),
        (Value::Neno(s), "geuza") => Ok(Value::neno(if plain_ascii(s.as_bytes()) {
            s.chars().rev().collect::<String>()
        } else {
            s.graphemes(true).rev().collect::<String>()
        })),
        (Value::Neno(s), "misimbo") => Ok(Value::list(
            s.chars().map(|c| Value::Namba(c as u32 as f64)).collect(),
        )),
        (Value::Neno(s), "baiti") => Ok(Value::Baiti(s.as_bytes().into())),
        (Value::Baiti(b), _) => bytes_method(b, method, args_val),
        (Value::Ahadi(t), "imekwisha") => Ok(Value::Ukweli(t.is_done())),
        (Value::Ahadi(t), "ghairi") => {
            t.cancel();
            Ok(Value::Tupu)
        }
        (Value::Neno(s), "kwa_namba") => Ok(match s.trim().parse::<f64>() {
            Ok(n) => Value::sawa(Value::Namba(n)),
            Err(_) => Value::kosa(format!("'{s}' si namba")),
        }),
        (Value::Neno(s), "kwa_herufi_ndogo") => Ok(Value::neno(s.to_lowercase())),
        (Value::Neno(s), "kwa_herufi_kubwa") => Ok(Value::neno(s.to_uppercase())),
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
            Ok(Value::neno(s.repeat(count as usize)))
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
            let parts: Vec<Value> = s.split(&sep).map(|p| Value::neno(p.to_string())).collect();
            Ok(Value::list(parts))
        }
        (Value::Neno(s), "badilisha") => {
            let from = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("badilisha inahitaji from na to".into()))?;
            let to = value::as_string(args_val.get(1).unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("badilisha inahitaji to".into()))?;
            Ok(Value::neno(s.replace(&from, &to)))
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
        (Value::Orodha(l), m) if is_list_method(m) => list_method(l, m, args_val),
        (Value::Orodha(l), "unganisha") => {
            let sep = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                .ok_or_else(|| EvalError::TypeErr("unganisha inahitaji Neno".into()))?;
            let mut parts = Vec::with_capacity(l.len());
            for item in l.iter() {
                parts.push(value::as_string(item).ok_or_else(|| {
                    EvalError::TypeErr("unganisha inahitaji Orodha ya Neno".into())
                })?);
            }
            Ok(Value::neno(parts.join(&sep)))
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
            Ok(Value::neno(parts.join(&sep)))
        }
        (Value::Orodha(l), "kwa_neno") => {
            let strings: Vec<Value> = l
                .iter()
                .map(|item| {
                    value::to_display_string(item)
                        .map(Value::neno)
                        .ok_or_else(|| {
                            EvalError::TypeErr(
                                "kwa_neno inahitaji Orodha yenye thamani zinazoweza kuwa Neno"
                                    .into(),
                            )
                        })
                })
                .collect::<Result<_, _>>()?;
            Ok(Value::list(strings))
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
            Ok(Value::list(
                l.chunks(size)
                    .map(|chunk| Value::list(chunk.to_vec()))
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
        (Value::Kamusi(m), "funguo") => Ok(Value::list(m.keys().map(MapKey::to_value).collect())),
        (Value::Kamusi(m), "thamani") => Ok(Value::list(m.values().cloned().collect())),
        (Value::Kamusi(m), "vipengele") => Ok(Value::list(
            m.iter()
                .map(|(k, v)| Value::Jozi(Box::new(k.to_value()), Box::new(v.clone())))
                .collect(),
        )),
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
        (Value::Seti(s), "muungano" | "makutano" | "tofauti" | "ni_sehemu_ya") => {
            let Some(Value::Seti(other)) = args_val.first() else {
                return Err(EvalError::TypeErr(format!("{method} inahitaji Seti")));
            };
            Ok(match method {
                "muungano" => {
                    // Copy the larger set and add the smaller one.
                    let (big, small) = if s.len() >= other.len() {
                        (s, other)
                    } else {
                        (other, s)
                    };
                    let mut out = (**big).clone();
                    out.extend(small.iter().cloned());
                    Value::Seti(Rc::new(out))
                }
                "makutano" => {
                    // Walk the smaller set, probe the larger.
                    let (big, small) = if s.len() >= other.len() {
                        (s, other)
                    } else {
                        (other, s)
                    };
                    Value::Seti(Rc::new(
                        small.iter().filter(|k| big.contains(*k)).cloned().collect(),
                    ))
                }
                "tofauti" => Value::Seti(Rc::new(
                    s.iter().filter(|k| !other.contains(*k)).cloned().collect(),
                )),
                _ => Value::Ukweli(s.len() <= other.len() && s.is_subset(other)),
            })
        }
        (Value::Seti(s), "urefu") => Ok(Value::Namba(s.len() as f64)),
        (Value::Seti(s), "clona") => Ok(Value::Seti(s.clone())),
        (Value::Seti(s), "orodha") => Ok(Value::list(s.iter().map(MapKey::to_value).collect())),
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
                        Ok(_) => Ok(Value::sawa(Value::neno(s))),
                        Err(e) => Ok(Value::kosa(e.to_string())),
                    }
                }
                None => Ok(Value::kosa("faili: imefungwa tayari")),
            }
        }
        (Value::Faili(cell), "soma_baiti") => {
            use std::io::Read;
            let mut guard = cell.borrow_mut();
            match guard.0.as_mut() {
                Some(f) => {
                    let mut bytes = Vec::new();
                    match f.read_to_end(&mut bytes) {
                        Ok(_) => Ok(Value::sawa(Value::Baiti(bytes.into()))),
                        Err(e) => Ok(Value::kosa(e.to_string())),
                    }
                }
                None => Ok(Value::kosa("faili: imefungwa tayari")),
            }
        }
        (Value::Faili(cell), "andika") => {
            use std::io::Write;
            let data = bytes_of(args_val.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
            let mut guard = cell.borrow_mut();
            match guard.0.as_mut() {
                Some(f) => match f.write_all(&data) {
                    Ok(()) => Ok(Value::sawa(Value::Tupu)),
                    Err(e) => Ok(Value::kosa(e.to_string())),
                },
                None => Ok(Value::kosa("faili: imefungwa tayari")),
            }
        }
        (Value::Faili(cell), "funga") => {
            cell.borrow_mut().0.take();
            Ok(Value::Tupu)
        }
        (Value::Mkondo(cell), _) => crate::builtins::mkondo::mkondo_method(cell, method, args_val),
        (Value::MkondoSikilizaji(l), _) => {
            crate::builtins::mkondo::sikilizaji_method(l, method, args_val)
        }
        (Value::MkondoUdp(u), _) => crate::builtins::mkondo::udp_method(u, method, args_val),
        // Kumbukumbu<T> is a plain owning Box, not a shared/interior-mutable cell like
        // Kasha_GC<T> — `.pata()` reads a clone of the boxed value; there is no `.weka()`
        // (in-place mutation) since `recv` here is already a clone of the binding, and
        // mutating that clone's Box would not affect the original `weka`-bound value.
        // Reassign the whole Kumbukumbu (`weka k = kumbukumbu_unda(newval)`) instead.
        (Value::Kumbukumbu(v), "pata") => Ok((**v).clone()),
        // `.tuma()` on a bounded channel waits while it is full; `.pokea()` waits for a value.
        // Both wait through `kazi_sawia`, so other tasks of the thread run meanwhile.
        (Value::NjiaTx(tx) | Value::NjiaTxBounded(tx), "tuma") => {
            let v = args_val.first().cloned().unwrap_or(Value::Hamna);
            let Some(sv) = v.try_into_send() else {
                return Ok(Value::kosa(
                    "tuma: thamani haiwezi kuvuka nyuzi (Kasha_GC/Faili/Mkondo)",
                ));
            };
            let sent = match tx.try_send(sv) {
                Ok(()) => true,
                Err(flume::TrySendError::Disconnected(_)) => false,
                Err(flume::TrySendError::Full(sv)) => {
                    let tx = tx.clone();
                    crate::kazi_sawia::block_on(async move { tx.send_async(sv).await.is_ok() })?
                }
            };
            Ok(if sent {
                Value::sawa(Value::Tupu)
            } else {
                Value::kosa("njia: upande wa pili umefungwa")
            })
        }
        (Value::NjiaRx(rx) | Value::NjiaRxBounded(rx), "pokea") => {
            let received = match rx.try_recv() {
                Ok(sv) => Ok(sv),
                Err(flume::TryRecvError::Disconnected) => Err(()),
                Err(flume::TryRecvError::Empty) => {
                    crate::platform::flush_stdout(); // about to wait: show what was printed
                    let rx = rx.clone();
                    crate::kazi_sawia::block_on(
                        async move { rx.recv_async().await.map_err(|_| ()) },
                    )?
                }
            };
            Ok(match received {
                Ok(sv) => Value::sawa(sv.into_value()),
                Err(()) => Value::kosa("pokea: upande wa kutuma umefungwa"),
            })
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
            lock_fungo(cell)?;
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
            lock_fungo(cell)?;
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
            lock_fungo(cell)?;
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

/// Argument `i` as a non-negative whole number (an index or a length).
fn arg_index(args_val: &[Value], i: usize) -> Option<usize> {
    args_val
        .get(i)
        .and_then(value::as_f64)
        .filter(|n| n.is_finite() && *n >= 0.0)
        .map(|n| n as usize)
}

/// `s` trimmed to `part` (a sub-slice of it), sharing `s` when nothing was trimmed.
fn trimmed(s: &value::Text, part: &str) -> Value {
    if part.len() == s.len() {
        Value::Neno(s.clone())
    } else {
        Value::neno(part.to_string())
    }
}

/// Characters `start..end` of `s` (`end` defaults to the end; both clamped), counted as
/// `urefu` counts them — byte offsets directly when `s` is plain ASCII.
fn char_slice(s: &str, start: usize, end: Option<usize>) -> String {
    if plain_ascii(s.as_bytes()) {
        let start = start.min(s.len());
        let end = end.unwrap_or(s.len()).min(s.len()).max(start);
        return s[start..end].to_string();
    }
    let mut bounds = s.grapheme_indices(true).map(|(i, _)| i).chain([s.len()]);
    let Some(from) = bounds.nth(start) else {
        return String::new();
    };
    let to = match end {
        Some(end) if end > start => bounds.nth(end - start - 1).unwrap_or(s.len()),
        Some(_) => from,
        None => s.len(),
    };
    s[from..to].to_string()
}

/// List methods that look at the elements as values (equality, order, sums): one
/// implementation for numbers ([`numbers_method`], shared with typed numeric lists) and one for
/// everything else.
fn is_list_method(m: &str) -> bool {
    matches!(
        m,
        "tupu"
            | "kwanza"
            | "mwisho"
            | "ina"
            | "tafuta"
            | "kata"
            | "geuza"
            | "panga"
            | "kubwa"
            | "ndogo"
            | "jumla"
            | "kipekee"
    )
}

fn list_method(l: &Rc<Vec<Value>>, method: &str, args_val: &[Value]) -> Result<Value, EvalError> {
    if let Some(nums) = as_numbers(l) {
        if let Some(out) = numbers_method(&nums, method, args_val) {
            return out.map(NumOut::into_value);
        }
    }
    match method {
        "tupu" => Ok(Value::Ukweli(l.is_empty())),
        "kwanza" => Ok(Value::Chaguo(l.first().cloned().map(Box::new))),
        "mwisho" => Ok(Value::Chaguo(l.last().cloned().map(Box::new))),
        "ina" | "tafuta" => {
            let x = args_val.first().unwrap_or(&Value::Hamna);
            let mut found = None;
            for (i, item) in l.iter().enumerate() {
                if equal(item, x)? {
                    found = Some(i);
                    break;
                }
            }
            Ok(if method == "ina" {
                Value::Ukweli(found.is_some())
            } else {
                Value::Chaguo(found.map(|i| Box::new(Value::Namba(i as f64))))
            })
        }
        "kata" => {
            let start = arg_index(args_val, 0).unwrap_or(0).min(l.len());
            let end = arg_index(args_val, 1)
                .unwrap_or(l.len())
                .min(l.len())
                .max(start);
            Ok(Value::list(l[start..end].to_vec()))
        }
        "geuza" => Ok(Value::list(l.iter().rev().cloned().collect())),
        "panga" => {
            check_orderable(l)?;
            let mut items = l.to_vec();
            items.sort_by(order);
            Ok(Value::list(items))
        }
        "kubwa" | "ndogo" => {
            check_orderable(l)?;
            let want = if method == "kubwa" {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            };
            let mut best: Option<&Value> = None;
            for item in l.iter() {
                if best.is_none_or(|b| order(item, b) == want) {
                    best = Some(item);
                }
            }
            Ok(Value::Chaguo(best.cloned().map(Box::new)))
        }
        "jumla" => Err(EvalError::TypeErr("jumla inahitaji Orodha ya Namba".into())),
        "kipekee" => {
            // Repeats as a `Seti` counts them; values a `Seti` cannot hold, by `==`.
            let mut kept: Vec<Value> = Vec::new();
            let mut seen: std::collections::HashSet<MapKey> = Default::default();
            for item in l.iter() {
                let fresh = match MapKey::try_from_value(item) {
                    Ok(key) => seen.insert(key),
                    Err(_) => {
                        let mut new = true;
                        for k in &kept {
                            if equal(k, item)? {
                                new = false;
                                break;
                            }
                        }
                        new
                    }
                };
                if fresh {
                    kept.push(item.clone());
                }
            }
            Ok(Value::list(kept))
        }
        _ => Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
    }
}

/// Which register file a typed numeric list's method result goes to (the bytecode compiler
/// gives `ListMethod` a destination of this kind).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NumOutKind {
    List,
    Num,
    Bool,
    Val,
}

/// Pure methods [`numbers_method`] implements on a typed numeric list, and where each result
/// goes.
pub(crate) const NUMBER_LIST_METHODS: &[(&str, NumOutKind)] = &[
    ("tupu", NumOutKind::Bool),
    ("ina", NumOutKind::Bool),
    ("jumla", NumOutKind::Num),
    ("kwanza", NumOutKind::Val),
    ("mwisho", NumOutKind::Val),
    ("kubwa", NumOutKind::Val),
    ("ndogo", NumOutKind::Val),
    ("tafuta", NumOutKind::Val),
    ("pata", NumOutKind::Val),
    ("kata", NumOutKind::List),
    ("geuza", NumOutKind::List),
    ("panga", NumOutKind::List),
    ("kipekee", NumOutKind::List),
    ("clona", NumOutKind::List),
];

/// Where `method`'s result goes when it runs on a typed numeric list, or `None` when it does
/// not run there (the list is then converted to generic values first).
pub(crate) fn number_list_result(method: &str) -> Option<NumOutKind> {
    NUMBER_LIST_METHODS
        .iter()
        .find(|(m, _)| *m == method)
        .map(|(_, k)| *k)
}

/// A numeric-list method's result.
pub(crate) enum NumOut {
    List(NumList),
    Num(f64),
    Bool(bool),
    Val(Value),
}

impl NumOut {
    pub(crate) fn into_value(self) -> Value {
        match self {
            NumOut::List(l) => Value::list(l.iter().map(Value::Namba).collect()),
            NumOut::Num(n) => Value::Namba(n),
            NumOut::Bool(b) => Value::Ukweli(b),
            NumOut::Val(v) => v,
        }
    }
}

/// The elements of `l` as a typed numeric list, when every one is a number.
fn as_numbers(l: &[Value]) -> Option<NumList> {
    let mut out = Vec::with_capacity(l.len());
    for v in l {
        match v {
            Value::Namba(n) => out.push(*n),
            _ => return None,
        }
    }
    Some(out.into_iter().collect())
}

fn some_number(n: Option<f64>) -> Value {
    Value::Chaguo(n.map(|n| Box::new(Value::Namba(n))))
}

/// `method` on a list of numbers — the one implementation for typed numeric lists
/// (`ListMethod`) and for generic lists that hold only numbers — or `None` when it is not one
/// of [`NUMBER_LIST_METHODS`] or its arguments are not numbers (the generic path then answers,
/// the same way).
pub(crate) fn numbers_method(
    l: &NumList,
    method: &str,
    args_val: &[Value],
) -> Option<Result<NumOut, EvalError>> {
    let number = |i: usize| match args_val.get(i) {
        Some(Value::Namba(n)) => Some(*n),
        _ => None,
    };
    let len = l.len();
    Some(Ok(match method {
        "tupu" => NumOut::Bool(len == 0),
        "kwanza" => NumOut::Val(some_number(l.get(0))),
        "mwisho" => NumOut::Val(some_number(len.checked_sub(1).and_then(|i| l.get(i)))),
        "pata" => {
            let i = args_val
                .first()
                .and_then(value::as_f64)
                .filter(|n| n.is_finite() && *n >= 0.0 && n.fract() == 0.0);
            NumOut::Val(some_number(i.and_then(|i| l.get(i as usize))))
        }
        "ina" => {
            let x = number(0)?;
            NumOut::Bool(l.position(x).is_some())
        }
        "tafuta" => {
            let x = number(0)?;
            NumOut::Val(some_number(l.position(x).map(|i| i as f64)))
        }
        "kata" => {
            let start = arg_index(args_val, 0).unwrap_or(0).min(len);
            let end = arg_index(args_val, 1).unwrap_or(len).min(len).max(start);
            NumOut::List(l.slice(start, end))
        }
        "geuza" => NumOut::List(l.reversed()),
        "panga" => NumOut::List(l.sorted(order_f64)),
        "kubwa" | "ndogo" => NumOut::Val(some_number(l.extreme(method == "kubwa", order_f64))),
        // Left to right, as a loop of `+` would add them.
        "jumla" => NumOut::Num(l.sum()),
        // Repeats as a `Seti` counts them: numbers by their exact value.
        "kipekee" => NumOut::List(l.unique()),
        "clona" => NumOut::List(l.clone()),
        _ => return None,
    }))
}

/// The order `panga` gives numbers: by value, NaN after every other number (a total order, so
/// sorting never depends on the algorithm).
pub(crate) fn order_f64(x: &f64, y: &f64) -> std::cmp::Ordering {
    x.partial_cmp(y)
        .unwrap_or_else(|| x.is_nan().cmp(&y.is_nan()))
}

/// `a == b` with the language's equality (numbers and text compared directly).
fn equal(a: &Value, b: &Value) -> Result<bool, EvalError> {
    match (a, b) {
        (Value::Namba(x), Value::Namba(y)) => Ok(x == y),
        (Value::Neno(x), Value::Neno(y)) => Ok(x == y),
        _ => Ok(matches!(
            super::ops::binary_value(&asili_parser::BinaryOp::Eq, a, b)?,
            Value::Ukweli(true)
        )),
    }
}

/// Values `panga`, `kubwa`/`ndogo` and `panga_kwa` can order: all numbers, all text, all
/// characters or all truth values.
fn check_orderable(items: &[Value]) -> Result<(), EvalError> {
    let kind = |v: &Value| match v {
        Value::Namba(_) => 0,
        Value::Neno(_) => 1,
        Value::Herufi(_) => 2,
        Value::Ukweli(_) => 3,
        _ => 4,
    };
    let first = items.first().map(kind);
    if first != Some(4) && items.iter().all(|v| Some(kind(v)) == first) {
        Ok(())
    } else {
        Err(EvalError::TypeErr(
            "kupanga kunahitaji Namba zote, Neno zote, Herufi zote au Ukweli zote".into(),
        ))
    }
}

/// A total order on values [`check_orderable`] accepts: numbers as [`order_f64`], text by its
/// characters, `si_kweli` before `kweli`.
fn order(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Namba(x), Value::Namba(y)) => order_f64(x, y),
        (Value::Neno(x), Value::Neno(y)) => x.as_ref().cmp(y.as_ref()),
        (Value::Herufi(x), Value::Herufi(y)) => x.cmp(y),
        (Value::Ukweli(x), Value::Ukweli(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}

/// `kiolezo` with each `{}` replaced by the next item's text (`{{` and `}}` are literal
/// braces); the number of `{}` must equal the number of items.
fn fill_template(kiolezo: &str, items: &[Value]) -> Result<String, EvalError> {
    let mut out = String::with_capacity(kiolezo.len() + 8 * items.len());
    let mut next = items.iter();
    let mut rest = kiolezo;
    while let Some(at) = rest.find(['{', '}']) {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        if let Some(after) = tail.strip_prefix("{{").or_else(|| tail.strip_prefix("}}")) {
            out.push_str(&tail[..1]);
            rest = after;
        } else if let Some(after) = tail.strip_prefix("{}") {
            let item = next.next().ok_or_else(|| {
                EvalError::TypeErr("jaza: nafasi {} ni nyingi kuliko thamani".into())
            })?;
            match item {
                Value::Neno(t) => out.push_str(t),
                other => {
                    out.push_str(&value::to_display_string(other).ok_or_else(|| {
                        EvalError::TypeErr("jaza: thamani haiwezi kuwa Neno".into())
                    })?)
                }
            }
            rest = after;
        } else {
            out.push_str(&tail[..1]);
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    if next.next().is_some() {
        return Err(EvalError::TypeErr(
            "jaza: thamani ni nyingi kuliko nafasi {}".into(),
        ));
    }
    Ok(out)
}

/// Whether `method` mutates `recv` in place (`ongeza`, `ingiza`, `ondoa`, `weka_key`,
/// `badilisha` on the collection types that have them).
pub(crate) fn is_mutating(recv: &Value, method: &str) -> bool {
    receiver_kind(recv).is_some_and(|kind| MUTATING_METHODS[kind as usize].contains(&method))
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
            Rc::make_mut(values).push(arg(0));
            Ok(Value::Tupu)
        }
        (Value::Orodha(values), "ingiza") => {
            let idx = index_arg(args_val)
                .ok_or_else(|| EvalError::TypeErr("ingiza inahitaji index na thamani".into()))?;
            match Rc::make_mut(values).get_mut(idx) {
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
                Value::Chaguo(Some(Box::new(Rc::make_mut(values).remove(idx))))
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
            match Rc::make_mut(values).get_mut(idx) {
                Some(slot) => {
                    *slot = arg(1);
                    Ok(Value::Tupu)
                }
                None => Err(EvalError::TypeErr(
                    "badilisha: fahirisi nje ya mipaka".into(),
                )),
            }
        }
        (Value::Orodha(values), "ongeza_zote") => {
            match args_val.first() {
                Some(Value::Orodha(more)) => Rc::make_mut(values).extend(more.iter().cloned()),
                _ => return Err(EvalError::TypeErr("ongeza_zote inahitaji Orodha".into())),
            }
            Ok(Value::Tupu)
        }
        (Value::Orodha(values), "futa_zote") => {
            if Rc::get_mut(values).is_some() {
                Rc::make_mut(values).clear();
            } else {
                *values = Rc::new(Vec::new()); // shared: drop this copy's reference instead
            }
            Ok(Value::Tupu)
        }
        (Value::Kamusi(map), "ondoa") => {
            let key_val = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("ondoa inahitaji ufunguo".into()))?;
            let key = MapKey::try_from_value(key_val)?;
            if !map.contains_key(&key) {
                return Ok(Value::Chaguo(None)); // no copy of a shared map for a miss
            }
            Ok(Value::Chaguo(Rc::make_mut(map).remove(&key).map(Box::new)))
        }
        (Value::Kamusi(map), "futa_zote") => {
            if Rc::get_mut(map).is_some() {
                Rc::make_mut(map).clear();
            } else {
                *map = Rc::new(Default::default());
            }
            Ok(Value::Tupu)
        }
        (Value::Kamusi(map), "ingiza" | "weka_key") => {
            let key_val = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("ingiza inahitaji ufunguo na thamani".into()))?;
            Rc::make_mut(map).insert(MapKey::try_from_value(key_val)?, arg(1));
            Ok(Value::Tupu)
        }
        (Value::Seti(set), "ongeza") => {
            let v = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("ongeza inahitaji thamani".into()))?;
            Rc::make_mut(set).insert(MapKey::try_from_value(v)?);
            Ok(Value::Tupu)
        }
        (Value::Seti(set), "ondoa") => {
            let v = args_val
                .first()
                .ok_or_else(|| EvalError::TypeErr("ondoa inahitaji thamani".into()))?;
            Ok(Value::Ukweli(
                Rc::make_mut(set).remove(&MapKey::try_from_value(v)?),
            ))
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
    receiver_kind(recv).is_some_and(|kind| CALLBACK_METHODS[kind as usize].contains(&method))
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
            for item in items.iter() {
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
            for item in items.iter() {
                mapped.push(call(&name, std::slice::from_ref(item))?);
            }
            Ok(Value::list(mapped))
        }
        "chuja" => {
            let mut kept = Vec::new();
            for item in items.iter() {
                if test(item)? {
                    kept.push(item.clone());
                }
            }
            Ok(Value::list(kept))
        }
        "hesabu" => {
            let mut count = 0.0;
            for item in items.iter() {
                if test(item)? {
                    count += 1.0;
                }
            }
            Ok(Value::Namba(count))
        }
        "panga_kwa" => {
            // Sort by the key the named function gives each element (stable); each key is
            // computed once.
            let mut keyed = Vec::with_capacity(items.len());
            for item in items.iter() {
                keyed.push((call(&name, std::slice::from_ref(item))?, item.clone()));
            }
            let keys: Vec<Value> = keyed.iter().map(|(k, _)| k.clone()).collect();
            check_orderable(&keys)?;
            keyed.sort_by(|a, b| order(&a.0, &b.0));
            Ok(Value::list(keyed.into_iter().map(|(_, v)| v).collect()))
        }
        "chunguza" => {
            for item in items.iter() {
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

/// `expr kama ty` conversion, shared by the evaluator and native code's host.
pub(crate) fn cast_value(v: Value, ty: &str) -> Result<Value, EvalError> {
    // The common `n kama Neno`, before any type-name handling.
    if let (Value::Namba(n), "Neno") = (&v, ty) {
        return Ok(Value::Neno(value::namba_text(*n)));
    }
    let t: std::borrow::Cow<str> = if ty.contains(' ') {
        ty.replace(' ', "").into()
    } else {
        ty.into()
    };
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
            Value::Namba(n) => std::rc::Rc::new(BigInt::from(*n as i64)),
            _ => std::rc::Rc::new(
                value::as_f64(&v)
                    .map(|n| BigInt::from(n as i64))
                    .unwrap_or_default(),
            ),
        };
        Ok(Value::NambaKuu(big))
    } else if t == "Namba_Sahihi" {
        use crate::value::BigDecimal;
        use num_traits::FromPrimitive;
        let dec = match &v {
            Value::NambaSahihi(b) => b.clone(),
            Value::NambaKuu(b) => std::rc::Rc::new(BigDecimal::from((**b).clone())),
            _ => std::rc::Rc::new(
                value::as_f64(&v)
                    .and_then(BigDecimal::from_f64)
                    .unwrap_or_default(),
            ),
        };
        Ok(Value::NambaSahihi(dec))
    } else if t == "Baiti" {
        Ok(match &v {
            Value::Baiti(_) => v,
            Value::Neno(s) => Value::Baiti(s.as_bytes().into()),
            Value::Orodha(_) => match bytes_from_list(&v) {
                Ok(b) => Value::Baiti(b.into()),
                Err(e) => return Err(EvalError::TypeErr(e)),
            },
            _ => {
                return Err(EvalError::TypeErr(
                    "kama Baiti inahitaji Neno, Orodha<Namba> au Baiti".into(),
                ))
            }
        })
    } else if t == "Neno" {
        match v {
            Value::Neno(_) => return Ok(v),
            Value::Namba(n) => return Ok(Value::Neno(value::namba_text(n))),
            // Bytes as UTF-8, invalid sequences replaced (`.kwa_neno()` refuses them instead).
            Value::Baiti(b) => return Ok(Value::neno(String::from_utf8_lossy(&b).into_owned())),
            _ => {}
        }
        Ok(Value::neno(match &v {
            Value::Namba(n) => value::format_namba(*n),
            Value::Ukweli(true) => "kweli".into(),
            Value::Ukweli(false) => "si_kweli".into(),
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
        let fits = match &*t {
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

/// Take `cell`'s lock: blocking the thread, or — while `sawia` tasks are about — waiting so that
/// they run (a task holding the lock may be the one that has to run to release it).
fn lock_fungo(cell: &crate::value::FungoCell) -> Result<(), EvalError> {
    if crate::kazi_sawia::tasks_active() {
        crate::kazi_sawia::wait_until(|| cell.try_lock())
    } else {
        cell.lock();
        Ok(())
    }
}

/// The bytes of `v` when it is `Baiti`, or the UTF-8 bytes of a `Neno` — what an operation on
/// bytes accepts as a needle or as data to write.
pub(crate) fn bytes_of(v: &Value) -> Option<std::borrow::Cow<'_, [u8]>> {
    match v {
        Value::Baiti(b) => Some(std::borrow::Cow::Borrowed(&b[..])),
        Value::Neno(s) => Some(std::borrow::Cow::Borrowed(s.as_bytes())),
        _ => None,
    }
}

/// An `Orodha` of whole numbers 0–255 as bytes, or why not.
pub(crate) fn bytes_from_list(v: &Value) -> Result<Vec<u8>, String> {
    let items = match v {
        Value::Orodha(items) => items,
        _ => return Err("baiti zinahitaji Orodha<Namba>".into()),
    };
    items
        .iter()
        .enumerate()
        .map(|(i, x)| match value::as_f64(x) {
            Some(n) if n.fract() == 0.0 && (0.0..=255.0).contains(&n) => Ok(n as u8),
            _ => Err(format!(
                "baiti: kipengele #{i} si namba kamili kati ya 0 na 255"
            )),
        })
        .collect()
}

/// Where `needle` (bytes, or one byte as a `Namba`) first occurs in `hay`.
fn find_bytes(hay: &[u8], needle: &Value) -> Option<usize> {
    match needle {
        Value::Namba(n) => hay.iter().position(|&b| b as f64 == *n),
        other => {
            let n = bytes_of(other)?;
            if n.is_empty() {
                return Some(0);
            }
            hay.windows(n.len()).position(|w| w == &n[..])
        }
    }
}

/// The methods of a `Baiti`.
fn bytes_method(b: &Rc<[u8]>, method: &str, args: &[Value]) -> Result<Value, EvalError> {
    let num = |i: usize| args.get(i).and_then(value::as_f64);
    let arg = |i: usize| args.get(i).unwrap_or(&Value::Hamna);
    let hex = |bytes: &[u8]| bytes.iter().map(|x| format!("{x:02x}")).collect::<String>();
    Ok(match method {
        "clona" => Value::Baiti(b.clone()),
        "urefu" => Value::Namba(b.len() as f64),
        "tupu" => Value::Ukweli(b.is_empty()),
        "kata" => {
            let len = b.len() as f64;
            let start = num(0).unwrap_or(0.0).clamp(0.0, len) as usize;
            let end = num(1).unwrap_or(len).clamp(start as f64, len) as usize;
            Value::Baiti(b[start..end].into())
        }
        "tafuta" => Value::Chaguo(find_bytes(b, arg(0)).map(|i| Box::new(Value::Namba(i as f64)))),
        "ina" => Value::Ukweli(find_bytes(b, arg(0)).is_some()),
        "anza_na" => Value::Ukweli(bytes_of(arg(0)).is_some_and(|p| b.starts_with(&p))),
        "maliza_na" => Value::Ukweli(bytes_of(arg(0)).is_some_and(|p| b.ends_with(&p))),
        "gawanya" => {
            let sep = bytes_of(arg(0)).unwrap_or_default();
            let mut parts = Vec::new();
            if sep.is_empty() {
                parts.push(Value::Baiti(b.clone()));
            } else {
                let mut rest = &b[..];
                while let Some(i) = rest.windows(sep.len()).position(|w| w == &sep[..]) {
                    parts.push(Value::Baiti(rest[..i].into()));
                    rest = &rest[i + sep.len()..];
                }
                parts.push(Value::Baiti(rest.into()));
            }
            Value::list(parts)
        }
        "geuza" => Value::Baiti(b.iter().rev().copied().collect::<Vec<_>>().into()),
        "kwa_neno" => match std::str::from_utf8(b) {
            Ok(s) => Value::sawa(Value::neno(s)),
            Err(e) => Value::kosa(format!(
                "baiti si UTF-8 halali (kuanzia baiti #{})",
                e.valid_up_to()
            )),
        },
        "kwa_orodha" => Value::list(b.iter().map(|&x| Value::Namba(x as f64)).collect()),
        "hex" => Value::neno(hex(b)),
        "base64" => {
            use base64::Engine;
            Value::neno(base64::engine::general_purpose::STANDARD.encode(b))
        }
        "hashi_sha256" => {
            use sha2::Digest;
            Value::neno(hex(&sha2::Sha256::digest(b)))
        }
        "hashi_sha512" => {
            use sha2::Digest;
            Value::neno(hex(&sha2::Sha512::digest(b)))
        }
        "soma_nambari" => {
            let at = num(0).unwrap_or(-1.0);
            let width = num(1).unwrap_or(1.0) as usize;
            let little = value::as_string(arg(2)).is_some_and(|m| m == "le");
            let value = (at >= 0.0 && matches!(width, 1 | 2 | 4 | 8))
                .then(|| b.get(at as usize..at as usize + width))
                .flatten()
                .map(|bytes| {
                    let mut buf = [0u8; 8];
                    if little {
                        buf[..width].copy_from_slice(bytes);
                        u64::from_le_bytes(buf)
                    } else {
                        buf[8 - width..].copy_from_slice(bytes);
                        u64::from_be_bytes(buf)
                    }
                });
            Value::Chaguo(value.map(|n| Box::new(Value::Namba(n as f64))))
        }
        _ => return Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
    })
}

/// The elements `kwa x katika v` visits: an `Orodha`'s items, or a `Kamusi`'s entries as
/// `Jozi(ufunguo, thamani)`. The loop iterates this snapshot (shared, not copied), so the body
/// may mutate `v`.
pub(crate) fn iter_items(v: Value) -> Result<Rc<Vec<Value>>, EvalError> {
    match v {
        Value::Orodha(items) => Ok(items),
        Value::Kamusi(map) => Ok(Rc::new(
            Rc::unwrap_or_clone(map)
                .into_iter()
                .map(|(k, v)| Value::Jozi(Box::new(k.to_value()), Box::new(v)))
                .collect(),
        )),
        Value::Baiti(b) => Ok(Rc::new(b.iter().map(|&x| Value::Namba(x as f64)).collect())),
        _ => Err(EvalError::TypeErr(
            "kwa...katika inashughulikia Orodha, Kamusi na Baiti tu".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_grapheme_count_matches_segmentation() {
        for s in [
            "",
            "a",
            "kipengele 123",
            "\r\n",
            "\r\r\n",
            "\n\r",
            "a\r\nb\r\n\r\n",
            "\r",
            "\t\0",
            "héllo",
            "👍🏽x",
            "e\u{301}",
        ] {
            assert_eq!(grapheme_count(s), s.graphemes(true).count(), "{s:?}");
        }
        // A '\r', a "\r\n" or a non-ASCII character at every position across word boundaries.
        for odd in ["\r", "\r\n", "é", "\u{7f}"] {
            for at in 0..=17 {
                let s = format!("{}{odd}{}", "a".repeat(at), "b".repeat(17 - at));
                assert_eq!(grapheme_count(&s), s.graphemes(true).count(), "{s:?}");
            }
        }
    }

    #[test]
    fn whole_numbers_format_like_floats() {
        let limit = 9_007_199_254_740_992.0f64;
        let mut samples = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            7.0,
            1e15,
            -1e15,
            limit - 1.0,
            1.0 - limit,
        ];
        samples.extend([
            limit,
            -limit,
            2.0 * limit,
            1e21,
            0.5,
            -2.5,
            1e-7,
            123456.789,
        ]);
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        for _ in 0..10_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            samples.push((x % (1 << 54)) as f64 - limit);
        }
        for n in samples {
            assert_eq!(value::format_namba(n), n.to_string(), "{n:?}");
        }
    }
}

#[cfg(test)]
mod method_table_tests {
    use super::*;
    use std::cell::RefCell;

    /// One value of each receiver kind that can be built without I/O.
    fn samples() -> Vec<(Kind, Value)> {
        let gc = Rc::new(RefCell::new(Value::Namba(1.0)));
        vec![
            (Kind::Neno, Value::neno("abc")),
            (Kind::Orodha, Value::list(vec![Value::Namba(1.0)])),
            (Kind::Kamusi, Value::Kamusi(Rc::default())),
            (Kind::Seti, Value::Seti(Rc::default())),
            (
                Kind::Chaguo,
                Value::Chaguo(Some(Box::new(Value::Namba(1.0)))),
            ),
            (Kind::Tokeo, Value::Tokeo(Ok(Box::new(Value::Namba(1.0))))),
            (
                Kind::Jozi,
                Value::Jozi(Box::new(Value::Namba(1.0)), Box::new(Value::Hamna)),
            ),
            (Kind::Wakati, Value::Wakati(0.0)),
            (
                Kind::KashaGCDhaifu,
                Value::KashaGCDhaifu(Rc::downgrade(&gc)),
            ),
            (Kind::KashaGC, Value::KashaGC(gc)),
            (
                Kind::Kumbukumbu,
                Value::Kumbukumbu(Box::new(Value::Namba(1.0))),
            ),
        ]
    }

    fn unknown(r: &Result<Value, EvalError>, method: &str) -> bool {
        matches!(r, Err(EvalError::Unknown(m)) if *m == format!("njia '{method}' haijulikani"))
    }

    #[test]
    fn every_listed_method_is_implemented() {
        let args = [Value::Namba(0.0), Value::Namba(1.0)];
        for (kind, recv) in samples() {
            for method in PURE_METHODS[kind as usize] {
                assert!(is_pure_method(&recv, method));
                let r = pure_method(&recv, method, &args);
                assert!(
                    !unknown(&r, method),
                    "{kind:?}.{method} is listed but not implemented"
                );
            }
            for method in MUTATING_METHODS[kind as usize] {
                let mut target = recv.clone();
                let r = mutate(&mut target, method, &args);
                assert!(
                    !unknown(&r, method),
                    "{kind:?}.{method} is listed but not implemented"
                );
            }
        }
    }
}
