//! Operator semantics shared by the tree-walking evaluator and native code's host.
//!
//! Both engines evaluate operands their own way (native code compiles typed
//! registers itself), but every operation on generic `Value`s goes through these functions so the two
//! can never disagree.

use asili_parser::{BinaryOp, UnaryOp};
use std::cmp::Ordering;

use crate::value::{
    self, big_numeric_binary_op, binary_cmp_neno, binary_f64, binary_f64_cmp, EvalError, Value,
};

/// `l op r` for already-evaluated operands. `na`/`au` short-circuiting is the caller's job
/// (it decides whether `r` is evaluated at all); given both values, they are checked here.
pub(crate) fn binary_value(op: &BinaryOp, l: &Value, r: &Value) -> Result<Value, EvalError> {
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
        BinaryOp::Add => match (l, r) {
            // One allocation of exactly the result's size; neither operand is copied first.
            (Value::Neno(s1), Value::Neno(s2)) => Ok(Value::Neno(value::concat_text(s1, s2))),
            (l, r) => binary_f64(l, r, "+", |a, b| a + b),
        },
        BinaryOp::Sub => binary_f64(&l, &r, "-", |a, b| a - b),
        BinaryOp::Mul => binary_f64(&l, &r, "*", |a, b| a * b),
        BinaryOp::Div => {
            let a =
                value::as_f64(&l).ok_or_else(|| EvalError::TypeErr("/ inahitaji Namba".into()))?;
            let b =
                value::as_f64(&r).ok_or_else(|| EvalError::TypeErr("/ inahitaji Namba".into()))?;
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
                .ok_or_else(|| EvalError::TypeErr("na_biti inahitaji Namba".into()))?
                as i64;
            let b = value::as_f64(&r)
                .ok_or_else(|| EvalError::TypeErr("na_biti inahitaji Namba".into()))?
                as i64;
            Ok(Value::Namba((a & b) as f64))
        }
        BinaryOp::BitOr => {
            let a = value::as_f64(&l)
                .ok_or_else(|| EvalError::TypeErr("au_biti inahitaji Namba".into()))?
                as i64;
            let b = value::as_f64(&r)
                .ok_or_else(|| EvalError::TypeErr("au_biti inahitaji Namba".into()))?
                as i64;
            Ok(Value::Namba((a | b) as f64))
        }
        BinaryOp::BitXor => {
            let a = value::as_f64(&l)
                .ok_or_else(|| EvalError::TypeErr("xor_biti inahitaji Namba".into()))?
                as i64;
            let b = value::as_f64(&r)
                .ok_or_else(|| EvalError::TypeErr("xor_biti inahitaji Namba".into()))?
                as i64;
            Ok(Value::Namba((a ^ b) as f64))
        }
        BinaryOp::Shl => {
            let a = value::as_f64(&l)
                .ok_or_else(|| EvalError::TypeErr("sogeza_kushoto inahitaji Namba".into()))?
                as i64;
            let b = value::as_f64(&r)
                .ok_or_else(|| EvalError::TypeErr("sogeza_kushoto inahitaji Namba".into()))?;
            let shift = b as i32;
            let shift = if !(0..=63).contains(&shift) {
                0
            } else {
                shift as u32
            };
            Ok(Value::Namba((a.wrapping_shl(shift)) as f64))
        }
        BinaryOp::Shr => {
            let a = value::as_f64(&l)
                .ok_or_else(|| EvalError::TypeErr("sogeza_kulia inahitaji Namba".into()))?
                as i64;
            let b = value::as_f64(&r)
                .ok_or_else(|| EvalError::TypeErr("sogeza_kulia inahitaji Namba".into()))?;
            let shift = b as i32;
            let shift = if !(0..=63).contains(&shift) {
                0
            } else {
                shift as u32
            };
            Ok(Value::Namba((a.wrapping_shr(shift)) as f64))
        }
    }
}

/// `op v` for the value-level unary operators (`jaribu` is [`jaribu`]).
pub(crate) fn unary_value(op: &UnaryOp, v: Value) -> Result<Value, EvalError> {
    match op {
        UnaryOp::Neg => {
            let n =
                value::as_f64(&v).ok_or_else(|| EvalError::TypeErr("- inahitaji Namba".into()))?;
            Ok(Value::Namba(-n))
        }
        UnaryOp::Not => {
            let b = match &v {
                Value::Ukweli(x) => *x,
                _ => return Err(EvalError::TypeErr("siyo inahitaji Ukweli".into())),
            };
            Ok(Value::Ukweli(!b))
        }
        UnaryOp::Jaribu => jaribu(&v),
        UnaryOp::BitNot => {
            let n = value::as_f64(&v)
                .ok_or_else(|| EvalError::TypeErr("siyo_biti inahitaji Namba".into()))?;
            let bits = n as i64;
            Ok(Value::Namba(!bits as f64))
        }
        UnaryOp::BorrowImm | UnaryOp::BorrowMut => Ok(v),
    }
}

/// `jaribu v`: unwrap `Tokeo`/`Chaguo`; an error or `Hamna` aborts execution.
pub(crate) fn jaribu(v: &Value) -> Result<Value, EvalError> {
    match v {
        Value::Tokeo(Ok(inner)) => Ok((**inner).clone()),
        Value::Tokeo(Err(e)) => Err(EvalError::Unknown(format!("KOSA: {e:?}"))),
        Value::Chaguo(Some(inner)) => Ok((**inner).clone()),
        Value::Chaguo(None) => Err(EvalError::Unknown("Chaguo: Hamna".into())),
        _ => Err(EvalError::TypeErr("jaribu inahitaji Tokeo/Chaguo".into())),
    }
}

/// `v?`: unwrap `Tokeo`/`Chaguo`; a `Tokeo` error returns it from the enclosing `kazi`
/// (`EvalError::Propagate`, which the function-call boundary turns into its return value).
pub(crate) fn propagate(v: Value) -> Result<Value, EvalError> {
    match v {
        Value::Tokeo(Ok(inner)) => Ok(*inner),
        Value::Tokeo(Err(e)) => Err(EvalError::Propagate(Value::Tokeo(Err(e)))),
        Value::Chaguo(Some(inner)) => Ok(*inner),
        Value::Chaguo(None) => Err(EvalError::Unknown("? Chaguo Hamna".into())),
        _ => Err(EvalError::TypeErr("? inahitaji Tokeo/Chaguo".into())),
    }
}

/// Condition truthiness for `ikiwa`/`wakati`: only `kweli` is true.
pub(crate) fn truthy(v: &Value) -> bool {
    matches!(v, Value::Ukweli(true))
}
