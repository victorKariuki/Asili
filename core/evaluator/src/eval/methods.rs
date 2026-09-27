//! Receiver methods that need no interpreter state.
//!
//! Shared by the tree-walking evaluator and the bytecode VM so both execution paths agree on
//! behaviour and error text.

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
        _ => Err(EvalError::Unknown(format!("njia '{method}' haijulikani"))),
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
            Value::Namba(n) if n.is_nan() => "Siyo_Namba".to_string(),
            Value::Namba(n) if n.is_infinite() && *n > 0.0 => "Ukomo".to_string(),
            Value::Namba(n) if n.is_infinite() => "-Ukomo".to_string(),
            Value::Namba(n) => n.to_string(),
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
