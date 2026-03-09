//! Runtime (kitekelezi): toleo, jina_os.

use std::collections::HashMap;

use crate::value::Value;
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("toleo".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Neno(
            option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0").into(),
        ))
    }));
    m.insert("jina_os".to_string(), Box::new(|_args: &[Value]| {
        Ok(Value::Neno(std::env::consts::OS.into()))
    }));
}
