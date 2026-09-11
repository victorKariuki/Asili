//! Value model, errors, control flow, and numeric helpers.

mod numeric;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

#[derive(Clone)]
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
    /// Reference-counted shared wrapper (opt-in `leta kasha_gc`). Cloning a `Value` clones the
    /// `Rc` handle (cheap, shares the allocation) — use `.shirikisha()`/`kasha_gc_shiriki` to be
    /// explicit about that at the Asili level; `Value::clone()` alone does not increment beyond
    /// what `Rc::clone` already does.
    KashaGC(Rc<RefCell<Value>>),
}

// Manual Debug impl (not #[derive]) so raw/REPL output uses Asili's own variant names —
// `Tokeo`/`Chaguo` wrap Rust's `Result`/`Option`, whose derived Debug would otherwise print
// the Rust-side `Ok(..)`/`Err(..)`/`Some(..)` literally instead of `Sawa(..)`/`Kosa(..)`/
// `Kuna(..)` (see `core/parser/src/parse.rs`'s `standard_enums()` for the canonical names).
impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Namba(n) => f.debug_tuple("Namba").field(n).finish(),
            Value::Neno(s) => f.debug_tuple("Neno").field(s).finish(),
            Value::Ukweli(b) => f.debug_tuple("Ukweli").field(b).finish(),
            Value::Tupu => write!(f, "Tupu"),
            Value::Hamna => write!(f, "Hamna"),
            Value::Chaguo(Some(v)) => write!(f, "Chaguo(Kuna({v:?}))"),
            Value::Chaguo(None) => write!(f, "Chaguo(Hamna)"),
            Value::Tokeo(Ok(v)) => write!(f, "Tokeo(Sawa({v:?}))"),
            Value::Tokeo(Err(e)) => write!(f, "Tokeo(Kosa({e:?}))"),
            Value::Orodha(items) => f.debug_tuple("Orodha").field(items).finish(),
            Value::Struct(name, fields) => f.debug_tuple("Struct").field(name).field(fields).finish(),
            Value::Enum(en, vn, data) => f.debug_tuple("Enum").field(en).field(vn).field(data).finish(),
            Value::Herufi(c) => f.debug_tuple("Herufi").field(c).finish(),
            Value::Jozi(a, b) => f.debug_tuple("Jozi").field(a).field(b).finish(),
            Value::Kamusi(m) => f.debug_tuple("Kamusi").field(m).finish(),
            Value::Wakati(s) => f.debug_tuple("Wakati").field(s).finish(),
            Value::Anuani(a) => f.debug_tuple("Anuani").field(a).finish(),
            Value::KashaGC(cell) => f.debug_tuple("KashaGC").field(cell).finish(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Namba(a), Value::Namba(b)) => a == b,
            (Value::Neno(a), Value::Neno(b)) => a == b,
            (Value::Ukweli(a), Value::Ukweli(b)) => a == b,
            (Value::Tupu, Value::Tupu) => true,
            (Value::Hamna, Value::Hamna) => true,
            (Value::Chaguo(a), Value::Chaguo(b)) => a == b,
            (Value::Tokeo(a), Value::Tokeo(b)) => a == b,
            (Value::Orodha(a), Value::Orodha(b)) => a == b,
            (Value::Struct(an, af), Value::Struct(bn, bf)) => an == bn && af == bf,
            (Value::Enum(an, av, ad), Value::Enum(bn, bv, bd)) => an == bn && av == bv && ad == bd,
            (Value::Herufi(a), Value::Herufi(b)) => a == b,
            (Value::Jozi(a1, a2), Value::Jozi(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Kamusi(a), Value::Kamusi(b)) => a == b,
            (Value::Wakati(a), Value::Wakati(b)) => a == b,
            (Value::Anuani(a), Value::Anuani(b)) => a == b,
            // Identity, not structural equality: comparing contents via RefCell::eq would panic
            // if either handle is currently mutably borrowed. Two handles are "equal" iff they
            // share the same allocation (the same underlying `weka` binding's shared cell).
            (Value::KashaGC(a), Value::KashaGC(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
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
