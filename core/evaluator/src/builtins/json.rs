//! JSON codec builtins: kwa_json (Value -> Neno), kutoka_json (Neno -> Value). See
//! docs/design/json-codec-design.md and value/json.rs for the conversion rules.

use std::collections::HashMap;

use super::BuiltinFn;
use crate::value::Value;

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    m.insert(
        "kwa_json".to_string(),
        Box::new(|args: &[Value]| {
            let thamani = args.first().unwrap_or(&Value::Hamna);
            match thamani.to_json() {
                Ok(j) => Ok(Value::sawa(Value::neno(j.to_string()))),
                Err(e) => Ok(Value::kosa(e.to_string())),
            }
        }),
    );
    m.insert(
        "kutoka_json".to_string(),
        Box::new(|args: &[Value]| {
            let neno = super::arg_str(args, 0);
            match serde_json::from_str::<serde_json::Value>(&neno) {
                Ok(j) => Ok(Value::sawa(Value::from_json(&j))),
                Err(e) => Ok(Value::kosa(format!("kutoka_json: JSON batili: {e}"))),
            }
        }),
    );
}
