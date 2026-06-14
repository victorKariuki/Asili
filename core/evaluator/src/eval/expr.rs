//! Expression evaluation and pattern matching.

use asili_parser::{BinaryOp, Expr, Pattern, UnaryOp};
use unicode_segmentation::UnicodeSegmentation;

use crate::runtime::Runtime;
use crate::value::{
    self, binary_cmp_neno, binary_f64, binary_f64_cmp, parse_number, EvalError, EvalOut, MapKey,
    Value,
};
use std::cmp::Ordering;

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
        Pattern::Ident(name) => {
            rt.env.define(name, v.clone());
            true
        }
        Pattern::Struct {
            struct_name,
            fields,
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
        Expr::Ident(name) => rt
            .env
            .get(name)
            .ok_or_else(|| EvalError::UndefinedVar(name.clone())),
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
            let idx = value::as_f64(&i_val)
                .ok_or_else(|| EvalError::TypeErr("fahirisi inahitaji Namba".into()))?;
            let idx = idx as i64;
            let idx = if idx < 0 { 0 } else { idx as usize };
            match &b {
                Value::Orodha(v) => {
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
                _ => Err(EvalError::TypeErr("fahirisi inahitaji Orodha".into())),
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
                    if b == 0.0 {
                        return Err(EvalError::DivByZero);
                    }
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
                    let shift = if shift < 0 || shift > 63 { 0 } else { shift as u32 };
                    Ok(Value::Namba(((a as i64).wrapping_shl(shift)) as f64))
                }
                BinaryOp::Shr => {
                    let a = value::as_f64(&l)
                        .ok_or_else(|| EvalError::TypeErr("sogeza_kulia inahitaji Namba".into()))? as i64;
                    let b = value::as_f64(&r)
                        .ok_or_else(|| EvalError::TypeErr("sogeza_kulia inahitaji Namba".into()))?;
                    let shift = b as i32;
                    let shift = if shift < 0 || shift > 63 { 0 } else { shift as u32 };
                    Ok(Value::Namba((a.wrapping_shr(shift)) as f64))
                }
            }
        }
        Expr::Cast { expr, ty, .. } => {
            let v = super::eval_expr_impl(expr, rt)?;
            let t = ty.name.replace(' ', "");
            if t == "Namba" {
                let n = value::as_f64(&v)
                    .or_else(|| value::as_string(&v).and_then(|s| s.parse::<f64>().ok()))
                    .or_else(|| value::as_char(&v).map(|c| c as u32 as f64))
                    .or_else(|| match &v { Value::Chaguo(Some(inner)) => value::as_f64(inner), _ => None });
                Ok(Value::Namba(n.unwrap_or(0.0)))
            } else if t == "Neno" {
                Ok(Value::Neno(match &v {
                    Value::Namba(n) => n.to_string(),
                    Value::Ukweli(true) => "kweli".into(),
                    Value::Ukweli(false) => "si_kweli".into(),
                    Value::Neno(s) => s.clone(),
                    Value::Herufi(c) => c.to_string(),
                    Value::Wakati(secs) => secs.to_string(),
                    Value::Anuani(a) => a.to_string(),
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
            } else if t.starts_with("Biti") || t.starts_with("uBiti") {
                let n = value::as_f64(&v).unwrap_or(0.0);
                let n_i = n as i64;
                let fits = match t.as_str() {
                    "Biti8" => n_i >= i8::MIN as i64 && n_i <= i8::MAX as i64,
                    "Biti32" => n_i >= i32::MIN as i64 && n_i <= i32::MAX as i64,
                    "Biti64" => true,
                    "uBiti8" => n_i >= 0 && n_i <= u8::MAX as i64,
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
            if let Expr::Ident(name) = &**callee {
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
                (Value::Neno(s), "urefu") => Ok(Value::Namba(s.graphemes(true).count() as f64)),
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
                        .get(0)
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
                (Value::Orodha(v), "urefu") => Ok(Value::Namba(v.len() as f64)),
                (Value::Orodha(v), "ongeza") => {
                    let elem = args_val.first().cloned().unwrap_or(Value::Hamna);
                    let mut new_v = v.clone();
                    new_v.push(elem);
                    if let Expr::Ident(name) = &**receiver {
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
                        if let Expr::Ident(name) = &**receiver {
                            rt.env.set(name, Value::Orodha(new_v));
                        }
                        Ok(Value::Chaguo(Some(Box::new(removed))))
                    }
                }
                (Value::Orodha(_), "kila_mmoja") => Ok(Value::Tupu),
                (Value::Kamusi(m), "ingiza") | (Value::Kamusi(m), "weka_key") => {
                    let key_val = args_val.first().ok_or_else(|| EvalError::TypeErr("ingiza inahitaji ufunguo na thamani".into()))?;
                    let val = args_val.get(1).cloned().unwrap_or(Value::Hamna);
                    let key = MapKey::try_from_value(key_val)?;
                    let mut new_m = m.clone();
                    new_m.insert(key, val);
                    if let Expr::Ident(name) = &**receiver {
                        rt.env.set(name, Value::Kamusi(new_m));
                    }
                    Ok(Value::Tupu)
                }
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
                (Value::Tokeo(res), "kosa") => match res {
                    Ok(_) => Err(EvalError::TypeErr("kosa() inahitaji Tokeo(Err)".into())),
                    Err(e) => Ok((**e).clone()),
                },
                (Value::Tokeo(res), "angu") => {
                    let mbadala = args_val.first().cloned().unwrap_or(Value::Hamna);
                    Ok(match res {
                        Ok(v) => (**v).clone(),
                        Err(_) => mbadala,
                    })
                }
                (Value::Jozi(a, _), "kwanza") => Ok((**a).clone()),
                (Value::Jozi(_, b), "pili") => Ok((**b).clone()),
                (Value::Wakati(secs), "sekunde") => Ok(Value::Namba(*secs)),
                (Value::Neno(_), _) | (Value::Orodha(_), _) | (Value::Kamusi(_), _) | (Value::Jozi(_, _), _) | (Value::Wakati(_), _) | (Value::Anuani(_), _) | (Value::Chaguo(_), _) | (Value::Tokeo(_), _) => Err(EvalError::TypeErr(format!(
                    "njia '{method_name}' haijulikani kwa aina hii"
                ))),
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
                _ => Err(EvalError::TypeErr(format!(
                    "mwito wa njia '{method_name}' unahitaji Neno, Orodha au umbo"
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
