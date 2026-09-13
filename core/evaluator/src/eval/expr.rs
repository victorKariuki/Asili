//! Expression evaluation and pattern matching.

use asili_parser::{BinaryOp, Expr, Pattern, UnaryOp};
use unicode_segmentation::UnicodeSegmentation;

use crate::runtime::Runtime;
use crate::value::{
    self, big_numeric_binary_op, binary_cmp_neno, binary_f64, binary_f64_cmp, parse_number,
    EvalError, EvalOut, MapKey, Value,
};
use std::cmp::Ordering;
use std::rc::Rc;

pub(crate) fn match_and_bind_pattern(pat: &Pattern, v: &Value, rt: &mut Runtime<'_>) -> bool {
    match pat {
        Pattern::Wildcard => true,
        Pattern::Literal(Expr::Number(s)) => {
            if let Value::Namba(n) = v {
                (parse_number(s) - n).abs() < f64::EPSILON
            } else {
                false
            }
        }
        Pattern::Literal(Expr::String(s)) => matches!(v, Value::Neno(x) if x == s),
        Pattern::Literal(Expr::Bool(b)) => matches!(v, Value::Ukweli(x) if *x == *b),
        Pattern::Literal(Expr::Hamna) => matches!(v, Value::Hamna | Value::Chaguo(None)),
        Pattern::Literal(Expr::Char(c)) => matches!(v, Value::Herufi(x) if *x == *c),
        Pattern::Ident { name, .. } => {
            rt.env.define(name, v.clone());
            true
        }
        Pattern::Struct {
            struct_name,
            fields,
            ..
        } => {
            let Value::Struct(name, flds) = v else {
                return false;
            };
            if name != struct_name {
                return false;
            }
            for (fname, subpat) in fields {
                let Some((_, fval)) = flds.iter().find(|(n, _)| n == fname) else {
                    return false;
                };
                if !match_and_bind_pattern(subpat, fval, rt) {
                    return false;
                }
            }
            true
        }
        Pattern::Jozi(p1, p2) => {
            let Value::Jozi(a, b) = v else {
                return false;
            };
            match_and_bind_pattern(p1, a, rt) && match_and_bind_pattern(p2, b, rt)
        }
        Pattern::Enum {
            enum_name,
            variant_name,
            data,
            ..
        } => {
            // `Tokeo`/`Chaguo` have two runtime shapes: `Value::Tokeo`/`Value::Chaguo` (from
            // builtins like `gawio`/`kamusi.pata`/casts) and `Value::Enum("Tokeo"/"Chaguo", ...)`
            // (from explicit `Tokeo::Sawa(x)`-style construction, which the parser registers as a
            // real jenum). A `Tokeo::Sawa(v)`/`Tokeo::Kosa(e)`/`Chaguo::Kuna(v)`/`Chaguo::Hamna`
            // pattern must match both shapes, or matching a builtin-produced Tokeo/Chaguo against
            // its own "constructor" pattern silently never fires (no error, arm just doesn't run).
            if enum_name == "Tokeo" {
                if let Value::Tokeo(res) = v {
                    return match (variant_name.as_str(), data, res) {
                        ("Sawa", Some(dpat), Ok(dval)) => match_and_bind_pattern(dpat, dval, rt),
                        ("Sawa", None, Ok(_)) => true,
                        ("Kosa", Some(dpat), Err(dval)) => match_and_bind_pattern(dpat, dval, rt),
                        ("Kosa", None, Err(_)) => true,
                        _ => false,
                    };
                }
            }
            if enum_name == "Chaguo" {
                if let Value::Chaguo(opt) = v {
                    return match (variant_name.as_str(), data, opt) {
                        ("Kuna", Some(dpat), Some(dval)) => match_and_bind_pattern(dpat, dval, rt),
                        ("Kuna", None, Some(_)) => true,
                        ("Hamna", None, None) => true,
                        _ => false,
                    };
                }
            }
            let Value::Enum(en, vn, en_data) = v else {
                return false;
            };
            if en != enum_name || vn != variant_name {
                return false;
            }
            match (data, en_data) {
                (None, None) => true,
                (Some(dpat), Some(dval)) => match_and_bind_pattern(dpat, dval, rt),
                _ => false,
            }
        }
        Pattern::Literal(_) => false,
    }
}

pub(crate) fn eval_expr_inner(expr: &Expr, rt: &mut Runtime<'_>) -> Result<Value, EvalError> {
    match expr {
        Expr::Number(s) => Ok(Value::Namba(parse_number(s))),
        Expr::String(s) => Ok(Value::Neno(s.clone())),
        Expr::Bool(b) => Ok(Value::Ukweli(*b)),
        Expr::Char(c) => Ok(Value::Herufi(*c)),
        Expr::Hamna => Ok(Value::Hamna),
        Expr::Group(e) => super::eval_expr_impl(e, rt),
        Expr::Ident { name, .. } => rt
            .env
            .get(name)
            .ok_or_else(|| EvalError::UndefinedVar(name.clone())),
        Expr::List { elements, .. } => {
            let vals: Vec<Value> = elements
                .iter()
                .map(|e| super::eval_expr_impl(e, rt))
                .collect::<Result<_, _>>()?;
            Ok(Value::Orodha(vals))
        }
        Expr::Map { entries, .. } => {
            use std::collections::HashMap;
            let mut m: HashMap<MapKey, Value> = HashMap::new();
            for (k, v) in entries {
                let kval = super::eval_expr_impl(k, rt)?;
                let vval = super::eval_expr_impl(v, rt)?;
                let key = MapKey::try_from_value(&kval)?;
                m.insert(key, vval);
            }
            Ok(Value::Kamusi(m))
        }
        Expr::StructLiteral {
            struct_name,
            fields,
            ..
        } => {
            let st = rt
                .module
                .structs
                .iter()
                .find(|s| s.name == *struct_name)
                .ok_or_else(|| EvalError::TypeErr(format!("umbo haijulikani: {}", struct_name)))?;
            let mut flds: Vec<(String, Value)> = Vec::with_capacity(st.fields.len());
            for (fname, _) in &st.fields {
                let (_, fexpr) = fields
                    .iter()
                    .find(|(n, _)| n == fname)
                    .ok_or_else(|| EvalError::TypeErr(format!("umbo linahitaji uga: {}", fname)))?;
                flds.push((fname.clone(), super::eval_expr_impl(fexpr, rt)?));
            }
            Ok(Value::Struct(struct_name.clone(), flds))
        }
        Expr::EnumConstruct {
            enum_name,
            variant_name,
            data,
            ..
        } => {
            let _en = rt
                .module
                .enums
                .iter()
                .find(|e| e.name == *enum_name)
                .ok_or_else(|| EvalError::TypeErr(format!("jenum haijulikani: {}", enum_name)))?;
            let variant_data = if let Some(d) = data {
                Some(Box::new(super::eval_expr_impl(d, rt)?))
            } else {
                None
            };
            Ok(Value::Enum(enum_name.clone(), variant_name.clone(), variant_data))
        }
        Expr::FieldAccess { receiver, field, .. } => {
            let recv = super::eval_expr_impl(receiver, rt)?;
            match &recv {
                Value::Struct(_, flds) => flds
                    .iter()
                    .find(|(n, _)| n == field)
                    .map(|(_, v)| v.clone())
                    .ok_or_else(|| EvalError::TypeErr(format!("uga haijulikani: {}", field))),
                _ => Err(EvalError::TypeErr("uga unahitaji kitu cha aina ya umbo".into())),
            }
        }
        Expr::Index { base, index, .. } => {
            let b = super::eval_expr_impl(base, rt)?;
            let i_val = super::eval_expr_impl(index, rt)?;
            match &b {
                Value::Kamusi(m) => {
                    let key = MapKey::try_from_value(&i_val)?;
                    Ok(m.get(&key).cloned().unwrap_or(Value::Hamna))
                }
                Value::Orodha(v) => {
                    let idx = value::as_f64(&i_val)
                        .ok_or_else(|| EvalError::TypeErr("fahirisi inahitaji Namba".into()))?;
                    let idx = idx as i64;
                    let idx = if idx < 0 { 0 } else { idx as usize };
                    if idx >= v.len() {
                        let msg = format!("fahirisi nje ya mipaka: {} (urefu {})", idx, v.len());
                        let kosa = Value::Struct(
                            "KosaMipaka".to_string(),
                            vec![("ujumbe".to_string(), Value::Neno(msg))],
                        );
                        Ok(Value::Tokeo(Err(Box::new(kosa))))
                    } else {
                        Ok(Value::Tokeo(Ok(Box::new(v[idx].clone()))))
                    }
                }
                _ => Err(EvalError::TypeErr("fahirisi inahitaji Orodha au Kamusi".into())),
            }
        }
        Expr::Unary { op, expr, .. } => {
            let v = super::eval_expr_impl(expr, rt)?;
            match op {
                UnaryOp::Neg => {
                    let n = value::as_f64(&v)
                        .ok_or_else(|| EvalError::TypeErr("- inahitaji Namba".into()))?;
                    Ok(Value::Namba(-n))
                }
                UnaryOp::Not => {
                    let b = match &v {
                        Value::Ukweli(x) => *x,
                        _ => return Err(EvalError::TypeErr("siyo inahitaji Ukweli".into())),
                    };
                    Ok(Value::Ukweli(!b))
                }
                UnaryOp::Jaribu => match &v {
                    Value::Tokeo(Ok(inner)) => Ok((**inner).clone()),
                    Value::Tokeo(Err(e)) => Err(EvalError::Unknown(format!("KOSA: {e:?}"))),
                    Value::Chaguo(Some(inner)) => Ok((**inner).clone()),
                    Value::Chaguo(None) => Err(EvalError::Unknown("Chaguo: Hamna".into())),
                    _ => Err(EvalError::TypeErr("jaribu inahitaji Tokeo/Chaguo".into())),
                },
                UnaryOp::BitNot => {
                    let n = value::as_f64(&v)
                        .ok_or_else(|| EvalError::TypeErr("siyo_biti inahitaji Namba".into()))?;
                    let bits = n as i64;
                    Ok(Value::Namba(!bits as f64))
                }
                // TODO(Phase III): BorrowImm/BorrowMut should produce Rejeo/Rejeo_Tenda values
                // tracked by a borrow checker. Currently they are identity ops — ownership is not enforced.
                UnaryOp::BorrowImm | UnaryOp::BorrowMut => Ok(v),
            }
        }
        Expr::Binary { left, op, right, .. } => {
            let l = super::eval_expr_impl(left, rt)?;
            let r = match op {
                BinaryOp::And => {
                    if let Value::Ukweli(false) = &l {
                        return Ok(Value::Ukweli(false));
                    }
                    super::eval_expr_impl(right, rt)?
                }
                BinaryOp::Or => {
                    if let Value::Ukweli(true) = &l {
                        return Ok(Value::Ukweli(true));
                    }
                    super::eval_expr_impl(right, rt)?
                }
                _ => super::eval_expr_impl(right, rt)?,
            };
            // Namba_Kuu/Namba_Sahihi arithmetic short-circuits before the plain-f64 path below —
            // a Namba operand mixed with either widens (infallibly) to match, matching the cast
            // direction documented for these types (Namba -> Namba_Kuu/Namba_Sahihi is
            // infallible; the reverse is fallible and goes through `kama`, not here).
            if matches!(l, Value::NambaKuu(_) | Value::NambaSahihi(_))
                || matches!(r, Value::NambaKuu(_) | Value::NambaSahihi(_))
            {
                if let Some(result) = big_numeric_binary_op(&l, op, &r)? {
                    return Ok(result);
                }
            }
            match op {
                BinaryOp::Add => {
                    match (value::as_string(&l), value::as_string(&r)) {
                        (Some(s1), Some(s2)) => Ok(Value::Neno(format!("{s1}{s2}"))),
                        _ => binary_f64(&l, &r, "+", |a, b| a + b),
                    }
                }
                BinaryOp::Sub => binary_f64(&l, &r, "-", |a, b| a - b),
                BinaryOp::Mul => binary_f64(&l, &r, "*", |a, b| a * b),
                BinaryOp::Div => {
                    let a = value::as_f64(&l)
                        .ok_or_else(|| EvalError::TypeErr("/ inahitaji Namba".into()))?;
                    let b = value::as_f64(&r)
                        .ok_or_else(|| EvalError::TypeErr("/ inahitaji Namba".into()))?;
                    Ok(Value::Namba(a / b))
                }
                BinaryOp::Rem => binary_f64(&l, &r, "%", |a, b| a % b),
                BinaryOp::Pow => binary_f64(&l, &r, "**", |a, b| a.powf(b)),
                BinaryOp::Eq => Ok(Value::Ukweli(l == r)),
                BinaryOp::Ne => Ok(Value::Ukweli(l != r)),
                BinaryOp::Gt => binary_cmp_neno(&l, &r, |o| o == Ordering::Greater)
                    .map(Ok)
                    .unwrap_or_else(|| binary_f64_cmp(&l, &r, ">", |a, b| a > b)),
                BinaryOp::Lt => binary_cmp_neno(&l, &r, |o| o == Ordering::Less)
                    .map(Ok)
                    .unwrap_or_else(|| binary_f64_cmp(&l, &r, "<", |a, b| a < b)),
                BinaryOp::Ge => binary_cmp_neno(&l, &r, |o| o != Ordering::Less)
                    .map(Ok)
                    .unwrap_or_else(|| binary_f64_cmp(&l, &r, ">=", |a, b| a >= b)),
                BinaryOp::Le => binary_cmp_neno(&l, &r, |o| o != Ordering::Greater)
                    .map(Ok)
                    .unwrap_or_else(|| binary_f64_cmp(&l, &r, "<=", |a, b| a <= b)),
                BinaryOp::And => {
                    let a = match &l {
                        Value::Ukweli(x) => *x,
                        _ => return Err(EvalError::TypeErr("na inahitaji Ukweli".into())),
                    };
                    let b = match &r {
                        Value::Ukweli(x) => *x,
                        _ => return Err(EvalError::TypeErr("na inahitaji Ukweli".into())),
                    };
                    Ok(Value::Ukweli(a && b))
                }
                BinaryOp::Or => {
                    let a = match &l {
                        Value::Ukweli(x) => *x,
                        _ => return Err(EvalError::TypeErr("au inahitaji Ukweli".into())),
                    };
                    let b = match &r {
                        Value::Ukweli(x) => *x,
                        _ => return Err(EvalError::TypeErr("au inahitaji Ukweli".into())),
                    };
                    Ok(Value::Ukweli(a || b))
                }
                BinaryOp::BitAnd => {
                    let a = value::as_f64(&l)
                        .ok_or_else(|| EvalError::TypeErr("na_biti inahitaji Namba".into()))? as i64;
                    let b = value::as_f64(&r)
                        .ok_or_else(|| EvalError::TypeErr("na_biti inahitaji Namba".into()))? as i64;
                    Ok(Value::Namba((a & b) as f64))
                }
                BinaryOp::BitOr => {
                    let a = value::as_f64(&l)
                        .ok_or_else(|| EvalError::TypeErr("au_biti inahitaji Namba".into()))? as i64;
                    let b = value::as_f64(&r)
                        .ok_or_else(|| EvalError::TypeErr("au_biti inahitaji Namba".into()))? as i64;
                    Ok(Value::Namba((a | b) as f64))
                }
                BinaryOp::BitXor => {
                    let a = value::as_f64(&l)
                        .ok_or_else(|| EvalError::TypeErr("xor_biti inahitaji Namba".into()))? as i64;
                    let b = value::as_f64(&r)
                        .ok_or_else(|| EvalError::TypeErr("xor_biti inahitaji Namba".into()))? as i64;
                    Ok(Value::Namba((a ^ b) as f64))
                }
                BinaryOp::Shl => {
                    let a = value::as_f64(&l)
                        .ok_or_else(|| EvalError::TypeErr("sogeza_kushoto inahitaji Namba".into()))? as i64;
                    let b = value::as_f64(&r)
                        .ok_or_else(|| EvalError::TypeErr("sogeza_kushoto inahitaji Namba".into()))?;
                    let shift = b as i32;
                    let shift = if !(0..=63).contains(&shift) { 0 } else { shift as u32 };
                    Ok(Value::Namba((a.wrapping_shl(shift)) as f64))
                }
                BinaryOp::Shr => {
                    let a = value::as_f64(&l)
                        .ok_or_else(|| EvalError::TypeErr("sogeza_kulia inahitaji Namba".into()))? as i64;
                    let b = value::as_f64(&r)
                        .ok_or_else(|| EvalError::TypeErr("sogeza_kulia inahitaji Namba".into()))?;
                    let shift = b as i32;
                    let shift = if !(0..=63).contains(&shift) { 0 } else { shift as u32 };
                    Ok(Value::Namba((a.wrapping_shr(shift)) as f64))
                }
            }
        }
        Expr::Cast { expr, ty, .. } => {
            let v = super::eval_expr_impl(expr, rt)?;
            let t = ty.name.replace(' ', "");
            if t == "Namba" && !matches!(v, Value::NambaKuu(_) | Value::NambaSahihi(_)) {
                let n = value::as_f64(&v)
                    .or_else(|| match &v { Value::Ukweli(b) => Some(if *b { 1.0 } else { 0.0 }), _ => None })
                    .or_else(|| value::as_string(&v).and_then(|s| s.parse::<f64>().ok()))
                    .or_else(|| value::as_char(&v).map(|c| c as u32 as f64))
                    .or_else(|| match &v { Value::Chaguo(Some(inner)) => value::as_f64(inner), _ => None });
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
                    _ => value::as_f64(&v).map(|n| BigInt::from(n as i64)).unwrap_or_default(),
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
        Expr::Call { callee, args, .. } => {
            let args_val: Vec<Value> = args
                .iter()
                .map(|a| super::eval_expr_impl(a, rt))
                .collect::<Result<_, _>>()?;
            if let Expr::Ident { name, .. } = &**callee {
                // tenda needs the current Module to spawn a thread running a named kazi from
                // it — unlike every other builtin, which is a plain Fn(&[Value]) with no
                // access to rt. Intercepted here, before the generic builtins dispatch, rather
                // than trying to thread Module access through BuiltinFn's signature for this
                // one function.
                if name == "tenda" {
                    return crate::builtins::sambamba::tenda(rt.module, &args_val);
                }
                if let Some(f) = rt.builtins.get(name) {
                    return f(&args_val);
                }
                // Clone so we can mutably borrow rt.env below without conflict.
                let module_fn = rt.module.functions.iter().find(|x| x.name == *name).cloned();
                if let Some(f) = module_fn {
                    rt.env.push_scope();
                    for (i, p) in f.params.iter().enumerate() {
                        let val = args_val.get(i).cloned().unwrap_or(Value::Hamna);
                        rt.env.define(&p.name, val);
                    }
                    let out = super::eval_block_impl(&f.body, rt);
                    rt.env.pop_scope();
                    return match out {
                        Ok(EvalOut::Return(v)) => Ok(v),
                        Ok(_) => Ok(Value::Tupu),
                        Err(e) => Err(e),
                    };
                }
                // Not found as builtin or module function.
                return Err(EvalError::UndefinedVar(name.clone()));
            }
            Err(EvalError::TypeErr("kitu kinachoweza kuitwa kinahitajika".into()))
        }
        Expr::MethodCall {
            receiver,
            method_name,
            args,
            ..
        } => {
            let recv = super::eval_expr_impl(receiver, rt)?;
            let args_val: Vec<Value> = args
                .iter()
                .map(|a| super::eval_expr_impl(a, rt))
                .collect::<Result<_, _>>()?;
            match (&recv, method_name.as_str()) {
                (Value::Neno(s), "clona") => Ok(Value::Neno(s.clone())),
                (Value::Neno(s), "urefu") => Ok(Value::Namba(s.graphemes(true).count() as f64)),
                (Value::Neno(s), "herufi_kwa") => {
                    // Grapheme-indexed, matching .urefu()'s existing counting convention (not
                    // byte or codepoint index) — a tokenizer walking "what's at position N"
                    // wants the same units .urefu() reports N in.
                    let idx = args_val.first().and_then(value::as_f64).map(|n| n as i64).unwrap_or(-1);
                    let ch = if idx >= 0 {
                        s.graphemes(true).nth(idx as usize).and_then(|g| g.chars().next())
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

                (Value::Orodha(v), "ongeza") => {
                    let elem = args_val.first().cloned().unwrap_or(Value::Hamna);
                    let mut new_v = v.clone();
                    new_v.push(elem);
                    if let Expr::Ident { name, .. } = &**receiver {
                        rt.env.set(name, Value::Orodha(new_v));
                    }
                    Ok(Value::Tupu)
                }
                (Value::Orodha(v), "ingiza") => {
                    let idx = args_val
                        .first()
                        .and_then(value::as_f64)
                        .map(|n| n as usize)
                        .ok_or_else(|| EvalError::TypeErr("ingiza inahitaji index na thamani".into()))?;
                    let val = args_val.get(1).cloned().unwrap_or(Value::Hamna);
                    if idx >= v.len() {
                        return Err(EvalError::TypeErr("ingiza: index nje ya mipaka".into()));
                    }
                    let mut new_v = v.clone();
                    new_v[idx] = val;
                    if let Expr::Ident { name, .. } = &**receiver {
                        rt.env.set(name, Value::Orodha(new_v));
                    }
                    Ok(Value::Tupu)
                }
                (Value::Orodha(v), "ondoa") => {
                    let idx = args_val
                        .first()
                        .and_then(value::as_f64)
                        .map(|n| n as usize)
                        .unwrap_or(0);
                    if idx >= v.len() {
                        Ok(Value::Chaguo(None))
                    } else {
                        let mut new_v = v.clone();
                        let removed = new_v.remove(idx);
                        if let Expr::Ident { name, .. } = &**receiver {
                            rt.env.set(name, Value::Orodha(new_v));
                        }
                        Ok(Value::Chaguo(Some(Box::new(removed))))
                    }
                }
                (Value::Orodha(v), "kila_mmoja") => {
                    let Some(cb_name) = args_val.first().and_then(value::as_string) else {
                        return Ok(Value::Tupu); // no callback provided — no-op
                    };
                    for elem in v {
                        if let Some(f) = rt.builtins.get(&cb_name) {
                            f(std::slice::from_ref(elem))?;
                        } else if let Some(f) = rt.module.functions.iter().find(|x| x.name == cb_name).cloned() {
                            rt.env.push_scope();
                            if let Some(p) = f.params.first() {
                                rt.env.define(&p.name, elem.clone());
                            }
                            super::eval_block_impl(&f.body, rt)?;
                            rt.env.pop_scope();
                        } else {
                            return Err(EvalError::TypeErr(format!("kazi haijulikani: {cb_name}")));
                        }
                    }
                    Ok(Value::Tupu)
                }
                (Value::Kamusi(m), "ingiza") | (Value::Kamusi(m), "weka_key") => {
                    let key_val = args_val.first().ok_or_else(|| EvalError::TypeErr("ingiza inahitaji ufunguo na thamani".into()))?;
                    let val = args_val.get(1).cloned().unwrap_or(Value::Hamna);
                    let key = MapKey::try_from_value(key_val)?;
                    let mut new_m = m.clone();
                    new_m.insert(key, val);
                    if let Expr::Ident { name, .. } = &**receiver {
                        rt.env.set(name, Value::Kamusi(new_m));
                    }
                    Ok(Value::Tupu)
                }
                (Value::Kamusi(m), "clona") => Ok(Value::Kamusi(m.clone())),
                (Value::Kamusi(m), "idadi") => Ok(Value::Namba(m.len() as f64)),
                (Value::Kamusi(m), "pata") => {
                    let key_val = args_val.first().ok_or_else(|| EvalError::TypeErr("pata inahitaji ufunguo".into()))?;
                    let key = MapKey::try_from_value(key_val)?;
                    Ok(Value::Chaguo(m.get(&key).cloned().map(Box::new)))
                }
                (Value::Kamusi(m), "funguo") => {
                    let keys: Vec<Value> = m
                        .keys()
                        .map(|k| match k {
                            MapKey::Neno(s) => Value::Neno(s.clone()),
                            MapKey::Namba(bits) => Value::Namba(f64::from_bits(*bits)),
                            MapKey::Ukweli(b) => Value::Ukweli(*b),
                            MapKey::Herufi(c) => Value::Herufi(*c),
                        })
                        .collect();
                    Ok(Value::Orodha(keys))
                }
                (Value::Kamusi(m), "vipo") => {
                    let key_val = args_val.first().ok_or_else(|| EvalError::TypeErr("vipo inahitaji ufunguo".into()))?;
                    let key = MapKey::try_from_value(key_val)?;
                    Ok(Value::Ukweli(m.contains_key(&key)))
                }
                (Value::Seti(s), "ongeza") => {
                    let v = args_val.first().ok_or_else(|| EvalError::TypeErr("ongeza inahitaji thamani".into()))?;
                    let key = MapKey::try_from_value(v)?;
                    let mut new_s = s.clone();
                    new_s.insert(key);
                    if let Expr::Ident { name, .. } = &**receiver {
                        rt.env.set(name, Value::Seti(new_s));
                    }
                    Ok(Value::Tupu)
                }
                (Value::Seti(s), "ondoa") => {
                    let v = args_val.first().ok_or_else(|| EvalError::TypeErr("ondoa inahitaji thamani".into()))?;
                    let key = MapKey::try_from_value(v)?;
                    let mut new_s = s.clone();
                    let removed = new_s.remove(&key);
                    if let Expr::Ident { name, .. } = &**receiver {
                        rt.env.set(name, Value::Seti(new_s));
                    }
                    Ok(Value::Ukweli(removed))
                }
                (Value::Seti(s), "ina") => {
                    let v = args_val.first().ok_or_else(|| EvalError::TypeErr("ina inahitaji thamani".into()))?;
                    let key = MapKey::try_from_value(v)?;
                    Ok(Value::Ukweli(s.contains(&key)))
                }
                (Value::Seti(s), "urefu") => Ok(Value::Namba(s.len() as f64)),
                (Value::Seti(s), "clona") => Ok(Value::Seti(s.clone())),
                (Value::Seti(s), "orodha") => {
                    Ok(Value::Orodha(s.iter().map(MapKey::to_value).collect()))
                }
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
                    let msg = value::as_string(args_val.first().unwrap_or(&Value::Hamna)).unwrap_or_else(|| "Chaguo: Hamna".into());
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
                // TODO: Missing Orodha methods from spec: badilisha(idx, val), chuja(kazi),
                // panga(kazi), pata(idx) -> Chaguo<T>, chunguza(kazi) -> Ukweli, onyesha().
                // Missing Kamusi methods: ondoa(key), thamani() -> Orodha<V>, idadi() -> Namba.
                (Value::Struct(struct_name, _), _) => {
                    // Search inherent impls first (no trait_name), then trait impls.
                    // This allows `shughuli ya Foo { }` and `shughuli ya Foo: Sifa { }`
                    // to coexist; methods from both blocks are callable on the same receiver.
                    let method = rt
                        .module
                        .impls
                        .iter()
                        .filter(|i| i.target == *struct_name && i.trait_name.is_none())
                        .flat_map(|i| i.body.iter())
                        .find(|mf| mf.name == *method_name)
                        .or_else(|| {
                            rt.module
                                .impls
                                .iter()
                                .filter(|i| i.target == *struct_name && i.trait_name.is_some())
                                .flat_map(|i| i.body.iter())
                                .find(|mf| mf.name == *method_name)
                        })
                        .cloned()
                        .ok_or_else(|| {
                            let has_any_impl =
                                rt.module.impls.iter().any(|i| i.target == *struct_name);
                            if has_any_impl {
                                EvalError::TypeErr(format!(
                                    "njia '{}' haijulikani kwa umbo '{}'",
                                    method_name, struct_name
                                ))
                            } else {
                                EvalError::TypeErr(format!(
                                    "umbo '{}' hauna shughuli yoyote iliyofafanuliwa",
                                    struct_name
                                ))
                            }
                        })?;

                    rt.env.push_scope();
                    for (i, p) in method.params.iter().enumerate() {
                        let val = if i == 0 {
                            recv.clone()
                        } else {
                            args_val.get(i - 1).cloned().unwrap_or(Value::Hamna)
                        };
                        rt.env.define(&p.name, val);
                    }
                    let out = super::eval_block_impl(&method.body, rt);
                    rt.env.pop_scope();
                    match out {
                        Ok(EvalOut::Return(v)) => Ok(v),
                        Ok(_) => Ok(Value::Tupu),
                        Err(e) => Err(e),
                    }
                }
                // Built-in methods on Tokeo-as-Enum (e.g. Tokeo::Sawa(x).ni_kosa())
                (Value::Enum(en, vn, data), "ni_kosa") if en == "Tokeo" => {
                    Ok(Value::Ukweli(vn == "Kosa"))
                }
                (Value::Enum(en, vn, data), "ni_sawa") if en == "Tokeo" => {
                    Ok(Value::Ukweli(vn == "Sawa"))
                }
                (Value::Enum(en, vn, data), "kosa") if en == "Tokeo" => {
                    if vn == "Kosa" {
                        Ok(data.as_ref().map(|v| *v.clone()).unwrap_or(Value::Hamna))
                    } else {
                        Err(EvalError::TypeErr("kosa() inahitaji Tokeo(Kosa)".into()))
                    }
                }
                (Value::Enum(en, vn, data), "angu") if en == "Tokeo" => {
                    let mbadala = args_val.first().cloned().unwrap_or(Value::Hamna);
                    if vn == "Sawa" {
                        Ok(data.as_ref().map(|v| *v.clone()).unwrap_or(Value::Hamna))
                    } else {
                        Ok(mbadala)
                    }
                }
                // Built-in methods on Chaguo-as-Enum (e.g. Chaguo::Kuna(x).ni_po())
                (Value::Enum(en, vn, _), "ni_po") if en == "Chaguo" => {
                    Ok(Value::Ukweli(vn == "Kuna"))
                }
                (Value::Enum(en, vn, _), "ni_tupu") if en == "Chaguo" => {
                    Ok(Value::Ukweli(vn == "Hamna"))
                }
                (Value::Enum(en, vn, data), "angu") if en == "Chaguo" => {
                    let mbadala = args_val.first().cloned().unwrap_or(Value::Hamna);
                    if vn == "Kuna" {
                        Ok(data.as_ref().map(|v| *v.clone()).unwrap_or(Value::Hamna))
                    } else {
                        Ok(mbadala)
                    }
                }
                (Value::Enum(en, vn, data), "hakikisha") if en == "Chaguo" => {
                    if vn == "Kuna" {
                        Ok(data.as_ref().map(|v| *v.clone()).unwrap_or(Value::Hamna))
                    } else {
                        let msg = value::as_string(args_val.first().unwrap_or(&Value::Hamna))
                            .unwrap_or_else(|| "Chaguo: Hamna".into());
                        Err(EvalError::Panic(msg))
                    }
                }
                (Value::KashaGC(cell), "pata") => cell
                    .try_borrow()
                    .map(|v| v.clone())
                    .map_err(|_| EvalError::Panic("kasha_gc: pata: tayari inatumika (weka ndani ya .weka)".into())),
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
                        None => Ok(Value::Tokeo(Err(Box::new(Value::Neno("faili: imefungwa tayari".into()))))),
                    }
                }
                (Value::Faili(cell), "andika") => {
                    use std::io::Write;
                    let data = value::as_string(args_val.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
                    let mut guard = cell.borrow_mut();
                    match guard.0.as_mut() {
                        Some(f) => match f.write_all(data.as_bytes()) {
                            Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                            Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
                        },
                        None => Ok(Value::Tokeo(Err(Box::new(Value::Neno("faili: imefungwa tayari".into()))))),
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
                        None => Ok(Value::Tokeo(Err(Box::new(Value::Neno("mkondo: imefungwa tayari".into()))))),
                    }
                }
                (Value::Mkondo(cell), "andika") => {
                    use std::io::Write;
                    let data = value::as_string(args_val.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
                    let mut guard = cell.borrow_mut();
                    match guard.0.as_mut() {
                        Some(s) => match s.write_all(data.as_bytes()) {
                            Ok(()) => Ok(Value::Tokeo(Ok(Box::new(Value::Tupu)))),
                            Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
                        },
                        None => Ok(Value::Tokeo(Err(Box::new(Value::Neno("mkondo: imefungwa tayari".into()))))),
                    }
                }
                (Value::Mkondo(cell), "funga") => {
                    cell.borrow_mut().0.take();
                    Ok(Value::Tupu)
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
                                Err(Box::new(Value::Neno("njia: upande wa pili umefungwa".into())))
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
                (Value::Enum(enum_name, _, _), _) => {
                    let method = rt
                        .module
                        .impls
                        .iter()
                        .filter(|i| i.target == *enum_name && i.trait_name.is_none())
                        .flat_map(|i| i.body.iter())
                        .find(|mf| mf.name == *method_name)
                        .or_else(|| {
                            rt.module
                                .impls
                                .iter()
                                .filter(|i| i.target == *enum_name && i.trait_name.is_some())
                                .flat_map(|i| i.body.iter())
                                .find(|mf| mf.name == *method_name)
                        })
                        .cloned()
                        .ok_or_else(|| {
                            let has_any_impl =
                                rt.module.impls.iter().any(|i| i.target == *enum_name);
                            if has_any_impl {
                                EvalError::TypeErr(format!(
                                    "njia '{}' haijulikani kwa jenum '{}'",
                                    method_name, enum_name
                                ))
                            } else {
                                EvalError::TypeErr(format!(
                                    "jenum '{}' hauna shughuli yoyote iliyofafanuliwa",
                                    enum_name
                                ))
                            }
                        })?;

                    rt.env.push_scope();
                    for (i, p) in method.params.iter().enumerate() {
                        let val = if i == 0 {
                            recv.clone()
                        } else {
                            args_val.get(i - 1).cloned().unwrap_or(Value::Hamna)
                        };
                        rt.env.define(&p.name, val);
                    }
                    let out = super::eval_block_impl(&method.body, rt);
                    rt.env.pop_scope();
                    match out {
                        Ok(EvalOut::Return(v)) => Ok(v),
                        Ok(_) => Ok(Value::Tupu),
                        Err(e) => Err(e),
                    }
                }
                _ => Err(EvalError::TypeErr(format!(
                    "mwito wa njia '{method_name}' unahitaji Neno, Orodha, jenum au umbo"
                ))),
            }
        }
        Expr::Propagate { expr, .. } => {
            let v = super::eval_expr_impl(expr, rt)?;
            match v {
                Value::Tokeo(Ok(inner)) => Ok(*inner),
                Value::Tokeo(Err(e)) => Err(EvalError::Propagate(Value::Tokeo(Err(e)))),
                Value::Chaguo(Some(inner)) => Ok(*inner),
                Value::Chaguo(None) => Err(EvalError::Unknown("? Chaguo Hamna".into())),
                _ => Err(EvalError::TypeErr("? inahitaji Tokeo/Chaguo".into())),
            }
        }
    }
}
