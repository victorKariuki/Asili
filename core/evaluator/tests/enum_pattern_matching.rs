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

/// Test 1: Pattern match on enum variant without data
#[test]
fn test_enum_pattern_simple_variant() {
    let src = r#"
        jenum Color {
            Red,
            Green,
            Blue
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            weka c = Color::Red
            linganisha c {
                Color::Red => { rejesha 1.0 }
                Color::Green => { rejesha 2.0 }
                Color::Blue => { rejesha 3.0 }
                _ => { rejesha 0.0 }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Namba(1.0));
}

/// Test 2: Pattern match on enum variant with data
#[test]
fn test_enum_pattern_with_data() {
    let src = r#"
        jenum Option {
            Some(Namba),
            Hamna
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            weka opt = Option::Some(42.0)
            linganisha opt {
                Option::Some(x) => { rejesha x }
                Option::Hamna => { rejesha 0.0 }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Namba(42.0));
}

/// Test 3: Pattern match with Hamna variant
#[test]
fn test_enum_pattern_hamna_variant() {
    let src = r#"
        jenum Option {
            Some(Namba),
            Hamna
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            weka opt = Option::Hamna
            linganisha opt {
                Option::Some(x) => { rejesha x }
                Option::Hamna => { rejesha 0.0 }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Namba(0.0));
}

/// Test 4: Pattern match on Result Ok variant
#[test]
fn test_enum_pattern_result_ok() {
    let src = r#"
        jenum Result {
            Ok(Namba),
            Err(Neno)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            weka res = Result::Ok(100.0)
            linganisha res {
                Result::Ok(val) => { rejesha val }
                Result::Err(_) => { rejesha 0.0 }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Namba(100.0));
}

/// Test 5: Pattern match on Result Err variant
#[test]
fn test_enum_pattern_result_err() {
    let src = r#"
        jenum Result {
            Ok(Namba),
            Err(Neno)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno {
            weka res = Result::Err("error")
            linganisha res {
                Result::Ok(_) => { rejesha "ok" }
                Result::Err(e) => { rejesha e }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Neno("error".to_string()));
}

/// Test 6: Wildcard pattern on enum
#[test]
fn test_enum_pattern_wildcard() {
    let src = r#"
        jenum Status {
            Active,
            Inactive,
            Pending
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            weka s = Status::Pending
            linganisha s {
                Status::Active => { rejesha 1.0 }
                _ => { rejesha 99.0 }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Namba(99.0));
}

/// Test 7: Multiple variants with different data types
#[test]
fn test_enum_pattern_mixed_variants() {
    let src = r#"
        jenum Message {
            Text(Neno),
            Number(Namba),
            None
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno {
            weka msg = Message::Text("hello")
            linganisha msg {
                Message::Text(s) => { rejesha s }
                Message::Number(_) => { rejesha "number" }
                Message::None => { rejesha "none" }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Neno("hello".to_string()));
}

/// Test 8: Pattern matching with standard Chaguo enum
#[test]
fn test_enum_pattern_chaguo() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            weka opt = Chaguo::Some(50.0)
            linganisha opt {
                Chaguo::Some(x) => { rejesha x }
                Chaguo::Hamna => { rejesha 0.0 }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Namba(50.0));
}

/// Test 9: Pattern matching with standard Tokeo enum
#[test]
fn test_enum_pattern_tokeo() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Ukweli {
            weka res = Tokeo::Ok(123.0)
            linganisha res {
                Tokeo::Ok(_) => { rejesha kweli }
                Tokeo::Err(_) => { rejesha si_kweli }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Ukweli(true));
}

/// Test 10: Nested pattern matching with enum data
#[test]
fn test_enum_pattern_nested() {
    let src = r#"
        jenum Wrapper {
            Item(Namba),
            Empty
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            weka w1 = Wrapper::Item(75.0)
            weka w2 = Wrapper::Empty
            linganisha w1 {
                Wrapper::Item(v) => { rejesha v * 2.0 }
                Wrapper::Empty => { rejesha 0.0 }
            }
        }
    "#;
    let v = parse_and_eval(src, "test");
    assert_eq!(v, asili_evaluator::Value::Namba(150.0));
}
