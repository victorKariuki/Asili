use std::collections::HashMap;
use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env};

/// Test 1: Parse module-level constant
#[test]
fn test_parse_module_constant() {
    let src = r#"
        thabiti PI = 3.14159

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.constants.len(), 1, "should parse one constant");
    assert_eq!(module.constants[0].name, "PI", "constant name should be PI");
}

/// Test 2: Multiple module-level constants
#[test]
fn test_multiple_module_constants() {
    let src = r#"
        thabiti PI = 3.14159
        thabiti E = 2.71828
        thabiti TAU = 6.28318

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.constants.len(), 3, "should parse three constants");
    assert_eq!(module.constants[0].name, "PI");
    assert_eq!(module.constants[1].name, "E");
    assert_eq!(module.constants[2].name, "TAU");
}

/// Test 3: Constant with different types
#[test]
fn test_constant_types() {
    let src = r#"
        thabiti NAMBA_CONST: Namba = 42.0
        thabiti NENO_CONST: Neno = "hello"
        thabiti UKWELI_CONST: Ukweli = kweli

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.constants.len(), 3, "should parse three constants");
}

/// Test 4: Constant parsed before functions
#[test]
fn test_constant_before_function() {
    let src = r#"
        thabiti MAGIC = 42.0

        kazi foo(n: Namba) -> Namba {
            rejesha n
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.constants.len(), 1, "should parse constant");
    assert_eq!(module.functions.len(), 2, "should parse two functions");
    assert_eq!(module.constants[0].name, "MAGIC");
    assert_eq!(module.functions[0].name, "foo");
}

/// Test 5: Constants and structs together
#[test]
fn test_constant_with_struct() {
    let src = r#"
        thabiti VERSION = "1.0.0"

        umbo Point {
            x: Namba
            y: Namba
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.constants.len(), 1, "should parse one constant");
    assert_eq!(module.structs.len(), 1, "should parse one struct");
}

/// Test 6: Semantic check with module constants
#[test]
fn test_semantic_check_module_constants() {
    let src = r#"
        thabiti PI = 3.14159
        thabiti E = 2.71828

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_ok(), "should pass semantic check: {:?}", result.err());
}

/// Test 7: Constants in ExportTable
#[test]
fn test_export_table_constants() {
    let src = r#"
        thabiti MAGIC: Namba = 42.0

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.constants.len(), 1);
    assert_eq!(module.constants[0].name, "MAGIC");
}

/// Test 8: Mix of constants and functions
#[test]
fn test_constants_and_functions_parsing() {
    let src = r#"
        thabiti PI = 3.14159
        thabiti E = 2.71828

        kazi foo() -> Namba {
            rejesha 1.0
        }

        thabiti TAU = 6.28318

        kazi bar(x: Namba) -> Namba {
            rejesha x + 1.0
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.constants.len(), 3, "should parse three constants");
    assert_eq!(module.functions.len(), 3, "should parse three functions");
}

/// Test 9: Constant with expression
#[test]
fn test_constant_with_expression() {
    let src = r#"
        thabiti PI = 3.14159
        thabiti TAU = 2.0 * PI

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.constants.len(), 2, "should parse two constants");
}

/// Test 10: Constants with explicit types
#[test]
fn test_constants_explicit_types() {
    let src = r#"
        thabiti PI: Namba = 3.14159
        thabiti MESSAGE: Neno = "Hello, World!"
        thabiti IS_READY: Ukweli = kweli

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.constants.len(), 3, "should parse three constants with types");
}

/// Test 11: a module-level constant is actually bound at runtime (not just parsed/type-checked).
/// Regression test: `thabiti` constants used to be recognized by the semantic analyzer but never
/// seeded into the evaluator's environment, so referencing one by name failed with UndefinedVar.
#[test]
fn test_constant_is_usable_at_runtime() {
    let src = r#"
        thabiti PI: Namba = 3.14159

        kazi kuu(hoja: Orodha<Neno>) -> Namba {
            rejesha PI
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = run_function(&module, "kuu", vec![Value::Orodha(vec![])])
        .expect("kuu should run and return PI");
    match result {
        Value::Namba(n) => assert!((n - 3.14159).abs() < 1e-9, "expected PI, got {n}"),
        other => panic!("expected Namba(3.14159), got {other:?}"),
    }
}

/// Test 12: a constant expression (not just a literal) evaluates before it's used.
#[test]
fn test_constant_expression_is_evaluated_before_use() {
    let src = r#"
        thabiti DOUBLE_PI: Namba = 3.14159 * 2.0

        kazi kuu(hoja: Orodha<Neno>) -> Namba {
            rejesha DOUBLE_PI
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = run_function(&module, "kuu", vec![Value::Orodha(vec![])])
        .expect("kuu should run and return DOUBLE_PI");
    match result {
        Value::Namba(n) => assert!((n - 6.28318).abs() < 1e-9, "expected 6.28318, got {n}"),
        other => panic!("expected Namba(6.28318), got {other:?}"),
    }
}
