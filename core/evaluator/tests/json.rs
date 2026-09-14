//! kwa_json / kutoka_json: Value <-> JSON string codec, exposed from the existing `mfumo`
//! module (no new `leta` target — see docs/design/json-codec-design.md).

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, extern_env_from_imports, Module};

fn compile(src: &str) -> Module {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    semantic_check_with_env(&module, false, fns, consts).expect("semantic check");
    module
}

#[test]
fn kwa_json_encodes_a_number() {
    let src = r#"
        leta mfumo

        kazi thamani() -> Neno {
            weka encoded = jaribu (kwa_json(42.0))
            rejesha encoded
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "thamani", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("42.0".to_string()));
}

#[test]
fn kwa_json_encodes_a_struct_reflectively() {
    let src = r#"
        leta mfumo

        umbo Pika { x: Namba, y: Neno }

        kazi thamani() -> Neno {
            weka p = Pika { x: 1.0, y: "hi" }
            weka encoded = jaribu (kwa_json(p))
            rejesha encoded
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "thamani", vec![]).expect("runs");
    let Value::Neno(s) = result else { panic!("expected Neno") };
    let parsed: serde_json::Value = serde_json::from_str(&s).expect("valid json");
    assert_eq!(parsed, serde_json::json!({"x": 1.0, "y": "hi"}));
}

#[test]
fn kutoka_json_decodes_an_object_as_kamusi() {
    let src = r#"
        leta mfumo

        kazi thamani() -> Namba {
            weka m = jaribu (kutoka_json("{\"a\": 5}"))
            rejesha m.pata("a") kama Namba
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "thamani", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(5.0));
}

#[test]
fn kutoka_json_rejects_invalid_json() {
    let src = r#"
        leta mfumo

        kazi thamani() -> Ukweli {
            linganisha kutoka_json("{ batili") {
                Tokeo::Kosa(_) => { rejesha kweli }
                Tokeo::Sawa(_) => { rejesha si_kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "thamani", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true), "invalid JSON must return Tokeo(Kosa(...)), not panic");
}

#[test]
fn round_trip_through_json_string() {
    // kutoka_json is declared Tokeo<Kamusi<Neno, Unknown>, Neno> (the common "parse a JSON
    // object" case, since a builtin's static return type can't vary with its input) — round
    // trip through an object shape so .pata(...) type-checks on the decoded value.
    let src = r#"
        leta mfumo

        kazi thamani() -> Namba {
            weka original = kamusi()
            original.ingiza("thamani", 7.0)
            weka encoded = jaribu (kwa_json(original))
            weka decoded = jaribu (kutoka_json(encoded))
            rejesha decoded.pata("thamani") kama Namba
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "thamani", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(7.0));
}

#[test]
fn kwa_json_rejects_kasha_gc_handle() {
    let src = r#"
        leta mfumo
        leta kasha_gc

        kazi thamani() -> Ukweli {
            weka a = kasha_gc_unda(1.0)
            linganisha kwa_json(a) {
                Tokeo::Kosa(_) => { rejesha kweli }
                Tokeo::Sawa(_) => { rejesha si_kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "thamani", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true), "Kasha_GC handles have no JSON representation");
}
