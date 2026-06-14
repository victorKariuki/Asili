use asili_lexer::tokenize;
use asili_parser::parse_tokens;

/// Test 1: Basic enum method definition and call
#[test]
fn test_enum_method_basic() {
    let src = r#"
        jenum Color {
            Red,
            Green,
            Blue
        }

        shughuli ya Color {
            kazi describe(self: Color) -> Neno {
                tena "kama rangi"
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha c = Color::Red
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums.len(), 1);
    assert_eq!(module.impls.len(), 1);
    assert_eq!(module.impls[0].target, "Color");
}

/// Test 2: Enum method with parameters
#[test]
fn test_enum_method_with_params() {
    let src = r#"
        jenum Status {
            Active,
            Inactive
        }

        shughuli ya Status {
            kazi compare(self: Status, other: Neno) -> Ukweli {
                tena kweli
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums.len(), 1);
    assert!(module.impls[0].body.len() > 0);
    assert_eq!(module.impls[0].body[0].params.len(), 2);
}

/// Test 3: Multiple methods on same enum
#[test]
fn test_enum_multiple_methods() {
    let src = r#"
        jenum Message {
            Text(Neno),
            Number(Namba)
        }

        shughuli ya Message {
            kazi length(self: Message) -> Namba {
                tena 0.0
            }

            kazi is_text(self: Message) -> Ukweli {
                tena si_kweli
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums.len(), 1);
    assert_eq!(module.impls[0].body.len(), 2);
}

/// Test 4: Enum method returning enum
#[test]
fn test_enum_method_returns_enum() {
    let src = r#"
        jenum Option {
            Some(Namba),
            None
        }

        shughuli ya Option {
            kazi unwrap(self: Option) -> Namba {
                tena 0.0
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums.len(), 1);
    assert_eq!(module.impls[0].body[0].name, "unwrap");
}

/// Test 5: Multiple impl blocks on same enum
#[test]
fn test_enum_multiple_impls() {
    let src = r#"
        jenum Result {
            Ok(Namba),
            Err(Neno)
        }

        shughuli ya Result {
            kazi is_ok(self: Result) -> Ukweli {
                tena kweli
            }
        }

        shughuli ya Result {
            kazi is_err(self: Result) -> Ukweli {
                tena si_kweli
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums.len(), 1);
    assert_eq!(module.impls.len(), 2);
    assert_eq!(module.impls[0].target, "Result");
    assert_eq!(module.impls[1].target, "Result");
}

/// Test 6: Enum method with self consumption
#[test]
fn test_enum_method_move_self() {
    let src = r#"
        jenum Box {
            Full(Namba),
            Empty
        }

        shughuli ya Box {
            kazi consume(self: Box) -> Namba {
                tena 0.0
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.impls[0].body[0].params[0].name, "self");
}

/// Test 7: Enum method returns namba
#[test]
fn test_enum_method_complex_return() {
    let src = r#"
        jenum Either {
            Left(Namba),
            Right(Neno)
        }

        shughuli ya Either {
            kazi to_namba(self: Either) -> Namba {
                tena 42.0
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.impls[0].body[0].return_type.name, "Namba");
}

/// Test 8: Enum with method calling convention
#[test]
fn test_enum_method_convention() {
    let src = r#"
        jenum Comparable {
            First,
            Second
        }

        shughuli ya Comparable {
            kazi compare(self: Comparable, other: Neno) -> Ukweli {
                tena kweli
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums.len(), 1);
    assert_eq!(module.impls.len(), 1);
}

/// Test 9: Enum method with no return
#[test]
fn test_enum_method_no_return() {
    let src = r#"
        jenum Event {
            Click,
            Press
        }

        shughuli ya Event {
            kazi handle(self: Event) -> Tupu {
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.impls[0].body[0].return_type.name, "Tupu");
}

/// Test 10: Mixed enum and struct impls
#[test]
fn test_mixed_enum_struct_impls() {
    let src = r#"
        jenum Status {
            Active,
            Inactive
        }

        umbo Person {
            name: Neno,
            age: Namba
        }

        shughuli ya Status {
            kazi describe(self: Status) -> Neno {
                tena "status"
            }
        }

        shughuli ya Person {
            kazi info(self: Person) -> Neno {
                tena "person"
            }
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums.len(), 1);
    assert_eq!(module.structs.len(), 1);
    assert_eq!(module.impls.len(), 2);
}
