//! `Value` <-> JSON codec. See `docs/design/json-codec-design.md` for the exclusion-set
//! rationale (deliberately stricter than, and independent from, `SendValue`/`try_into_send` —
//! see that type's own doc comment in `value/mod.rs`).

use std::collections::HashMap;

use super::{ErrorKind, EvalError, MapKey, Value};

/// Recursion depth cap for `to_json`/`from_json`. Guards against a stack overflow on
/// adversarial/malformed input once a future HTTP layer accepts JSON request bodies from an
/// untrusted network peer — this codec is written for that eventual caller, not only for
/// trusted in-program use today.
const MAX_DEPTH: usize = 64;

impl Value {
    /// Converts to a `serde_json::Value`. Fails (rather than silently dropping data) on any
    /// variant with no JSON representation, or once nesting exceeds `MAX_DEPTH`.
    pub fn to_json(&self) -> Result<serde_json::Value, EvalError> {
        to_json_depth(self, 0)
    }

    /// Converts a `serde_json::Value` into an Asili `Value`. Infallible: every JSON value has a
    /// natural `Value` representation (JSON numbers become `Namba`, since JSON itself has no
    /// arbitrary-precision or fixed-width integer type to route to `NambaKuu`/`Biti*` instead).
    pub fn from_json(j: &serde_json::Value) -> Value {
        from_json_depth(j, 0)
    }
}

fn to_json_depth(v: &Value, depth: usize) -> Result<serde_json::Value, EvalError> {
    if depth > MAX_DEPTH {
        return Err(EvalError::Coded {
            kind: ErrorKind::BadInput,
            message: format!("kwa_json: muundo umepitiliza kina cha juu ({MAX_DEPTH})"),
        });
    }
    use serde_json::Value as J;
    Ok(match v {
        Value::Namba(n) => serde_json::Number::from_f64(*n).map(J::Number).unwrap_or(J::Null),
        Value::Neno(s) => J::String(s.clone()),
        Value::Ukweli(b) => J::Bool(*b),
        Value::Tupu | Value::Hamna => J::Null,
        Value::Herufi(c) => J::String(c.to_string()),
        // Arbitrary precision doesn't round-trip through f64/JSON number — encode as a decimal
        // string, matching this codebase's existing NambaKuu/NambaSahihi -> Neno cast
        // convention (see eval/expr.rs's `kama Neno` cast arms).
        Value::NambaKuu(n) => J::String(n.to_string()),
        Value::NambaSahihi(n) => J::String(n.to_string()),
        // A raw memory address has no safe reason to be a JSON number a client could do
        // arithmetic on — encode as a string, deliberately.
        Value::Anuani(a) => J::String(a.to_string()),
        Value::Wakati(secs) => serde_json::Number::from_f64(*secs).map(J::Number).unwrap_or(J::Null),
        Value::Chaguo(opt) => match opt {
            Some(inner) => to_json_depth(inner, depth + 1)?,
            None => J::Null,
        },
        Value::Tokeo(res) => {
            let (tag, inner) = match res {
                Ok(v) => ("Sawa", v),
                Err(v) => ("Kosa", v),
            };
            let mut m = serde_json::Map::new();
            m.insert(tag.to_string(), to_json_depth(inner, depth + 1)?);
            J::Object(m)
        }
        Value::Orodha(items) => J::Array(
            items
                .iter()
                .map(|item| to_json_depth(item, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        Value::Jozi(a, b) => J::Array(vec![to_json_depth(a, depth + 1)?, to_json_depth(b, depth + 1)?]),
        Value::Kamusi(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                out.insert(map_key_to_json_field(k), to_json_depth(v, depth + 1)?);
            }
            J::Object(out)
        }
        Value::Seti(set) => J::Array(set.iter().map(|k| k.to_value()).map(|v| v.to_json()).collect::<Result<_, _>>()?),
        // Struct(name, fields) is already a name + field-list pair, reflection-friendly with no
        // per-type code needed — v1 does not support field rename/omission (wire format uses
        // internal field names verbatim); see the design doc for the future `#[jina("...")]`
        // extension point if that's ever needed.
        Value::Struct(_name, fields) => {
            let mut out = serde_json::Map::with_capacity(fields.len());
            for (fname, v) in fields {
                out.insert(fname.clone(), to_json_depth(v, depth + 1)?);
            }
            J::Object(out)
        }
        Value::Enum(_enum_name, variant, data) => {
            let mut out = serde_json::Map::new();
            out.insert("aina".to_string(), J::String(variant.clone()));
            if let Some(d) = data {
                out.insert("data".to_string(), to_json_depth(d, depth + 1)?);
            }
            J::Object(out)
        }
        #[cfg(not(target_arch = "wasm32"))]
        Value::TlsUsanidi(_) => {
            return Err(EvalError::Coded {
                kind: ErrorKind::BadInput,
                message: format!("kwa_json: aina '{}' haiwezi kubadilishwa kuwa JSON", variant_name(v)),
            });
        }
        Value::KashaGC(_)
        | Value::KashaGCDhaifu(_)
        | Value::Faili(_)
        | Value::Mkondo(_)
        | Value::MkondoSikilizaji(_)
        | Value::Kumbukumbu(_) => {
            return Err(EvalError::Coded {
                kind: ErrorKind::BadInput,
                message: format!("kwa_json: aina '{}' haiwezi kubadilishwa kuwa JSON", variant_name(v)),
            });
        }
        Value::NjiaTx(_)
        | Value::NjiaRx(_)
        | Value::NjiaTxBounded(_)
        | Value::NjiaRxBounded(_)
        | Value::Fungo(_) => {
            return Err(EvalError::Coded {
                kind: ErrorKind::BadInput,
                message: format!("kwa_json: aina '{}' haiwezi kubadilishwa kuwa JSON", variant_name(v)),
            });
        }
    })
}

/// `Kamusi` keys are `MapKey` (Neno/Namba/Ukweli/Herufi), but a JSON object's keys are always
/// strings — this stringifies non-Neno keys rather than rejecting them, since a `Kamusi<Namba,
/// _>` is common enough to be worth supporting on the wire (as `{"1": ...}`-shaped output),
/// matching how JSON.stringify(Map) and most JSON-object-from-map codecs behave elsewhere.
fn map_key_to_json_field(k: &MapKey) -> String {
    match k {
        MapKey::Neno(s) => s.clone(),
        MapKey::Namba(bits) => f64::from_bits(*bits).to_string(),
        MapKey::Ukweli(b) => b.to_string(),
        MapKey::Herufi(c) => c.to_string(),
    }
}

fn variant_name(v: &Value) -> &'static str {
    match v {
        Value::KashaGC(_) => "Kasha_GC",
        Value::KashaGCDhaifu(_) => "Kasha_GC_Dhaifu",
        Value::MkondoSikilizaji(_) => "MkondoSikilizaji",
        #[cfg(not(target_arch = "wasm32"))]
        Value::TlsUsanidi(_) => "TlsUsanidi",
        Value::Faili(_) => "Faili",
        Value::Mkondo(_) => "Mkondo",
        Value::Kumbukumbu(_) => "Kumbukumbu",
        Value::NjiaTx(_) => "NjiaTx",
        Value::NjiaRx(_) => "NjiaRx",
        Value::NjiaTxBounded(_) => "NjiaTxBounded",
        Value::NjiaRxBounded(_) => "NjiaRxBounded",
        Value::Fungo(_) => "Fungo",
        _ => "?",
    }
}

fn from_json_depth(j: &serde_json::Value, depth: usize) -> Value {
    if depth > MAX_DEPTH {
        return Value::Hamna;
    }
    use serde_json::Value as J;
    match j {
        J::Null => Value::Hamna,
        J::Bool(b) => Value::Ukweli(*b),
        J::Number(n) => Value::Namba(n.as_f64().unwrap_or(f64::NAN)),
        J::String(s) => Value::Neno(s.clone()),
        J::Array(items) => Value::Orodha(items.iter().map(|v| from_json_depth(v, depth + 1)).collect()),
        J::Object(map) => {
            let mut fields: Vec<(String, Value)> = Vec::with_capacity(map.len());
            let mut out: HashMap<MapKey, Value> = HashMap::with_capacity(map.len());
            for (k, v) in map {
                let value = from_json_depth(v, depth + 1);
                fields.push((k.clone(), value.clone()));
                out.insert(MapKey::Neno(k.clone()), value);
            }
            // A JSON object has no distinction between "this was an Asili Struct/Tokeo/Enum"
            // and "this was a Kamusi" — decode as Kamusi<Neno, _>, the structurally accurate
            // JSON-native shape; a caller that needs a specific Struct can construct one from
            // the decoded Kamusi's fields. `fields` is unused for the Kamusi case but kept
            // available for a future rename/typed-decode extension point rather than discarded.
            let _ = fields;
            Value::Kamusi(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;

    fn round_trip(v: &Value) -> Value {
        let j = v.to_json().expect("to_json");
        Value::from_json(&j)
    }

    #[test]
    fn scalars_round_trip() {
        assert_eq!(round_trip(&Value::Namba(42.5)), Value::Namba(42.5));
        assert_eq!(round_trip(&Value::Neno("habari".into())), Value::Neno("habari".into()));
        assert_eq!(round_trip(&Value::Ukweli(true)), Value::Ukweli(true));
        assert_eq!(Value::Tupu.to_json().unwrap(), serde_json::Value::Null);
        assert_eq!(Value::Hamna.to_json().unwrap(), serde_json::Value::Null);
    }

    #[test]
    fn herufi_encodes_as_single_char_string() {
        let j = Value::Herufi('A').to_json().unwrap();
        assert_eq!(j, serde_json::json!("A"));
    }

    #[test]
    fn namba_kuu_encodes_as_decimal_string() {
        let n = Value::NambaKuu(num_bigint::BigInt::from(123456789012345678_i64));
        let j = n.to_json().unwrap();
        assert_eq!(j, serde_json::json!("123456789012345678"));
    }

    #[test]
    fn namba_sahihi_encodes_as_decimal_string() {
        use std::str::FromStr;
        let n = Value::NambaSahihi(bigdecimal::BigDecimal::from_str("3.1415926535").unwrap());
        let j = n.to_json().unwrap();
        assert_eq!(j, serde_json::json!("3.1415926535"));
    }

    #[test]
    fn anuani_encodes_as_string_not_number() {
        let j = Value::Anuani(0xdeadbeef).to_json().unwrap();
        assert_eq!(j, serde_json::json!("3735928559"));
    }

    #[test]
    fn orodha_round_trips() {
        let v = Value::Orodha(vec![Value::Namba(1.0), Value::Namba(2.0), Value::Neno("x".into())]);
        let j = v.to_json().unwrap();
        assert_eq!(j, serde_json::json!([1.0, 2.0, "x"]));
    }

    #[test]
    fn chaguo_some_unwraps_none_becomes_null() {
        assert_eq!(Value::Chaguo(Some(Box::new(Value::Namba(5.0)))).to_json().unwrap(), serde_json::json!(5.0));
        assert_eq!(Value::Chaguo(None).to_json().unwrap(), serde_json::Value::Null);
    }

    #[test]
    fn tokeo_encodes_tagged() {
        let ok = Value::Tokeo(Ok(Box::new(Value::Namba(1.0))));
        assert_eq!(ok.to_json().unwrap(), serde_json::json!({"Sawa": 1.0}));
        let err = Value::Tokeo(Err(Box::new(Value::Neno("kosa".into()))));
        assert_eq!(err.to_json().unwrap(), serde_json::json!({"Kosa": "kosa"}));
    }

    #[test]
    fn struct_encodes_as_flat_object_reflectively() {
        let v = Value::Struct(
            "Pika".to_string(),
            vec![("x".to_string(), Value::Namba(1.0)), ("y".to_string(), Value::Neno("hi".into()))],
        );
        let j = v.to_json().unwrap();
        assert_eq!(j, serde_json::json!({"x": 1.0, "y": "hi"}));
    }

    #[test]
    fn enum_encodes_with_tag_and_optional_data() {
        let unit = Value::Enum("Rangi".into(), "Nyekundu".into(), None);
        assert_eq!(unit.to_json().unwrap(), serde_json::json!({"aina": "Nyekundu"}));
        let with_data = Value::Enum("Chaguo".into(), "Kuna".into(), Some(Box::new(Value::Namba(7.0))));
        assert_eq!(with_data.to_json().unwrap(), serde_json::json!({"aina": "Kuna", "data": 7.0}));
    }

    #[test]
    fn kasha_gc_rejected() {
        let v = Value::KashaGC(Rc::new(RefCell::new(Value::Namba(1.0))));
        assert!(v.to_json().is_err());
    }

    #[test]
    fn njia_and_fungo_rejected_even_though_send_safe() {
        // These pass `Value::try_into_send` (they ARE Send-safe, for tenda's benefit) but have
        // no JSON representation — the JSON codec's exclusion set is intentionally stricter
        // than SendValue's, not a reuse of it. See the module doc comment.
        let (tx, rx) = std::sync::mpsc::channel::<super::super::SendValue>();
        let tx_val = Value::NjiaTx(std::sync::Arc::new(std::sync::Mutex::new(tx)));
        let rx_val = Value::NjiaRx(std::sync::Arc::new(std::sync::Mutex::new(rx)));
        assert!(tx_val.try_into_send().is_some(), "sanity: NjiaTx is Send-safe");
        assert!(tx_val.to_json().is_err());
        assert!(rx_val.to_json().is_err());
    }

    #[test]
    fn depth_limit_fails_cleanly_instead_of_overflowing_stack() {
        let mut v = Value::Namba(0.0);
        for _ in 0..(MAX_DEPTH + 10) {
            v = Value::Orodha(vec![v]);
        }
        assert!(v.to_json().is_err(), "must fail cleanly past MAX_DEPTH, not stack-overflow");
    }

    #[test]
    fn kamusi_round_trips_through_neno_keys() {
        let mut map = HashMap::new();
        map.insert(MapKey::Neno("a".into()), Value::Namba(1.0));
        let v = Value::Kamusi(map);
        let j = v.to_json().unwrap();
        assert_eq!(j, serde_json::json!({"a": 1.0}));
    }

    #[test]
    fn from_json_object_decodes_as_kamusi() {
        let j = serde_json::json!({"a": 1, "b": "x"});
        let v = Value::from_json(&j);
        match v {
            Value::Kamusi(m) => {
                assert_eq!(m.get(&MapKey::Neno("a".into())), Some(&Value::Namba(1.0)));
                assert_eq!(m.get(&MapKey::Neno("b".into())), Some(&Value::Neno("x".into())));
            }
            other => panic!("expected Kamusi, got {other:?}"),
        }
    }
}
