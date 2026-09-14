use asili_lexer::tokenize;
use asili_parser::parse_tokens;

/// Test 1: Simple enum without data
#[test]
fn test_parse_simple_enum() {
    let src = r#"
        jenum Rangi {
            Nyeusi,
            Nyingine,
            Nzuri
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let rangi = module.enums.iter().find(|e| e.name == "Rangi").expect("Rangi enum");
    assert_eq!(rangi.variants.len(), 3);
    assert_eq!(rangi.variants[0].name, "Nyeusi");
    assert!(rangi.variants[0].data.is_none());
}

/// Test 2: Enum with variant data
#[test]
fn test_parse_enum_with_data() {
    let src = r#"
        jenum Matokeo {
            Mafanikio(Namba),
            Kosa(Neno)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let matokeo = module.enums.iter().find(|e| e.name == "Matokeo").expect("Matokeo enum");
    assert_eq!(matokeo.variants.len(), 2);
    assert!(matokeo.variants[0].data.is_some(), "first variant should have data");
    assert!(matokeo.variants[1].data.is_some(), "second variant should have data");
}

/// Test 3: Generic enum
#[test]
fn test_parse_generic_enum() {
    let src = r#"
        jenum Chaguo<T> {
            Kuna(T),
            Hamna
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let chaguo_test = module.enums.iter().find(|e| e.name == "Chaguo" && e.generics.len() == 1).expect("Chaguo<T> enum");
    assert_eq!(chaguo_test.generics.len(), 1);
    assert_eq!(chaguo_test.generics[0], "T");
}

/// Test 4: Enum with complex types
#[test]
fn test_parse_enum_complex_types() {
    let src = r#"
        jenum Container {
            ListData(Orodha<Namba>),
            MapData(Kamusi<Neno, Namba>),
            JustString(Neno)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let container = module.enums.iter().find(|e| e.name == "Container").expect("Container enum");
    assert_eq!(container.variants.len(), 3);
}

/// Test 5: Multiple enums in module
#[test]
fn test_parse_multiple_enums() {
    let src = r#"
        jenum Color {
            Red,
            Green,
            Blue
        }

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
    assert!(module.enums.iter().any(|e| e.name == "Color"));
    assert!(module.enums.iter().any(|e| e.name == "Size"));
}

/// Test 6: Enum mixed with structs
#[test]
fn test_parse_enum_with_struct() {
    let src = r#"
        umbo Point {
            x: Namba
            y: Namba
        }

        jenum Shape {
            Circle(Namba),
            Rectangle(Point)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.structs.len(), 1);
    assert!(module.enums.iter().any(|e| e.name == "Shape"));
}

/// Test 7: Public enum
#[test]
fn test_parse_public_enum() {
    let src = r#"
        umma jenum Status {
            Active,
            Inactive,
            Pending
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let status = module.enums.iter().find(|e| e.name == "Status").expect("Status enum");
    assert!(status.is_public, "enum should be public");
}

/// Test 8: Enum with single variant
#[test]
fn test_parse_single_variant_enum() {
    let src = r#"
        jenum Unit {
            TheOne
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let unit = module.enums.iter().find(|e| e.name == "Unit").expect("Unit enum");
    assert_eq!(unit.variants.len(), 1);
}

/// Test 9: Enum before and after functions
#[test]
fn test_parse_enum_mixed_with_functions() {
    let src = r#"
        jenum Result {
            Ok,
            Err
        }

        kazi foo() -> Tupu {
        }

        jenum Option {
            Some,
            None
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.enums.iter().any(|e| e.name == "Result"));
    assert!(module.enums.iter().any(|e| e.name == "Option"));
    assert_eq!(module.functions.len(), 2);
}

/// Test 10: Enum with trailing comma
#[test]
fn test_parse_enum_trailing_comma() {
    let src = r#"
        jenum Status {
            Active,
            Inactive,
            Pending,
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let status = module.enums.iter().find(|e| e.name == "Status").expect("Status enum");
    assert_eq!(status.variants.len(), 3);
}
