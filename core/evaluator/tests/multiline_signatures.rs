use asili_lexer::tokenize;
use asili_parser::parse_tokens;

/// Test 1: Parse multi-line function parameters
#[test]
fn test_multiline_params() {
    let src = r#"
        kazi test(
            x: Namba,
            y: Namba,
            z: Namba
        ) -> Namba {
            rejesha x + y + z
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    assert!(module.functions.len() >= 2);
    let test_fn = module.functions.iter().find(|f| f.name == "test").expect("test function");
    assert_eq!(test_fn.params.len(), 3);
}

/// Test 2: Parse multi-line with different indentation
#[test]
fn test_multiline_various_indent() {
    let src = r#"
        kazi foo(
        a: Namba,
            b: Neno,
                c: Ukweli
        ) -> Namba {
            rejesha 0.0
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let foo_fn = module.functions.iter().find(|f| f.name == "foo").expect("foo function");
    assert_eq!(foo_fn.params.len(), 3);
}

/// Test 3: Parse multi-line with trailing comma
#[test]
fn test_multiline_trailing_comma() {
    let src = r#"
        kazi bar(
            x: Namba,
            y: Namba,
        ) -> Namba {
            rejesha x
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let bar_fn = module.functions.iter().find(|f| f.name == "bar").expect("bar function");
    assert_eq!(bar_fn.params.len(), 2);
}

/// Test 4: Parse multi-line return type
#[test]
fn test_multiline_return_type() {
    let src = r#"
        kazi baz(x: Namba) -> Orodha<Namba> {
            rejesha orodha(x)
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let baz_fn = module.functions.iter().find(|f| f.name == "baz").expect("baz function");
    assert_eq!(baz_fn.return_type.name.replace(" ", ""), "Orodha<Namba>");
}

/// Test 5: Parse multi-line struct parameters
#[test]
fn test_multiline_complex_types() {
    let src = r#"
        kazi complex(
            a: Kamusi<Neno, Namba>,
            b: Orodha<
                Jozi<Namba, Neno>
            >
        ) -> Tupu {
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let complex_fn = module.functions.iter().find(|f| f.name == "complex").expect("complex function");
    assert_eq!(complex_fn.params.len(), 2);
}

/// Test 6: Single-line should still work
#[test]
fn test_single_line_still_works() {
    let src = r#"
        kazi simple(x: Namba, y: Neno, z: Ukweli) -> Namba {
            rejesha 1.0
        }

        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let simple_fn = module.functions.iter().find(|f| f.name == "simple").expect("simple function");
    assert_eq!(simple_fn.params.len(), 3);
}
