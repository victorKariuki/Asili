use asili_lexer::tokenize;
use asili_parser::parse_tokens;

/// Test 1: Chaguo (Option) enum is available
#[test]
fn test_chaguo_enum_available() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha opt = Chaguo::Some(42.0)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Chaguo"));
}

/// Test 2: Chaguo has correct variants
#[test]
fn test_chaguo_variants() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let chaguo = module.enums.iter().find(|e| e.name == "Chaguo").expect("Chaguo");
    assert_eq!(chaguo.variants.len(), 2);
    assert_eq!(chaguo.variants[0].name, "Some");
    assert_eq!(chaguo.variants[1].name, "Hamna");
}

/// Test 3: Chaguo is generic with T parameter
#[test]
fn test_chaguo_generic() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let chaguo = module.enums.iter().find(|e| e.name == "Chaguo").expect("Chaguo");
    assert_eq!(chaguo.generics.len(), 1);
    assert_eq!(chaguo.generics[0], "T");
}

/// Test 4: Tokeo (Result) enum is available
#[test]
fn test_tokeo_enum_available() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha res = Tokeo::Ok(10.0)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Tokeo"));
}

/// Test 5: Tokeo has correct variants
#[test]
fn test_tokeo_variants() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let tokeo = module.enums.iter().find(|e| e.name == "Tokeo").expect("Tokeo");
    assert_eq!(tokeo.variants.len(), 2);
    assert_eq!(tokeo.variants[0].name, "Ok");
    assert_eq!(tokeo.variants[1].name, "Err");
}

/// Test 6: Tokeo is generic with T and E parameters
#[test]
fn test_tokeo_generic() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let tokeo = module.enums.iter().find(|e| e.name == "Tokeo").expect("Tokeo");
    assert_eq!(tokeo.generics.len(), 2);
    assert_eq!(tokeo.generics[0], "T");
    assert_eq!(tokeo.generics[1], "E");
}

/// Test 7: Chaguo Some with data
#[test]
fn test_chaguo_some_with_data() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha opt = Chaguo::Some("hello")
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Chaguo"));
}

/// Test 8: Chaguo Hamna without data
#[test]
fn test_chaguo_hamna_no_data() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha opt = Chaguo::Hamna
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Chaguo"));
}

/// Test 9: Tokeo Ok with data
#[test]
fn test_tokeo_ok_with_data() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha res = Tokeo::Ok(100.0)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Tokeo"));
}

/// Test 10: Tokeo Err with data
#[test]
fn test_tokeo_err_with_data() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha res = Tokeo::Err("error message")
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Tokeo"));
}

/// Test 11: Standard enums don't duplicate user-defined
#[test]
fn test_no_duplicate_standard_enums() {
    let src = r#"
        jenum Chaguo {
            Custom
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let chaguo_count = module.enums.iter().filter(|e| e.name == "Chaguo").count();
    assert_eq!(chaguo_count, 2);
}

/// Test 12: Standard enums are public
#[test]
fn test_standard_enums_public() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let chaguo = module.enums.iter().find(|e| e.name == "Chaguo").expect("Chaguo");
    assert!(chaguo.is_public);
    let tokeo = module.enums.iter().find(|e| e.name == "Tokeo").expect("Tokeo");
    assert!(tokeo.is_public);
}
