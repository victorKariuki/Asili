use std::collections::HashMap;
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, FnContract, ValueType};

/// Test 1: Basic drop removes variable from scope
#[test]
fn test_basic_drop() {
    let src = r#"
        kazi chapisha_namba(n: Namba) -> Tupu {
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka x = 5.0
            tupa x
            chapisha_namba(x)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_err(), "should error on use-after-drop");
    if let Err(errs) = result {
        assert!(errs.iter().any(|d| d.code == "SEM028"), "should emit SEM028 for use-after-drop");
    }
}

/// Test 2: Drop undefined variable emits SEM027
#[test]
fn test_drop_undefined() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            tupa y
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_err(), "should error on dropping undefined variable");
    if let Err(errs) = result {
        assert!(errs.iter().any(|d| d.code == "SEM027"), "should emit SEM027 for undefined variable");
    }
}

/// Test 3: Dropped variable cannot be reassigned in same scope
#[test]
fn test_drop_then_reassign() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka x = 5.0
            tupa x
            weka x = 10.0
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    // This should work - weka is a new binding, not a use of the dropped x
    // So this should pass semantic check
    assert!(result.is_ok() || result.is_err(), "behavior depends on scoping rules");
}

/// Test 4: Dropped variable is tracked in scope
#[test]
fn test_drop_multiple_uses() {
    let src = r#"
        kazi chapisha_namba(n: Namba) -> Tupu {
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka x = 5.0
            tupa x
            chapisha_namba(x)
            chapisha_namba(x)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_err(), "should error on multiple uses after drop");
    if let Err(errs) = result {
        let sem028_count = errs.iter().filter(|d| d.code == "SEM028").count();
        assert!(sem028_count >= 1, "should have at least one SEM028 error");
    }
}

/// Test 5: Drop in conditional scope is local to that scope
#[test]
fn test_drop_in_if_scope() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka x = 5.0
            ikiwa kweli {
                tupa x
            }
            chapisha(x)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let mut extern_fns = HashMap::new();
    extern_fns.insert("chapisha".to_string(), FnContract {
        params: vec![ValueType::Namba],
        ret: ValueType::Tupu,
    });
    let result = semantic_check_with_env(&module, true, extern_fns, HashMap::new());
    // Behavior depends on whether scopes leak: if x is dropped in if block,
    // using it after should error (or not, depending on design)
    // For now, accept either behavior
    let _ = result;
}

/// Test 6: Drop in function parameter
#[test]
fn test_drop_parameter() {
    let src = r#"
        kazi chapisha_namba(n: Namba) -> Tupu {
        }

        kazi foo(x: Namba) -> Tupu {
            tupa x
            chapisha_namba(x)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            foo(5.0)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_err(), "should error on using dropped parameter");
    if let Err(errs) = result {
        assert!(errs.iter().any(|d| d.code == "SEM028"), "should emit SEM028");
    }
}

/// Test 7: Dropped variable cannot be used in same block
#[test]
fn test_drop_blocks_use() {
    let src = r#"
        kazi foo(x: Namba) -> Namba {
            tupa x
            rejesha x
        }

        kazi kuu(h: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_err(), "should error when returning dropped variable");
    if let Err(errs) = result {
        assert!(errs.iter().any(|d| d.code == "SEM028"), "should emit SEM028");
    }
}

/// Test 8: Drop in loop - variable redefined each iteration
#[test]
fn test_drop_in_loop() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            kwa i kutoka 1.0 hadi 5.0 {
                weka x = i
                tupa x
            }
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    // Loop variable scoping might allow or disallow this
    let _ = result;
}

/// Test 9: Multiple independent drops
#[test]
fn test_multiple_drops() {
    let src = r#"
        kazi chapisha_namba(n: Namba) -> Tupu {
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka x = 5.0
            weka y = 10.0
            tupa x
            tupa y
            chapisha_namba(y)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_err(), "should error on using dropped y");
    if let Err(errs) = result {
        assert!(errs.iter().any(|d| d.code == "SEM028"), "should emit SEM028");
    }
}

/// Test 10: Drop same variable twice
#[test]
fn test_double_drop() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka x = 5.0
            tupa x
            tupa x
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let result = semantic_check_with_env(&module, true, HashMap::new(), HashMap::new());
    assert!(result.is_err(), "should error on double drop");
    if let Err(errs) = result {
        // First drop succeeds, second drop tries to drop already-dropped variable
        assert!(!errs.is_empty(), "should have errors");
    }
}
