use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env};
use asili_evaluator::run_function;
use std::collections::HashMap;

fn parse_and_eval(src: &str, func: &str) -> asili_evaluator::Value {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    semantic_check_with_env(&module, true, HashMap::new(), HashMap::new()).expect("semantic");
    run_function(&module, func, vec![]).expect("run")
}

/// Test 1: Create and return Ok result
#[test]
fn test_tokeo_ok_value() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Tupu {
            weka res = Tokeo::Ok(42.0)
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Tupu);
}

/// Test 2: Create and return Err result
#[test]
fn test_tokeo_err_value() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Tupu {
            weka res = Tokeo::Err("error")
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Tupu);
}

/// Test 3: Check if result is ok
#[test]
fn test_tokeo_ni_kosa_ok() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Ukweli {
            weka res = Tokeo::Ok(100.0)
            rejesha res.ni_kosa()
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Ukweli(false));
}

/// Test 4: Check if result is error
#[test]
fn test_tokeo_ni_kosa_err() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Ukweli {
            weka res = Tokeo::Err("something failed")
            rejesha res.ni_kosa()
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Ukweli(true));
}

/// Test 5: Extract error from Err
#[test]
fn test_tokeo_extract_error() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno {
            weka res = Tokeo::Err("failed")
            rejesha res.kosa()
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Neno("failed".to_string()));
}

/// Test 6: Get ok value with default
#[test]
fn test_tokeo_angu_ok() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            weka res = Tokeo::Ok(50.0)
            rejesha res.angu(0.0)
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Namba(50.0));
}

/// Test 7: Get error value with default
#[test]
fn test_tokeo_angu_err() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            weka res = Tokeo::Err("problem")
            rejesha res.angu(99.0)
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Namba(99.0));
}

/// Test 8: Propagate error with ?
#[test]
fn test_tokeo_propagate() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi may_fail() -> Tokeo {
            rejesha Tokeo::Err("oops")
        }
        kazi test() -> Tupu {
            weka res = may_fail()
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Tupu);
}

/// Test 9: Chaguo Some value
#[test]
fn test_chaguo_some() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Tupu {
            weka opt = Chaguo::Some(42.0)
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Tupu);
}

/// Test 10: Chaguo None value
#[test]
fn test_chaguo_none() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Tupu {
            weka opt = Chaguo::Hamna
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Tupu);
}

/// Test 11: Chaguo ni_po check
#[test]
fn test_chaguo_ni_po() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Ukweli {
            weka opt = Chaguo::Some(10.0)
            rejesha opt.ni_po()
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Ukweli(true));
}

/// Test 12: Chaguo ni_tupu check
#[test]
fn test_chaguo_ni_tupu() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Ukweli {
            weka opt = Chaguo::Hamna
            rejesha opt.ni_tupu()
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Ukweli(true));
}
