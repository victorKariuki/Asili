//! Numeric and loop helpers for Value.

use std::cmp::Ordering;

use super::{EvalError, EvalOut, LoopAction, Value};

pub(crate) fn handle_loop_out(my_label: Option<&String>, out: EvalOut) -> LoopAction {
    match out {
        EvalOut::Return(v) => LoopAction::Propagate(EvalOut::Return(v)),
        EvalOut::Break(None) => LoopAction::Break,
        EvalOut::Break(Some(ref l)) => {
            if my_label == Some(l) {
                LoopAction::Break
            } else {
                LoopAction::Propagate(EvalOut::Break(Some(l.clone())))
            }
        }
        EvalOut::Continue(None) => LoopAction::Continue,
        EvalOut::Continue(Some(ref l)) => {
            if my_label != Some(l) {
                LoopAction::Propagate(EvalOut::Continue(Some(l.clone())))
            } else {
                LoopAction::Continue
            }
        }
        EvalOut::Next => LoopAction::Continue,
    }
}

pub(crate) fn parse_number(s: &str) -> f64 {
    s.trim().parse().unwrap_or(0.0)
}

pub(crate) fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Namba(n) => Some(*n),
        Value::Wakati(n) => Some(*n),
        Value::Anuani(n) => Some(*n as f64),
        _ => None,
    }
}

pub(crate) fn as_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Anuani(n) => Some(*n),
        Value::Namba(n) => Some(*n as u64),
        _ => None,
    }
}

pub(crate) fn as_string(v: &Value) -> Option<String> {
    match v {
        Value::Neno(s) => Some(s.clone()),
        _ => None,
    }
}

pub(crate) fn as_char(v: &Value) -> Option<char> {
    match v {
        Value::Herufi(c) => Some(*c),
        _ => None,
    }
}

pub(crate) fn arg_f64(args: &[Value], idx: usize, fn_name: &str) -> Result<f64, EvalError> {
    args.get(idx)
        .and_then(as_f64)
        .ok_or_else(|| EvalError::TypeErr(format!("{fn_name} inahitaji Namba")))
}

pub(crate) fn args_f64_2(args: &[Value], fn_name: &str) -> Result<(f64, f64), EvalError> {
    let a = arg_f64(args, 0, fn_name)?;
    let b = arg_f64(args, 1, fn_name)?;
    Ok((a, b))
}

pub(crate) fn binary_f64(
    l: &Value,
    r: &Value,
    op_name: &str,
    f: impl Fn(f64, f64) -> f64,
) -> Result<Value, EvalError> {
    let a = as_f64(l).ok_or_else(|| EvalError::TypeErr(format!("{op_name} inahitaji Namba")))?;
    let b = as_f64(r).ok_or_else(|| EvalError::TypeErr(format!("{op_name} inahitaji Namba")))?;
    Ok(Value::Namba(f(a, b)))
}

pub(crate) fn binary_f64_cmp(
    l: &Value,
    r: &Value,
    op_name: &str,
    f: impl Fn(f64, f64) -> bool,
) -> Result<Value, EvalError> {
    let a = as_f64(l).ok_or_else(|| EvalError::TypeErr(format!("{op_name} inahitaji Namba")))?;
    let b = as_f64(r).ok_or_else(|| EvalError::TypeErr(format!("{op_name} inahitaji Namba")))?;
    Ok(Value::Ukweli(f(a, b)))
}

/// Lexicographic comparison for Neno. Returns Some(Ukweli) when both are strings, else None.
pub(crate) fn binary_cmp_neno(
    l: &Value,
    r: &Value,
    f: impl Fn(Ordering) -> bool,
) -> Option<Value> {
    let s1 = as_string(l)?;
    let s2 = as_string(r)?;
    Some(Value::Ukweli(f(s1.cmp(&s2))))
}

pub(crate) fn assign_f64_op(
    current: &Value,
    rhs: &Value,
    op_name: &str,
    f: impl Fn(f64, f64) -> f64,
) -> Result<Value, EvalError> {
    let a = as_f64(current).ok_or_else(|| EvalError::TypeErr(format!("{op_name} inahitaji Namba")))?;
    let b = as_f64(rhs).ok_or_else(|| EvalError::TypeErr(format!("{op_name} inahitaji Namba")))?;
    Ok(Value::Namba(f(a, b)))
}
