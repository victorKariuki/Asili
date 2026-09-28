//! Expression evaluation and pattern matching.

use asili_parser::{BinaryOp, Expr, Pattern, UnaryOp};

use crate::runtime::Runtime;
use crate::value::{parse_number, EvalError, EvalOut, MapKey, Value};

use super::methods::{index_element, index_value};

/// `base[index]`; `as_tokeo` for the `b[i]?` / `jaribu b[i]` forms.
fn eval_index(
    base: &Expr,
    index: &Expr,
    rt: &mut Runtime<'_>,
    as_tokeo: bool,
) -> Result<Value, EvalError> {
    rt.count_index_read();
    let i_val = super::eval_expr_impl(index, rt)?;
    let read = if as_tokeo { index_value } else { index_element };
    if let Expr::Ident { name, .. } = base {
        if let Some(value) = rt.env.get_ref(name) {
            return read(value, &i_val);
        }
    }
    let b = super::eval_expr_impl(base, rt)?;
    read(&b, &i_val)
}

fn invoke_named_callback(
    rt: &mut Runtime<'_>,
    name: &str,
    args: &[Value],
) -> Result<Value, EvalError> {
    if let Some(f) = rt.builtins.get(name) {
        return f(args);
    }
    if let Some(hook) = rt.vm {
        if let Some(result) = (hook.call)(hook.vm, name, args) {
            return result;
        }
    }
    let module = rt.module;
    let Some(f) = module.functions.iter().find(|x| x.name == name) else {
        return Err(EvalError::TypeErr(format!("kazi haijulikani: {name}")));
    };
    rt.env.push_scope();
    for (i, p) in f.params.iter().enumerate() {
        rt.env
            .define(&p.name, args.get(i).cloned().unwrap_or(Value::Hamna));
    }
    let out = super::eval_block_impl(&f.body, rt);
    rt.env.pop_scope();
    match out {
        Ok(EvalOut::Return(v)) => Ok(v),
        Ok(_) => Ok(Value::Tupu),
        Err(e) => Err(e),
    }
}

/// Whether `v` matches `pat`, binding the pattern's names in the tree-walker's current scope.
pub(crate) fn match_and_bind_pattern(pat: &Pattern, v: &Value, rt: &mut Runtime<'_>) -> bool {
    match_pattern(pat, v, &mut |name, value| {
        rt.env.define(name, value.clone())
    })
}

/// Whether `v` matches `pat`, calling `bind` for each name the pattern binds, in order (a later
/// binding of the same name wins). The single implementation of `linganisha` patterns: the
/// tree-walker binds into its scope, the VM into registers.
pub(crate) fn match_pattern(pat: &Pattern, v: &Value, bind: &mut dyn FnMut(&str, &Value)) -> bool {
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
            bind(name, v);
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
                if !match_pattern(subpat, fval, bind) {
                    return false;
                }
            }
            true
        }
        Pattern::Jozi(p1, p2) => {
            let Value::Jozi(a, b) = v else {
                return false;
            };
            match_pattern(p1, a, bind) && match_pattern(p2, b, bind)
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
                        ("Sawa", Some(dpat), Ok(dval)) => match_pattern(dpat, dval, bind),
                        ("Sawa", None, Ok(_)) => true,
                        ("Kosa", Some(dpat), Err(dval)) => match_pattern(dpat, dval, bind),
                        ("Kosa", None, Err(_)) => true,
                        _ => false,
                    };
                }
            }
            if enum_name == "Chaguo" {
                if let Value::Chaguo(opt) = v {
                    return match (variant_name.as_str(), data, opt) {
                        ("Kuna", Some(dpat), Some(dval)) => match_pattern(dpat, dval, bind),
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
                (Some(dpat), Some(dval)) => match_pattern(dpat, dval, bind),
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
        Expr::If {
            cond,
            then_expr,
            else_if,
            else_expr,
            ..
        } => {
            let matches = |value: &Value| matches!(value, Value::Ukweli(true));
            if matches(&super::eval_expr_impl(cond, rt)?) {
                super::eval_expr_impl(then_expr, rt)
            } else {
                for (branch_cond, branch_expr) in else_if {
                    if matches(&super::eval_expr_impl(branch_cond, rt)?) {
                        return super::eval_expr_impl(branch_expr, rt);
                    }
                }
                match else_expr {
                    Some(expr) => super::eval_expr_impl(expr, rt),
                    None => Ok(Value::Hamna),
                }
            }
        }
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
            Ok(Value::Enum(
                enum_name.clone(),
                variant_name.clone(),
                variant_data,
            ))
        }
        Expr::FieldAccess {
            receiver, field, ..
        } => {
            let recv = super::eval_expr_impl(receiver, rt)?;
            super::methods::field_of(&recv, field)
        }
        Expr::Index { base, index, .. } => eval_index(base, index, rt, false),
        Expr::Unary { op, expr, .. } => {
            let v = match (op, &**expr) {
                // `jaribu b[i]`: unwrap the index's `Tokeo`, not a plain element.
                (UnaryOp::Jaribu, Expr::Index { base, index, .. }) => {
                    eval_index(base, index, rt, true)?
                }
                _ => super::eval_expr_impl(expr, rt)?,
            };
            super::ops::unary_value(op, v)
        }
        Expr::Binary {
            left, op, right, ..
        } => {
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
            super::ops::binary_value(op, &l, &r)
        }
        Expr::Cast { expr, ty, .. } => {
            let v = super::eval_expr_impl(expr, rt)?;
            super::methods::cast_value(v, &ty.name)
        }
        Expr::Call { callee, args, .. } => {
            rt.count_function_call();
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
                // mkondo_tumikia needs the current Module for the same reason tenda does — its
                // worker threads look up and invoke a named kazi per accepted connection.
                if name == "mkondo_tumikia" {
                    return crate::builtins::mkondo::mkondo_tumikia(rt.module, &args_val);
                }
                // mkondo_tumikia_http: the HTTP/1.1-framed counterpart, same Module-access
                // reason. See core/evaluator/src/builtins/http.rs.
                if name == "mkondo_tumikia_http" {
                    return crate::builtins::http::mkondo_tumikia_http(rt.module, &args_val);
                }
                if let Some(f) = rt.builtins.get(name) {
                    return f(&args_val);
                }
                // Mixed mode: a `kazi` the VM runs goes back to it (and its native code).
                if let Some(hook) = rt.vm {
                    if let Some(result) = (hook.call)(hook.vm, name, &args_val) {
                        return result;
                    }
                }
                // Borrow through a copy of the module reference (not `rt`), so `rt` stays free
                // for the body — no clone of the function's AST per call.
                let module = rt.module;
                if let Some(f) = module.functions.iter().find(|x| x.name == *name) {
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
            Err(EvalError::TypeErr(
                "kitu kinachoweza kuitwa kinahitajika".into(),
            ))
        }
        Expr::MethodCall {
            receiver,
            method_name,
            args,
            ..
        } => {
            rt.count_method_call();
            let receiver_name = match &**receiver {
                Expr::Ident { name, .. } => Some(name.as_str()),
                _ => None,
            };
            let mutating_method = matches!(
                method_name.as_str(),
                "ongeza" | "ingiza" | "weka_key" | "ondoa" | "badilisha"
            );
            if args.is_empty() {
                if let Some(name) = receiver_name {
                    if let Some(value) = rt.env.get_ref(name) {
                        match (value, method_name.as_str()) {
                            (Value::Orodha(values), "urefu") => {
                                return Ok(Value::Namba(values.len() as f64));
                            }
                            (Value::Kamusi(map), "idadi") => {
                                return Ok(Value::Namba(map.len() as f64));
                            }
                            (Value::Seti(set), "urefu") => {
                                return Ok(Value::Namba(set.len() as f64));
                            }
                            _ => {}
                        }
                    }
                }
            }
            if let (Some(name), true) = (receiver_name, mutating_method) {
                if rt
                    .env
                    .get_ref(name)
                    .is_some_and(|v| super::methods::is_mutating(v, method_name))
                {
                    let args_val: Vec<Value> = args
                        .iter()
                        .map(|a| super::eval_expr_impl(a, rt))
                        .collect::<Result<_, _>>()?;
                    if let Some(target) = rt.env.get_mut(name) {
                        return super::methods::mutate(target, method_name, &args_val);
                    }
                }
            }
            let recv = super::eval_expr_impl(receiver, rt)?;
            let args_val: Vec<Value> = args
                .iter()
                .map(|a| super::eval_expr_impl(a, rt))
                .collect::<Result<_, _>>()?;

            match (&recv, method_name.as_str()) {
                _ if super::methods::is_pure_method(&recv, method_name) => {
                    super::methods::pure_method(&recv, method_name, &args_val)
                }
                // A mutating method on a temporary receiver (not a named local).
                _ if super::methods::is_mutating(&recv, method_name) => {
                    super::methods::mutate_temporary(recv.clone(), method_name, &args_val)
                }
                _ if super::methods::is_callback_method(&recv, method_name) => {
                    super::methods::callback_method(
                        &recv,
                        method_name,
                        &args_val,
                        &mut |name, a| invoke_named_callback(rt, name, a),
                    )
                }
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
            let v = match &**expr {
                // `b[i]?`: an out-of-range index becomes the propagated `Tokeo` error.
                Expr::Index { base, index, .. } => eval_index(base, index, rt, true)?,
                _ => super::eval_expr_impl(expr, rt)?,
            };
            super::ops::propagate(v)
        }
    }
}
