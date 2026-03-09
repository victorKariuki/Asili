//! Kiungo (FFI bridge): load library, call symbol. Stub returns error.

use std::collections::HashMap;

use crate::value::{self, Value};
use super::BuiltinFn;

fn tokeo_err(msg: String) -> Value {
    Value::Tokeo(Err(Box::new(Value::Neno(msg))))
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("saza_kiungo".to_string(), Box::new(|args: &[Value]| {
        let _njia = value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        Ok(tokeo_err("saza_kiungo haijatengenezwa".into()))
    }));
    m.insert("wito_kiungo".to_string(), Box::new(|args: &[Value]| {
        let _anuani = value::as_u64(args.get(0).unwrap_or(&Value::Hamna)).unwrap_or(0);
        let _jina = value::as_string(args.get(1).unwrap_or(&Value::Hamna)).unwrap_or_default();
        Ok(tokeo_err("wito_kiungo haijatengenezwa".into()))
    }));
}
