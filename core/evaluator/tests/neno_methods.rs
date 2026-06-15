use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env};
use asili_evaluator::run_function;
use std::collections::HashMap;

fn parse_and_eval(src: &str) -> asili_evaluator::Value {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    semantic_check_with_env(&module, true, HashMap::new(), HashMap::new()).expect("semantic");
    run_function(&module, "test", vec![]).expect("run")
}

/// Test 1: urefu (length) method - character count
#[test]
fn test_neno_urefu() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba { weka s = "habari" rejesha s.urefu() }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Namba(6.0));
}

/// Test 2: biti_ngapi (byte count) method
#[test]
fn test_neno_biti_ngapi() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba { weka s = "abc" rejesha s.biti_ngapi() }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Namba(3.0));
}

/// Test 3: kwa_herufi_ndogo (lowercase) method
#[test]
fn test_neno_kwa_herufi_ndogo() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno { weka s = "HeLLo" rejesha s.kwa_herufi_ndogo() }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Neno("hello".to_string()));
}

/// Test 4: kwa_herufi_kubwa (uppercase) method
#[test]
fn test_neno_kwa_herufi_kubwa() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno { weka s = "HeLLo" rejesha s.kwa_herufi_kubwa() }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Neno("HELLO".to_string()));
}

/// Test 5: anza_na (starts_with) method
#[test]
fn test_neno_anza_na() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Ukweli { weka s = "habari" rejesha s.anza_na("hab") }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Ukweli(true));
}

/// Test 6: anza_na returns false
#[test]
fn test_neno_anza_na_false() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Ukweli { weka s = "habari" rejesha s.anza_na("xyz") }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Ukweli(false));
}

/// Test 7: maliza_na (ends_with) method
#[test]
fn test_neno_maliza_na() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Ukweli { weka s = "habari" rejesha s.maliza_na("ri") }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Ukweli(true));
}

/// Test 8: gawanya (split) method
#[test]
fn test_neno_gawanya() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Tupu {
            weka s = "a,b,c"
            weka result = s.gawanya(",")
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    semantic_check_with_env(&module, true, HashMap::new(), HashMap::new()).expect("semantic");
    let v = run_function(&module, "test", vec![]).expect("run");
    assert_eq!(v, asili_evaluator::Value::Tupu);
}

/// Test 9: badilisha (replace) method
#[test]
fn test_neno_badilisha() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno { weka s = "hello world" rejesha s.badilisha("world", "asili") }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Neno("hello asili".to_string()));
}

/// Test 10: kata (slice) method
#[test]
fn test_neno_kata() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno { weka s = "hello" rejesha s.kata(1, 4) }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Neno("ell".to_string()));
}

/// Test 11: tafuta (find) method - found
#[test]
fn test_neno_tafuta_found() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Tupu {
            weka s = "hello"
            weka result = s.tafuta("ll")
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    semantic_check_with_env(&module, true, HashMap::new(), HashMap::new()).expect("semantic");
    let v = run_function(&module, "test", vec![]).expect("run");
    assert_eq!(v, asili_evaluator::Value::Tupu);
}

/// Test 12: tafuta (find) method - not found
#[test]
fn test_neno_tafuta_not_found() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Tupu {
            weka s = "hello"
            weka result = s.tafuta("xyz")
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    semantic_check_with_env(&module, true, HashMap::new(), HashMap::new()).expect("semantic");
    let v = run_function(&module, "test", vec![]).expect("run");
    assert_eq!(v, asili_evaluator::Value::Tupu);
}
