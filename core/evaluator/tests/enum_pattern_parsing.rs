use asili_lexer::tokenize;
use asili_parser::parse_tokens;

/// Test 1: Parse simple enum pattern
#[test]
fn test_parse_enum_pattern_simple() {
    let src = r#"
        jenum Color {
            Red,
            Green
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            linganisha x {
                Color::Red => { }
                _ => { }
            }
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Color"));
}

/// Test 2: Parse enum pattern with data
#[test]
fn test_parse_enum_pattern_with_data() {
    let src = r#"
        jenum MyOption {
            Some(Namba),
            Hamna
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            linganisha opt {
                MyOption::Some(x) => { }
                MyOption::Hamna => { }
            }
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "MyOption"));
}

/// Test 3: Parse multiple enum patterns
#[test]
fn test_parse_multiple_enum_patterns() {
    let src = r#"
        jenum MyResult {
            Ok(Namba),
            Err(Neno)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            linganisha res {
                MyResult::Ok(v) => { }
                MyResult::Err(e) => { }
                _ => { }
            }
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "MyResult"));
}

/// Test 4: Parse enum pattern with wildcard data
#[test]
fn test_parse_enum_pattern_wildcard_data() {
    let src = r#"
        jenum Message {
            Text(Neno),
            Number(Namba)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            linganisha msg {
                Message::Text(_) => { }
                Message::Number(_) => { }
            }
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Message"));
}

/// Test 5: Parse standard Chaguo pattern
#[test]
fn test_parse_chaguo_pattern() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            linganisha opt {
                Chaguo::Kuna(x) => { }
                Chaguo::Hamna => { }
            }
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Chaguo"));
}

/// Test 6: Parse standard Tokeo pattern
#[test]
fn test_parse_tokeo_pattern() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            linganisha res {
                Tokeo::Sawa(v) => { }
                Tokeo::Kosa(e) => { }
            }
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Tokeo"));
}

/// Test 7: Parse nested enum pattern
#[test]
fn test_parse_nested_enum_pattern() {
    let src = r#"
        jenum Wrapper {
            Item(Namba)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            linganisha w {
                Wrapper::Item(v) => { }
                _ => { }
            }
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Wrapper"));
}

/// Test 8: Parse enum variant without data pattern
#[test]
fn test_parse_enum_no_data_pattern() {
    let src = r#"
        jenum Status {
            Active,
            Inactive
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            linganisha s {
                Status::Active => { }
                Status::Inactive => { }
            }
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Status"));
}
