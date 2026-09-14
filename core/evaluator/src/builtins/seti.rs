//! Seti<T> (set): seti() constructor. Always in scope via msingi, like Orodha/Kamusi.
//! Instance methods (.ongeza/.ina/.ondoa/.urefu/.clona) are dispatched in eval/expr.rs, not
//! here — this module only registers the free-function constructor.

use std::collections::{HashMap, HashSet};

use crate::value::{MapKey, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("seti".to_string(), Box::new(|args: &[Value]| {
        let mut set = HashSet::new();
        for v in args {
            set.insert(MapKey::try_from_value(v)?);
        }
        Ok(Value::Seti(set))
    }));
    m.insert("seti_tupu".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Seti(HashSet::new()))
    }));
}
