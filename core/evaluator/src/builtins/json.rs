//! JSON codec builtins: kwa_json (Value -> Neno), kutoka_json (Neno -> Value). See
//! docs/design/json-codec-design.md and value/json.rs for the conversion rules.

use std::collections::HashMap;

use crate::value::Value;
use super::BuiltinFn;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert("kwa_json".to_string(), Box::new(|args: &[Value]| {
        let thamani = args.first().unwrap_or(&Value::Hamna);
        match thamani.to_json() {
            Ok(j) => Ok(Value::Tokeo(Ok(Box::new(Value::Neno(j.to_string()))))),
            Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(e.to_string()))))),
        }
    }));
    m.insert("kutoka_json".to_string(), Box::new(|args: &[Value]| {
        let neno = crate::value::as_string(args.first().unwrap_or(&Value::Hamna)).unwrap_or_default();
        match serde_json::from_str::<serde_json::Value>(&neno) {
            Ok(j) => Ok(Value::Tokeo(Ok(Box::new(Value::from_json(&j))))),
            Err(e) => Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!(
                "kutoka_json: JSON batili: {e}"
            )))))),
        }
    }));
}
