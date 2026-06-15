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
    assert!(module.enums.iter().any(|e| e.name == "Color"));
    assert!(module.impls.iter().any(|i| i.target == "Color"));
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
    assert!(module.enums.iter().any(|e| e.name == "Status"));
    let status_impl = module.impls.iter().find(|i| i.target == "Status").expect("Status impl");
    assert!(!status_impl.body.is_empty());
    assert_eq!(status_impl.body[0].params.len(), 2);
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
    assert!(module.enums.iter().any(|e| e.name == "Message"));
    let message_impl = module.impls.iter().find(|i| i.target == "Message").expect("Message impl");
    assert_eq!(message_impl.body.len(), 2);
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
    assert!(module.enums.iter().any(|e| e.name == "Option"));
    let option_impl = module.impls.iter().find(|i| i.target == "Option").expect("Option impl");
    assert_eq!(option_impl.body[0].name, "unwrap");
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
    assert!(module.enums.iter().any(|e| e.name == "Result"));
    let result_impls: Vec<_> = module.impls.iter().filter(|i| i.target == "Result").collect();
    assert_eq!(result_impls.len(), 2);
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
    let box_impl = module.impls.iter().find(|i| i.target == "Box").expect("Box impl");
    assert_eq!(box_impl.body[0].params[0].name, "self");
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
    let either_impl = module.impls.iter().find(|i| i.target == "Either").expect("Either impl");
    assert_eq!(either_impl.body[0].return_type.name, "Namba");
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
    assert!(module.enums.iter().any(|e| e.name == "Comparable"));
    assert!(module.impls.iter().any(|i| i.target == "Comparable"));
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
    let event_impl = module.impls.iter().find(|i| i.target == "Event").expect("Event impl");
    assert_eq!(event_impl.body[0].return_type.name, "Tupu");
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
    assert!(module.enums.iter().any(|e| e.name == "Status"));
    assert_eq!(module.structs.len(), 1);
    assert!(module.impls.iter().any(|i| i.target == "Status"));
    assert!(module.impls.iter().any(|i| i.target == "Person"));
}
