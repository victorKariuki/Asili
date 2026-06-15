use asili_lexer::tokenize;
use asili_parser::parse_tokens;

/// Test 1: Simple enum variant construction without data
#[test]
fn test_enum_construct_simple() {
    let src = r#"
        jenum Color {
            Red,
            Green,
            Blue
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            tena Color::Red
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Color"), "should parse Color enum");
    let e = module.enums.iter().find(|e| e.name == "Color").unwrap();
    assert_eq!(e.variants.len(), 3);
}

/// Test 2: Enum variant with data construction
#[test]
fn test_enum_construct_with_data() {
    let src = r#"
        jenum Option {
            Some(Namba),
            None
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            tena Option::Some(42.0)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let e = module.enums.iter().find(|e| e.name == "Option").expect("Option enum");
    assert_eq!(e.variants[0].name, "Some");
    assert!(e.variants[0].data.is_some());
}

/// Test 3: Multiple enum constructions
#[test]
fn test_enum_multiple_constructions() {
    let src = r#"
        jenum Status {
            Active,
            Inactive,
            Pending
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha s1 = Status::Active
            acha s2 = Status::Inactive
            acha s3 = Status::Pending
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums[0].variants.len(), 3);
}

/// Test 4: Enum with string data
#[test]
fn test_enum_with_string_data() {
    let src = r#"
        jenum Message {
            Text(Neno),
            Number(Namba),
            Empty
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha m1 = Message::Text("hello")
            acha m2 = Message::Number(123.0)
            acha m3 = Message::Empty
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums[0].variants.len(), 3);
    assert!(module.enums[0].variants[0].data.is_some());
    assert!(module.enums[0].variants[1].data.is_some());
    assert!(module.enums[0].variants[2].data.is_none());
}

/// Test 5: Nested enum construction
#[test]
fn test_enum_nested_construction() {
    let src = r#"
        jenum Result<T, E> {
            Ok(T),
            Err(E)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha r = Result::Ok("success")
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums[0].generics.len(), 2);
}

/// Test 6: Enum construction in variable binding
#[test]
fn test_enum_in_variable_binding() {
    let src = r#"
        jenum Bool {
            True,
            False
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha value = Bool::True
            acha other = Bool::False
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums[0].variants.len(), 2);
}

/// Test 7: Enum construction with complex data type
#[test]
fn test_enum_complex_data() {
    let src = r#"
        jenum Container {
            ListData(Orodha<Namba>),
            MapData(Kamusi<Neno, Namba>),
            JustString(Neno),
            Empty
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha list = orodha(1.0, 2.0, 3.0)
            acha c = Container::ListData(list)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums[0].variants.len(), 4);
    assert!(module.enums[0].variants[3].data.is_none());
}

/// Test 8: Multiple enum types
#[test]
fn test_multiple_enum_types() {
    let src = r#"
        jenum Shape {
            Circle,
            Rectangle,
            Triangle
        }

        jenum Color {
            Red,
            Green,
            Blue
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha shape = Shape::Circle
            acha color = Color::Red
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Shape"), "should have Shape enum");
    assert!(module.enums.iter().any(|e| e.name == "Color"), "should have Color enum");
}

/// Test 9: Enum construction with multiple variants
#[test]
fn test_enum_generic_construction() {
    let src = r#"
        jenum Pair {
            Values(Namba),
            Single(Namba),
            Empty
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha p = Pair::Single(10.0)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums[0].variants.len(), 3);
}

/// Test 10: Enum construction in return statement
#[test]
fn test_enum_in_return() {
    let src = r#"
        jenum Status {
            Success,
            Error,
            Pending
        }

        kazi make_status(hoja: Orodha<Neno>) -> Tupu {
            tena Status::Success
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            acha s = make_status(hoja)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.enums[0].name, "Status");
}
