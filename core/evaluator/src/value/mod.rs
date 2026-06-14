//! Value model, errors, control flow, and numeric helpers.

mod numeric;

use std::collections::HashMap;

pub(crate) use numeric::{
    args_f64_2, arg_f64, assign_f64_op, as_char, as_f64, as_string, as_u64, binary_cmp_neno,
    binary_f64, binary_f64_cmp, handle_loop_out, parse_number,
};

/// Hashable key for Kamusi. Only Neno, Namba, Ukweli, Herufi are allowed as map keys.
/// Namba uses f64::to_bits() for canonical hashing (NaN is supported).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapKey {
    Neno(String),
    Namba(u64),
    Ukweli(bool),
    Herufi(char),
}

impl MapKey {
    pub fn to_value(&self) -> Value {
        match self {
            MapKey::Neno(s) => Value::Neno(s.clone()),
            MapKey::Namba(b) => Value::Namba(f64::from_bits(*b)),
            MapKey::Ukweli(b) => Value::Ukweli(*b),
            MapKey::Herufi(c) => Value::Herufi(*c),
        }
    }

    pub fn try_from_value(v: &Value) -> Result<MapKey, EvalError> {
        match v {
            Value::Neno(s) => Ok(MapKey::Neno(s.clone())),
            Value::Namba(n) => Ok(MapKey::Namba(n.to_bits())),
            Value::Ukweli(b) => Ok(MapKey::Ukweli(*b)),
            Value::Herufi(c) => Ok(MapKey::Herufi(*c)),
            _ => Err(EvalError::TypeErr(
                "kamusi: ufunguo lazima uwe Neno, Namba, Ukweli au Herufi".into(),
            )),
        }
    }
}

// TODO(Phase II): Missing value variants from the spec:
//   - Seti(HashSet<MapKey>) — ordered set type (Seti<T>)
//   - Mfululizo(&[Value]) — slice/view into an Orodha without cloning (requires lifetime or Rc)
//   - NambaKuu(BigInt) — arbitrary-precision integer (Namba_Kuu); needs the `num-bigint` crate
//   - NambaSahihi(BigDecimal) — arbitrary-precision decimal (Namba_Sahihi)
//   - FixedInt(i64, IntWidth) — Biti8/Biti32/Biti64/uBiti8/uBiti32/uBiti64 distinct from Namba
// Adding these requires updating all match arms in eval/expr.rs and the bytecode VM.

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Namba(f64),
    Neno(String),
    Ukweli(bool),
    Tupu,
    Hamna,
    Chaguo(Option<Box<Value>>),
    Tokeo(Result<Box<Value>, Box<Value>>),
    Orodha(Vec<Value>),
    Struct(String, Vec<(String, Value)>),
    Enum(String, String, Option<Box<Value>>), // enum_name, variant_name, optional_data
    Herufi(char),
    Jozi(Box<Value>, Box<Value>),
    Kamusi(HashMap<MapKey, Value>),
    /// Time: seconds since Unix epoch (majira module).
    Wakati(f64),
    /// Raw memory address (syscall, kiungo).
    Anuani(u64),
}

#[derive(Debug)]
pub enum EvalError {
    Panic(String),
    UndefinedVar(String),
    TypeErr(String),
    DivByZero,
    /// Propagate: ? on Tokeo(Err) — return this value from the current function.
    Propagate(Value),
    Unknown(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::Panic(m) => write!(f, "paparika: {m}"),
            EvalError::UndefinedVar(n) => write!(f, "jina '{n}' halijulikani"),
            EvalError::TypeErr(m) => write!(f, "aina: {m}"),
            EvalError::DivByZero => write!(f, "gawio kwa sifuri"),
            EvalError::Propagate(v) => write!(f, "KOSA: {v:?}"),
            EvalError::Unknown(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for EvalError {}

#[derive(Debug)]
pub enum EvalOut {
    Return(Value),
    Break(Option<String>),
    Continue(Option<String>),
    Next,
}

#[derive(Debug)]
pub(crate) enum LoopAction {
    Continue,
    Break,
    Propagate(EvalOut),
}
