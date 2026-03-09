//! Sambamba (concurrency): anza_mwendo, subiri_mwendo. Stub; full impl would need Send runtime.

use std::collections::HashMap;

use crate::value::{self, Value};
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("anza_mwendo".to_string(), Box::new(|args: &[Value]| {
        let _kazi = value::as_string(args.get(0).unwrap_or(&Value::Hamna)).unwrap_or_default();
        let _hoja = args.get(1).cloned();
        let _ = (_kazi, _hoja);
        Ok(Value::Namba(0.0))
    }));
    m.insert("subiri_mwendo".to_string(), Box::new(|args: &[Value]| {
        let _id = value::as_f64(args.first().unwrap_or(&Value::Hamna)).unwrap_or(0.0);
        let _ = _id;
        Ok(Value::Tokeo(Ok(Box::new(Value::Tupu))))
    }));
}
