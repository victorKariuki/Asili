//! Parser integration tests: parse_tokens, semantic_check, discover_tests.

use std::collections::HashMap;

use asili_lexer::tokenize;
use asili_parser::{
    discover_tests, parse_tokens, semantic_check, semantic_check_with_env,
    semantic_check_with_options, BinaryOp, Expr, FnContract, ImportPath, Pattern, Stmt, ValueType,
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
            ..Default::default()
        },
    );
    semantic_check_with_env(&module, true, extern_fns, HashMap::new()).expect("semantics");
}

#[test]
fn supports_grouped_weka_declarations() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { weka x = 1.0, y = x + 1.0 }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    semantic_check(&module).expect("semantics");
}

#[test]
fn builtin_stdlib_is_ambient() {
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha((sakafu(3.7)) kama Neno) }";
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    semantic_check(&module).expect("ambient builtin stdlib");
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
    if let Stmt::While {
        label, body: block, ..
    } = &body[0]
    {
        assert_eq!(label.as_deref(), Some("nje"));
        assert_eq!(block.statements.len(), 1);
        if let Stmt::Break {
            label: break_label, ..
        } = &block.statements[0]
        {
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
    if let Stmt::While {
        label, body: block, ..
    } = &kuu.body.statements[0]
    {
        assert_eq!(label.as_deref(), Some("nje"));
        if let Stmt::Break {
            label: break_label, ..
        } = &block.statements[0]
        {
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
        if let Expr::MethodCall {
            receiver,
            method_name,
            args,
            ..
        } = &module[*value]
        {
            if let Expr::Ident { name: r, .. } = &module[*receiver] {
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
    let n = 1001;
    let open: String = "(".repeat(n);
    let close: String = ")".repeat(n);
    let src = format!(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu {{ weka x = {}1{} }}",
        open, close
    );
    let toks = tokenize(&src).expect("tokens");
    let result = parse_tokens(&toks);
    assert!(
        result.is_err(),
        "parse should fail when recursion depth exceeded"
    );
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
    if let Stmt::Let { value, .. } = stmt {
        assert!(matches!(
            &module[*value],
            Expr::Binary {
                op: BinaryOp::BitAnd,
                ..
            }
        ));
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
        assert!(matches!(
            &arms[1].pattern,
            Pattern::Literal(Expr::Bool(true))
        ));
        assert!(matches!(
            &arms[2].pattern,
            Pattern::Literal(Expr::Bool(false))
        ));
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
    assert!(
        result.is_err(),
        "parse should fail for invalid number 1.2.3"
    );
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
            ret: ValueType::Tokeo(Box::new(ValueType::Namba), Box::new(ValueType::Neno)),
            ..Default::default()
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

#[test]
fn method_call_type_checking_is_implemented() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka x = "hi"
            weka n = x.urefu()
        }
    "#;
    let toks = tokenize(src).expect("tokens");
    let module = parse_tokens(&toks).expect("parse");
    let kuu = module.functions.iter().find(|f| f.name == "kuu").unwrap();

    if let Stmt::Let { value, .. } = &kuu.body.statements[1] {
        if let Expr::MethodCall {
            receiver,
            method_name,
            args,
            ..
        } = &module[*value]
        {
            assert_eq!(method_name, "urefu");
            assert!(args.is_empty());
            assert!(matches!(&module[*receiver], Expr::Ident { .. }));
        } else {
            panic!("expected MethodCall");
        }
    } else {
        panic!("expected Let with method call");
    }
}

#[test]
fn umbo_names_resolve_inside_generic_types() {
    // `n` is a `Nukta` (not an unknown or a `Namba`), so `n.x` checks.
    let src = r#"
umbo Nukta {
    x: Namba,
    y: Namba,
}
kazi jumla_ya(ns: Orodha<Nukta>) -> Namba {
    weka s: Namba = 0
    kwa n katika ns {
        s += n.x + n.y
    }
    rejesha s
}
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka ns: Orodha<Nukta> = []
    ns.ongeza(Nukta { x: 1, y: 2 })
    chapisha(jumla_ya(ns) kama Neno)
}
"#;
    let module = parse_tokens(&tokenize(src).unwrap()).unwrap();
    semantic_check(&module).expect("fields of a umbo inside Orodha<...>");
}

#[test]
fn trailing_commas_semicolons_and_repeat_lists() {
    let src = r#"
umbo Nukta {
    x: Namba,
    y: Namba,
}
jenum Rangi {
    Nyekundu,
    Kijani,
}
kazi jumla(
    a: Namba,
    b: Namba,
) -> Namba {
    rejesha a + b
}
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka a = [1, 2, 3,]
    weka b = 5; weka c = 6;
    weka m = { "x": 1, "y": 2, }
    weka p = Nukta { x: 1, y: 2, }
    weka sifuri: Orodha<Namba> = [0; 500];
    chapisha(jumla(
        p.x,
        p.y,
    ) kama Neno);
};
"#;
    let module = parse_tokens(&tokenize(src).unwrap()).expect("parses");
    semantic_check(&module).expect("checks");
    let kuu = module.functions.iter().find(|f| f.name == "kuu").unwrap();
    // `;` separates statements but adds none.
    assert_eq!(kuu.body.statements.len(), 7);
    // `[0; 500]` is `orodha_rudia(0, 500)`.
    let Stmt::Let { value, .. } = &kuu.body.statements[5] else {
        panic!("expected a weka");
    };
    match &module[*value] {
        Expr::Call { callee, args, .. } => {
            assert!(matches!(&module[*callee], Expr::Ident { name, .. } if name == "orodha_rudia"));
            assert_eq!(args.len(), 2);
        }
        other => panic!("expected orodha_rudia call, got {other:?}"),
    }
    let Stmt::Let { value, .. } = &kuu.body.statements[0] else {
        panic!("expected a weka");
    };
    match &module[*value] {
        Expr::List { elements, .. } => assert_eq!(elements.len(), 3),
        other => panic!("expected a list, got {other:?}"),
    }
}

#[test]
fn one_syntax_error_per_mistake() {
    // Each broken function is reported once; the parser resumes at the next item instead of
    // flagging every remaining token.
    let src = "kazi mbaya() -> Namba {\n    weka a = (1 + \n    rejesha a\n}\n\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    weka b = 5 +* 3\n    chapisha(\"x\")\n}\n";
    let errors = parse_tokens(&tokenize(src).unwrap()).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
}

#[test]
fn deep_nesting_needs_no_machine_stack() {
    // The parser keeps nesting on heap stacks: 900 nested brackets and 900 nested blocks parse
    // on a thread with a 512 KiB stack (a recursive descent parser needs megabytes for this).
    let depth = 900;
    let expr = format!("{}1{}", "[(".repeat(depth / 2), ")]".repeat(depth / 2));
    let blocks = format!(
        "{}weka y = 2{}",
        "ikiwa kweli {\n".repeat(depth),
        "}\n".repeat(depth)
    );
    let src = format!("kazi kuu(hoja: Orodha<Neno>) -> Tupu {{\n weka x = {expr}\n {blocks}\n}}\n");
    let parsed = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            let module = parse_tokens(&tokenize(&src).unwrap()).map(|m| m.functions.len());
            // Dropping the deep tree is recursive (a property of the tree, not the parser).
            std::mem::forget(src);
            module
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(parsed, Ok(1));
}

#[test]
fn recovery_reports_each_broken_block_and_resumes_after_semicolon() {
    // An error skips to the end of its own block (or the next `;`), not the whole function.
    let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    ikiwa kweli {\n        weka a = 1 +* 2\n        chapisha(a)\n    }\n    weka b = (3\n    chapisha(b)\n}\n\nkazi f() -> Tupu { weka c = +; weka d = 1 +* 2 }\n";
    let errors = parse_tokens(&tokenize(src).unwrap()).unwrap_err();
    let codes: Vec<_> = errors.iter().map(|d| d.code).collect();
    assert_eq!(
        codes,
        ["PAR071", "PAR070", "PAR071", "PAR071"],
        "{errors:?}"
    );
}

#[test]
fn input_ending_mid_expression_is_an_error_not_a_crash() {
    for src in [
        "kazi f() -> Tupu { weka x = g(",
        "kazi f() -> Tupu { weka x = a.",
        "kazi f() -> Tupu { weka x = [1, ",
        "kazi f() -> Tupu { linganisha x { Jenum::A(",
        "kazi f() -> Tupu { weka x = ikiwa a { 1 } vinginevyo",
    ] {
        assert!(parse_tokens(&tokenize(src).unwrap()).is_err(), "{src}");
    }
}

#[test]
fn unknown_builtin_method_is_a_compile_error() {
    let check = |body: &str| {
        let src = format!(
            "kazi kuu(hoja: Orodha<Neno>) -> Tupu {{\n    weka a: Orodha<Namba> = [3, 1]\n    {body}\n}}\n"
        );
        let tokens = tokenize(&src).expect("tokenize");
        let module = parse_tokens(&tokens).expect("parse");
        semantic_check(&module)
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|d| (d.code, d.message))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        check("weka b = a.pangilia()"),
        vec![("SEM040", "njia 'pangilia' haipo kwa 'Orodha'".to_string())]
    );
    assert!(check("weka n = a.urefu()").is_empty());
    assert!(check("a.ongeza(2)").is_empty());
}
