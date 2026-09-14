//! Namba_Kuu (arbitrary-precision integer) / Namba_Sahihi (arbitrary-precision decimal).
//! No literal syntax — construct via namba_kuu_kutoka/namba_sahihi_kutoka (parsing a Neno) or
//! an infallible `kama Namba_Kuu`/`kama Namba_Sahihi` cast from Namba.

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
fn namba_kuu_kutoka_parses_a_huge_integer() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Neno {
            weka n = jaribu (namba_kuu_kutoka("123456789012345678901234567890"))
            rejesha n kama Neno
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("123456789012345678901234567890".to_string()));
}

#[test]
fn namba_kuu_addition_beyond_f64_precision() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Neno {
            weka a = jaribu (namba_kuu_kutoka("99999999999999999999999999999999"))
            weka b = jaribu (namba_kuu_kutoka("1"))
            rejesha (a + b) kama Neno
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("100000000000000000000000000000000".to_string()));
}

#[test]
fn namba_kuu_from_invalid_string_is_kosa() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Ukweli {
            linganisha namba_kuu_kutoka("sio namba") {
                Tokeo::Sawa(_) => { rejesha si_kweli }
                Tokeo::Kosa(_) => { rejesha kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn namba_kuu_division_is_exact_integer_division() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Neno {
            weka a = jaribu (namba_kuu_kutoka("100"))
            weka b = jaribu (namba_kuu_kutoka("3"))
            rejesha (a / b) kama Neno
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("33".to_string()), "Namba_Kuu / should truncate, not produce a fraction");
}

#[test]
fn namba_kuu_comparison() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Ukweli {
            weka a = jaribu (namba_kuu_kutoka("999999999999999999999"))
            weka b = jaribu (namba_kuu_kutoka("1000000000000000000000"))
            rejesha a < b
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn namba_widens_to_namba_kuu_in_mixed_arithmetic() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Neno {
            weka a = jaribu (namba_kuu_kutoka("1000000000000000000000"))
            rejesha (a + 1) kama Neno
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("1000000000000000000001".to_string()));
}

#[test]
fn cast_namba_to_namba_kuu_is_infallible() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Neno {
            weka n = 42 kama Namba_Kuu
            rejesha n kama Neno
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("42".to_string()));
}

#[test]
fn cast_namba_kuu_to_namba_is_fallible_chaguo() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Ukweli {
            weka n = jaribu (namba_kuu_kutoka("12345"))
            weka c = n kama Namba
            rejesha c.ni_po()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn namba_sahihi_kutoka_preserves_decimal_precision() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Neno {
            weka n = jaribu (namba_sahihi_kutoka("0.123456789012345678901234567890"))
            rejesha n kama Neno
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("0.123456789012345678901234567890".to_string()));
}

#[test]
fn namba_sahihi_addition() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Neno {
            weka a = jaribu (namba_sahihi_kutoka("0.1"))
            weka b = jaribu (namba_sahihi_kutoka("0.2"))
            rejesha (a + b) kama Neno
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(
        result,
        Value::Neno("0.3".to_string()),
        "Namba_Sahihi's decimal arithmetic must not reproduce f64's 0.1+0.2 rounding artifact"
    );
}

#[test]
fn mixing_namba_kuu_and_namba_sahihi_promotes_to_sahihi() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Neno {
            weka a = jaribu (namba_kuu_kutoka("2"))
            weka b = jaribu (namba_sahihi_kutoka("0.5"))
            rejesha (a + b) kama Neno
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("2.5".to_string()));
}

#[test]
fn division_by_zero_namba_kuu_is_evalerror_not_panic() {
    let src = r#"
        leta hisabati

        kazi jaribu() -> Namba {
            weka a = jaribu (namba_kuu_kutoka("5"))
            weka b = jaribu (namba_kuu_kutoka("0"))
            weka c = a / b
            rejesha 1
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]);
    assert!(result.is_err(), "dividing Namba_Kuu by zero should be a clean error, not a panic");
}
