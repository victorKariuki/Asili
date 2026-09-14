use std::collections::HashMap;
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env};

/// Test 1: Valid enum passes semantic check
#[test]
fn test_enum_semantic_valid() {
    let src = r#"
        jenum Color {
            Red,
            Green,
            Blue
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_ok(), "valid enum should pass semantic check: {:?}", result.err());
}

/// Test 2: Duplicate variant names error
#[test]
fn test_enum_duplicate_variants() {
    let src = r#"
        jenum Status {
            Active,
            Inactive,
            Active
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_err(), "duplicate variants should error");
    if let Err(errs) = result {
        assert!(errs.iter().any(|d| d.code == "SEM093"), "should emit SEM093 for duplicate variants");
    }
}

/// Test 3: Generic enum semantic check
#[test]
fn test_generic_enum_semantic() {
    let src = r#"
        jenum Option<T> {
            Some(T),
            None
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_ok(), "generic enum should pass: {:?}", result.err());
}

/// Test 4: Multiple enums with different variants
#[test]
fn test_multiple_enums_semantic() {
    let src = r#"
        jenum Result {
            Ok,
            Err
        }

        jenum Status {
            Running,
            Stopped,
            Paused
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_ok(), "multiple different enums should pass: {:?}", result.err());
}

/// Test 5: Enum with complex variant types
#[test]
fn test_enum_complex_types() {
    let src = r#"
        jenum Container {
            ListData(Orodha<Namba>),
            MapData(Kamusi<Neno, Namba>),
            JustString(Neno),
            Empty
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_ok(), "complex variant types should pass: {:?}", result.err());
}

/// Test 6: Enum mixed with other declarations
#[test]
fn test_enum_with_other_declarations() {
    let src = r#"
        jenum Shape {
            Circle,
            Rectangle
        }

        jenum Color {
            Red,
            Blue
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_ok(), "enum with other enums should pass: {:?}", result.err());
}

/// Test 7: Public enum with private enum
#[test]
fn test_public_private_enums() {
    let src = r#"
        umma jenum PublicResult {
            Ok,
            Err
        }

        jenum PrivateStatus {
            Active,
            Inactive
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_ok(), "public and private enums should pass: {:?}", result.err());
    let public_result = module.enums.iter().find(|e| e.name == "PublicResult").expect("PublicResult");
    let private_status = module.enums.iter().find(|e| e.name == "PrivateStatus").expect("PrivateStatus");
    assert!(public_result.is_public);
    assert!(!private_status.is_public);
}

/// Test 8: Enum variant case sensitivity
#[test]
fn test_enum_case_sensitivity() {
    let src = r#"
        jenum Status {
            active,
            Active,
            ACTIVE
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    // Different cases should be treated as different variants
    assert!(result.is_ok(), "case-sensitive variants should pass: {:?}", result.err());
}

/// Test 9: Empty enum (no variants)
#[test]
fn test_empty_enum() {
    let src = r#"
        jenum Empty {
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_ok(), "empty enum should pass: {:?}", result.err());
    assert_eq!(module.enums[0].variants.len(), 0);
}

/// Test 10: Enum in constants module
#[test]
fn test_enum_with_constants() {
    let src = r#"
        thabiti DEFAULT_SIZE: Namba = 100.0

        jenum Size {
            Small,
            Medium,
            Large
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_ok(), "enum with constants should pass: {:?}", result.err());
}
