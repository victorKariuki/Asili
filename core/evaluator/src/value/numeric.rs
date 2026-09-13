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

// TODO: parse_number silently returns 0.0 for any invalid numeric literal (e.g. "0x1F", "1_000",
// "1e3" with locale-specific separators, binary "0b1010"). Invalid literals should produce a lex
// error at tokenization time, not silently evaluate to zero at runtime.
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

/// Namba_Kuu/Namba_Sahihi arithmetic and comparison. A `Namba` operand widens infallibly to
/// match the other side's type (`Namba` -> `Namba_Kuu` truncates toward zero if the other side
/// is `Namba_Kuu`, since an integer type can't represent a fraction; `Namba` -> `Namba_Sahihi`
/// otherwise, via `BigDecimal::from_f64` so the exact IEEE-754 value round-trips rather than a
/// lossy decimal-string reformat). Two `Namba_Kuu` operands stay in `BigInt` (preserving exact
/// integer semantics for `/`/`%`); any `Namba_Sahihi` involved promotes both sides to decimal.
/// Returns `Ok(None)` (not an error) for a non-numeric operand or an op this helper doesn't
/// handle (bitwise ops fall through to the caller's existing error path for those).
pub(crate) fn big_numeric_binary_op(
    l: &Value,
    op: &asili_parser::BinaryOp,
    r: &Value,
) -> Result<Option<Value>, EvalError> {
    use asili_parser::BinaryOp;
    use super::{BigDecimal, BigInt};
    use num_traits::FromPrimitive;

    let is_big = |v: &Value| matches!(v, Value::NambaKuu(_) | Value::NambaSahihi(_));
    if !is_big(l) && !is_big(r) {
        return Ok(None);
    }

    let both_int = matches!(l, Value::NambaKuu(_) | Value::Namba(_))
        && matches!(r, Value::NambaKuu(_) | Value::Namba(_))
        && (matches!(l, Value::NambaKuu(_)) || matches!(r, Value::NambaKuu(_)));

    if both_int {
        let to_bigint = |v: &Value| -> BigInt {
            match v {
                Value::NambaKuu(n) => n.clone(),
                Value::Namba(n) => BigInt::from(*n as i64),
                _ => unreachable!("both_int guard checked above"),
            }
        };
        let ai = to_bigint(l);
        let bi = to_bigint(r);
        let zero = BigInt::from(0);
        let result = match op {
            BinaryOp::Add => Value::NambaKuu(&ai + &bi),
            BinaryOp::Sub => Value::NambaKuu(&ai - &bi),
            BinaryOp::Mul => Value::NambaKuu(&ai * &bi),
            BinaryOp::Div if bi == zero => return Err(EvalError::DivByZero),
            BinaryOp::Div => Value::NambaKuu(&ai / &bi),
            BinaryOp::Rem if bi == zero => return Err(EvalError::DivByZero),
            BinaryOp::Rem => Value::NambaKuu(&ai % &bi),
            BinaryOp::Pow => {
                let exp: u32 = bi.try_into().map_err(|_| {
                    EvalError::TypeErr("** ya Namba_Kuu inahitaji kipeo kisicho hasi".into())
                })?;
                Value::NambaKuu(ai.pow(exp))
            }
            BinaryOp::Eq => Value::Ukweli(ai == bi),
            BinaryOp::Ne => Value::Ukweli(ai != bi),
            BinaryOp::Gt => Value::Ukweli(ai > bi),
            BinaryOp::Lt => Value::Ukweli(ai < bi),
            BinaryOp::Ge => Value::Ukweli(ai >= bi),
            BinaryOp::Le => Value::Ukweli(ai <= bi),
            _ => return Ok(None),
        };
        return Ok(Some(result));
    }

    let to_bigdecimal = |v: &Value| -> Option<BigDecimal> {
        match v {
            Value::NambaSahihi(n) => Some(n.clone()),
            Value::NambaKuu(n) => Some(BigDecimal::from(n.clone())),
            Value::Namba(n) => BigDecimal::from_f64(*n),
            _ => None,
        }
    };
    let (Some(ad), Some(bd)) = (to_bigdecimal(l), to_bigdecimal(r)) else {
        return Ok(None);
    };
    let zero = BigDecimal::from(0);
    let result = match op {
        BinaryOp::Add => Value::NambaSahihi(&ad + &bd),
        BinaryOp::Sub => Value::NambaSahihi(&ad - &bd),
        BinaryOp::Mul => Value::NambaSahihi(&ad * &bd),
        BinaryOp::Div if bd == zero => return Err(EvalError::DivByZero),
        BinaryOp::Div => Value::NambaSahihi(ad / bd),
        BinaryOp::Rem if bd == zero => return Err(EvalError::DivByZero),
        BinaryOp::Rem => Value::NambaSahihi(&ad % &bd),
        BinaryOp::Pow => {
            // BigDecimal only supports integer exponents (exact decimal arithmetic has no
            // general fractional-power operation); the right operand's own integer value
            // (truncated) is used directly rather than round-tripping through the widened ad/bd.
            let exp = match r {
                Value::Namba(n) => *n as i64,
                Value::NambaKuu(n) => n.to_string().parse().unwrap_or(0),
                Value::NambaSahihi(n) => n.to_string().parse::<f64>().unwrap_or(0.0) as i64,
                _ => 0,
            };
            Value::NambaSahihi(ad.powi(exp))
        }
        BinaryOp::Eq => Value::Ukweli(ad == bd),
        BinaryOp::Ne => Value::Ukweli(ad != bd),
        BinaryOp::Gt => Value::Ukweli(ad > bd),
        BinaryOp::Lt => Value::Ukweli(ad < bd),
        BinaryOp::Ge => Value::Ukweli(ad >= bd),
        BinaryOp::Le => Value::Ukweli(ad <= bd),
        _ => return Ok(None),
    };
    Ok(Some(result))
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
