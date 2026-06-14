//! Parser integration tests: parse_tokens, semantic_check, discover_tests.

use std::collections::HashMap;

use asili_lexer::tokenize;
use asili_parser::{
    discover_tests, parse_tokens, semantic_check, semantic_check_with_env, semantic_check_with_options,
    FnContract, BinaryOp, Expr, ImportPath, Pattern, Stmt, ValueType,
};

#[test]
fn parses_main() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"x\") }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let mut extern_fns = HashMap::new();
    extern_fns.insert(
        "chapisha".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tupu,
        },
    );
    semantic_check_with_env(&module, true, extern_fns, HashMap::new()).expect("semantics");
}

#[test]
fn supports_match_and_try_parse() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { weka x = jaribu gawio(1,2)? linganisha x { _ => { chapisha(\"x\") } } }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    assert!(semantic_check_with_options(&module, true).is_err());
}

#[test]
fn detects_use_after_move() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { weka a: Neno = \"x\" weka b = a weka c = a }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    assert!(semantic_check(&module).is_err());
}

#[test]
fn detects_borrow_alias_conflict() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { weka a: Neno = \"x\" weka r1 = azima a weka r2 = azima_tenda a }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    assert!(semantic_check(&module).is_err());
}

#[test]
fn discovers_tests_with_attribute() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { } #[jaribio] kazi jaribio_moja() -> Tupu { }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let tests = discover_tests(&module);
    assert_eq!(tests.len(), 1);
}

#[test]
fn parses_lebo_and_labeled_break() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { lebo nje: wakati milele { vunja 'nje } }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let kuu = module.functions.iter().find(|f| f.name == "kuu").unwrap();
    let body = &kuu.body.statements;
    assert_eq!(body.len(), 1);
    if let Stmt::While { label, body: block, .. } = &body[0] {
        assert_eq!(label.as_deref(), Some("nje"));
        assert_eq!(block.statements.len(), 1);
        if let Stmt::Break { label: break_label, .. } = &block.statements[0] {
            assert_eq!(break_label.as_deref(), Some("nje"));
        } else {
            panic!("expected Break in loop body");
        }
    } else {
        panic!("expected While as first stmt");
    }
    semantic_check(&module).expect("semantics");
}

#[test]
fn parses_labeled_break_with_quote() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { lebo 'nje: wakati milele { vunja 'nje } }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let kuu = module.functions.iter().find(|f| f.name == "kuu").unwrap();
    if let Stmt::While { label, body: block, .. } = &kuu.body.statements[0] {
        assert_eq!(label.as_deref(), Some("nje"));
        if let Stmt::Break { label: break_label, .. } = &block.statements[0] {
            assert_eq!(break_label.as_deref(), Some("nje"));
        }
    }
}

#[test]
fn parses_vunja_with_optional_label() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { wakati milele { vunja } }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let kuu = module.functions.iter().find(|f| f.name == "kuu").unwrap();
    if let Stmt::While { body: block, .. } = &kuu.body.statements[0] {
        if let Stmt::Break { label, .. } = &block.statements[0] {
            assert!(label.is_none());
        }
    }
}

#[test]
fn parses_selective_import() {
    let src = "leta mfumo::{chapisha, toka} kazi kuu(hoja: Orodha<Neno>) -> Tupu { }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    assert_eq!(module.imports.len(), 1);
    match &module.imports[0].path {
        ImportPath::Selective { module: m, names } => {
            assert_eq!(m, "mfumo");
            assert_eq!(names.as_slice(), ["chapisha", "toka"]);
        }
        _ => panic!("expected Selective import"),
    }
    semantic_check(&module).expect("semantics");
}

#[test]
fn type_from_decl_kamusi_and_others() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { weka a: Kamusi<Neno, Namba> = Hamna }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let kuu = module.functions.iter().find(|f| f.name == "kuu").unwrap();
    if let Stmt::Let { ty: Some(ty), .. } = &kuu.body.statements[0] {
        assert!(
            ty.name.replace(' ', "").starts_with("Kamusi<"),
            "expected Kamusi type in declaration"
        );
    } else {
        panic!("expected Let with type");
    }
}

#[test]
fn parses_method_call() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { weka x = \"hi\" weka n = x.urefu() }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let kuu = module.functions.iter().find(|f| f.name == "kuu").unwrap();
    if let Stmt::Let { value, .. } = &kuu.body.statements[1] {
        if let Expr::MethodCall { receiver, method_name, args, .. } = value {
            if let Expr::Ident(r) = &**receiver {
                assert_eq!(r, "x");
            } else {
                panic!("expected Ident receiver");
            }
            assert_eq!(method_name, "urefu");
            assert!(args.is_empty());
        } else {
            panic!("expected MethodCall");
        }
    }
}

#[test]
fn recursion_depth_limit_parse() {
    let n = 101;
    let open: String = "(".repeat(n);
    let close: String = ")".repeat(n);
    let src = format!("kazi kuu(hoja: Orodha<Neno>) -> Tupu {{ weka x = {}1{} }}", open, close);
    let toks = tokenize(&src).expect("tokens");
    let result = parse_tokens(&toks);
    assert!(result.is_err(), "parse should fail when recursion depth exceeded");
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|d| d.code == "PAR073"),
        "expected PAR073 (undani mno), got: {:?}",
        errs.iter().map(|d| d.code).collect::<Vec<_>>()
    );
}

#[test]
fn parses_bitwise_and_shift() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { weka x = 1 na_biti 2 weka y = 4 sogeza_kushoto 1 weka z = siyo_biti 0 }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let kuu = module.functions.iter().find(|f| f.name == "kuu").unwrap();
    let stmt = &kuu.body.statements[0];
    if let Stmt::Let { value: Expr::Binary { op: BinaryOp::BitAnd, .. }, .. } = stmt {
    } else {
        panic!("expected Let with na_biti (BitAnd)");
    }
}

#[test]
fn parses_pattern_hamna_kweli() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { linganisha Hamna { Hamna => {} kweli => {} si_kweli => {} _ => {} } }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let kuu = module.functions.iter().find(|f| f.name == "kuu").unwrap();
    let stmt = &kuu.body.statements[0];
    if let Stmt::Match { arms, .. } = stmt {
        assert_eq!(arms.len(), 4);
        assert!(matches!(&arms[0].pattern, Pattern::Literal(Expr::Hamna)));
        assert!(matches!(&arms[1].pattern, Pattern::Literal(Expr::Bool(true))));
        assert!(matches!(&arms[2].pattern, Pattern::Literal(Expr::Bool(false))));
        assert!(matches!(&arms[3].pattern, Pattern::Wildcard));
    } else {
        panic!("expected Match");
    }
}

#[test]
fn invalid_number_literal_rejected() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { weka x = 1 }";
    let mut toks = tokenize(src).expect("tokens");
    for t in &mut toks {
        if t.lexeme == "1" {
            t.lexeme = "1.2.3".to_string();
            break;
        }
    }
    let result = parse_tokens(&toks);
    assert!(result.is_err(), "parse should fail for invalid number 1.2.3");
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|d| d.code == "PAR072"),
        "expected PAR072 (namba batili), got: {:?}",
        errs.iter().map(|d| d.code).collect::<Vec<_>>()
    );
}

#[test]
fn unconsumed_tokeo_emits_sem048() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
        kazi foo() -> Tupu {
            weka r = gawio(1, 0)
            rejesha
        }
    "#;
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let mut extern_fns = HashMap::new();
    extern_fns.insert(
        "gawio".to_string(),
        FnContract {
            params: vec![ValueType::Namba, ValueType::Namba],
            ret: ValueType::Tokeo(
                Box::new(ValueType::Namba),
                Box::new(ValueType::Neno),
            ),
        },
    );
    let result = semantic_check_with_env(&module, true, extern_fns, HashMap::new());
    assert!(result.is_err(), "unconsumed Tokeo should produce SEM048");
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|d| d.code == "SEM048"),
        "expected SEM048 (Tokeo haukutumiwa), got: {:?}",
        errs.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}
