//! `Kasha_GC<T>` (managed memory): reference-counted shared wrapper. Requires `leta kasha_gc`.
//!
//! Sharing and refcounting need no special-casing beyond the `Value::KashaGC` variant itself:
//! `Value::clone()` on it is `Rc::clone` (cheap, shares the allocation — every read of a
//! `weka`-bound variable clones its `Value`, so `weka b = a` already shares correctly), and
//! `tupa`/scope-exit already drops the `Value` normally, which drops the `Rc` and decrements the
//! strong count via ordinary Rust ownership. See `Value::KashaGC` and its `PartialEq` impl
//! (identity via `Rc::ptr_eq`, not structural — comparing contents could panic on a borrow
//! conflict) in `crate::value`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::Value;
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("kasha_gc_unda".to_string(), Box::new(|args: &[Value]| {
        let inner = args.first().cloned().unwrap_or(Value::Hamna);
        Ok(Value::KashaGC(Rc::new(RefCell::new(inner))))
    }));
    // Downgrade: Kasha_GC<T> has no cycle collector, so a reference cycle through it leaks
    // permanently. kasha_gc_dhaifu() is the user-level escape hatch — hold a Dhaifu in the
    // back-pointer of a cycle-prone structure and .imarisha() (upgrade) only when actually
    // needed, so the cycle's strong count can still reach zero on the forward direction.
    m.insert("kasha_gc_dhaifu".to_string(), Box::new(|args: &[Value]| {
        match args.first() {
            Some(Value::KashaGC(cell)) => Ok(Value::KashaGCDhaifu(Rc::downgrade(cell))),
            _ => Err(crate::value::EvalError::TypeErr(
                "kasha_gc_dhaifu: hoja lazima iwe Kasha_GC<T>".to_string(),
            )),
        }
    }));
}
