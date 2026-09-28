use asili_evaluator::run_function;
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env};
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
    assert_eq!(v, asili_evaluator::Value::neno("hello".to_string()));
}

/// Test 4: kwa_herufi_kubwa (uppercase) method
#[test]
fn test_neno_kwa_herufi_kubwa() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno { weka s = "HeLLo" rejesha s.kwa_herufi_kubwa() }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::neno("HELLO".to_string()));
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
    assert_eq!(v, asili_evaluator::Value::neno("hello asili".to_string()));
}

/// Test 10: kata (slice) method
#[test]
fn test_neno_kata() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno { weka s = "hello" rejesha s.kata(1, 4) }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::neno("ell".to_string()));
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

/// Test 13: herufi_kwa (char-at-index) — in range
#[test]
fn test_neno_herufi_kwa_in_range() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Chaguo<Herufi> { weka s = "habari" rejesha s.herufi_kwa(2) }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(
        v,
        asili_evaluator::Value::Chaguo(Some(Box::new(asili_evaluator::Value::Herufi('b'))))
    );
}

/// Test 14: herufi_kwa — out of range returns Hamna, not a panic
#[test]
fn test_neno_herufi_kwa_out_of_range() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Chaguo<Herufi> { weka s = "abc" rejesha s.herufi_kwa(99) }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Chaguo(None));
}

/// Test 15: herufi_kwa — negative index is out of range, not a wraparound/panic
#[test]
fn test_neno_herufi_kwa_negative_index() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Chaguo<Herufi> { weka s = "abc" rejesha s.herufi_kwa(-1) }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Chaguo(None));
}

/// Test 16: herufi_kwa is grapheme-indexed, matching urefu()'s own counting convention —
/// a multi-byte character (café's é) counts as one position, same as urefu() counts it as one.
#[test]
fn test_neno_herufi_kwa_is_grapheme_indexed_not_byte_indexed() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba { weka s = "café" rejesha s.urefu() }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(
        v,
        asili_evaluator::Value::Namba(4.0),
        "café is 4 graphemes despite é being 2 bytes"
    );
}

#[test]
fn test_neno_convenience_helpers() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno {
            weka s = "ha ha "
            ikiwa s.tupu() { rejesha "mbaya" }
            ikiwa siyo s.ina("ha") { rejesha "mbaya" }
            ikiwa s.hesabu("ha") != 2 { rejesha "mbaya" }
            rejesha "x".rudia(3)
        }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::neno("xxx".to_string()));
}

#[test]
fn test_orodha_helpers() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi ongeza_moja(x: Namba) -> Namba { rejesha x + 1 }
        kazi ni_ya_kati(x: Namba) -> Ukweli { rejesha x > 1 }
        kazi test() -> Neno {
            weka a: Orodha<Namba> = [1, 2, 3]
            weka b = a.ramani("ongeza_moja")
            weka c = a.chuja("ni_ya_kati")
            ikiwa a.hesabu("ni_ya_kati") != 2 { rejesha "kosa" }
            ikiwa siyo a.chunguza("ni_ya_kati") { rejesha "kosa" }
            a.badilisha(0, 9)
            ikiwa a.pata(0).angu(0) != 9 { rejesha "kosa" }
            weka maneno: Orodha<Neno> = ["2", "3"]
            rejesha maneno.unganisha(",")
        }

    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::neno("2,3".to_string()));
}

#[test]
fn test_orodha_chunks_and_string_pipeline() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi mstari(row: Orodha<Namba>) -> Neno {
            rejesha row.kwa_neno().jiunge(" ")
        }
        kazi test() -> Neno {
            weka board: Orodha<Namba> = [1, 2, 3, 4, 5, 6]
            rejesha board.vipande(3).ramani("mstari").jiunge("\n")
        }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::neno("1 2 3\n4 5 6".to_string()));
}

#[test]
fn test_symbolic_bitwise_aliases() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Namba {
            rejesha (12 & 10) + (12 | 3) + (12 ^ 10) + (1 << 3) + (16 >> 2)
        }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::Namba(41.0));
}

#[test]
fn test_if_expression() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test() -> Neno {
            weka n = 3
            weka ujumbe = ikiwa n > 2 { "kubwa" } vinginevyo { "ndogo" }
            rejesha ujumbe
        }
    "#;
    let v = parse_and_eval(src);
    assert_eq!(v, asili_evaluator::Value::neno("kubwa".to_string()));
}

#[test]
fn test_pair_destructuring() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi test(p: Jozi<Namba, Namba>) -> Namba {
            weka (a, b) = p
            rejesha a + b
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    semantic_check_with_env(&module, true, HashMap::new(), HashMap::new()).expect("semantic");
    let v = run_function(
        &module,
        "test",
        vec![asili_evaluator::Value::Jozi(
            Box::new(asili_evaluator::Value::Namba(4.0)),
            Box::new(asili_evaluator::Value::Namba(5.0)),
        )],
    )
    .expect("run");
    assert_eq!(v, asili_evaluator::Value::Namba(9.0));
}
