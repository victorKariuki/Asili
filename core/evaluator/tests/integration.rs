//! Evaluator integration tests.

use std::collections::HashMap;

use asili_evaluator::{eval_expr, run_function, run_function_with_telemetry, run_main, run_test_with_module, execute_tests, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, FnContract, Module, ValueType};

fn parse_and_check(src: &str) -> Module {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    semantic_check_with_env(&module, false, std::collections::HashMap::new(), std::collections::HashMap::new()).expect("semantic");
    module
}

/// Extern env with prelude (msingi) + matumizi so tests can use chapisha, paparika, orodha, etc.
fn test_extern_env() -> (HashMap<String, FnContract>, HashMap<String, ValueType>) {
    let mut fns = HashMap::new();
    fns.insert("orodha".to_string(), FnContract { params: vec![], ret: ValueType::Orodha(Box::new(ValueType::Unknown)) });
    fns.insert("kamusi_tupu".to_string(), FnContract { params: vec![], ret: ValueType::Kamusi(Box::new(ValueType::Unknown), Box::new(ValueType::Unknown)) });
    fns.insert("jozi".to_string(), FnContract { params: vec![ValueType::Unknown, ValueType::Unknown], ret: ValueType::Jozi(Box::new(ValueType::Unknown), Box::new(ValueType::Unknown)) });
    let neno_tupu = FnContract { params: vec![ValueType::Neno], ret: ValueType::Tupu };
    fns.insert("chapisha".to_string(), neno_tupu.clone());
    fns.insert("onyo".to_string(), neno_tupu.clone());
    fns.insert("makosa".to_string(), neno_tupu.clone());
    fns.insert("paparika".to_string(), neno_tupu);
    fns.insert("omba".to_string(), FnContract { params: vec![ValueType::Neno], ret: ValueType::Neno });
    let mut consts = HashMap::new();
    consts.insert("KWELI".to_string(), ValueType::Ukweli);
    consts.insert("SIYO_KWELI".to_string(), ValueType::Ukweli);
    (fns, consts)
}

fn parse_and_check_with_stdlib(src: &str) -> Module {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = test_extern_env();
    semantic_check_with_env(&module, false, fns, consts).expect("semantic");
    module
}

fn parse_only(src: &str) -> Module {
    let toks = tokenize(src).expect("tokenize");
    parse_tokens(&toks).expect("parse")
}

#[test]
fn asb_is_deterministic() {
    use asili_evaluator::emit_asb;
    let module = Module {
        imports: vec![],
        functions: vec![],
        structs: vec![],
        traits: vec![],
        impls: vec![],
        constants: vec![],
    };
    let a = emit_asb(&module, "abc");
    let b = emit_asb(&module, "abc");
    assert_eq!(a, b, "emit_asb bytes deterministic");
}

#[test]
fn asb_roundtrip() {
    use asili_evaluator::{emit_asb, load_asb};
    let module = parse_and_check("kazi kuu(hoja: Orodha<Neno>) -> Tupu { }");
    let bytes = emit_asb(&module, "source");
    let loaded = load_asb(&bytes).expect("load_asb");
    assert_eq!(module.functions.len(), loaded.functions.len());
}

#[test]
fn eval_expr_literals_and_arithmetic() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi add() -> Namba { rejesha 2 + 3 }",
    );
    let mut env = asili_evaluator::Env::new();
    let v = eval_expr(
        &asili_parser::Expr::Binary {
            left: Box::new(asili_parser::Expr::Number("2".into())),
            op: asili_parser::BinaryOp::Add,
            right: Box::new(asili_parser::Expr::Number("3".into())),
            line: 1,
        },
        &mut env,
        &module,
    )
    .expect("eval");
    assert_eq!(v, Value::Namba(5.0));
}

#[test]
fn run_function_returns_value() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi seven() -> Namba { rejesha 7 }",
    );
    let v = run_function(&module, "seven", vec![]).expect("run");
    assert_eq!(v, Value::Namba(7.0));
}

#[test]
fn run_function_with_telemetry_returns_peak_depth() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi seven() -> Namba { rejesha 7 }",
    );
    let (v, peak) = run_function_with_telemetry(&module, "seven", vec![]).expect("run");
    assert_eq!(v, Value::Namba(7.0));
    assert!(peak > 0, "peak depth should be tracked");
}

#[test]
fn global_constants_ukomo_siyo_namba() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi inf() -> Namba { rejesha Ukomo }
         kazi nan_val() -> Namba { rejesha Siyo_Namba }",
    );
    let v = run_function(&module, "inf", vec![]).expect("run");
    assert!(matches!(v, Value::Namba(x) if x.is_infinite() && x > 0.0));
    let v = run_function(&module, "nan_val", vec![]).expect("run");
    assert!(matches!(v, Value::Namba(x) if x.is_nan()));
}

#[test]
fn run_function_paparika_fails() {
    let module = parse_and_check_with_stdlib(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi fail_test() -> Tupu { paparika(\"test\") }",
    );
    let result = run_function(&module, "fail_test", vec![]);
    assert!(result.is_err());
    if let Err(asili_evaluator::EvalError::Panic(m)) = result {
        assert!(m.contains("test"), "panic message should contain 'test': got {}", m);
    } else {
        panic!("expected Panic error");
    }
}

#[test]
fn run_test_with_module_pass() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi pass_test() -> Tupu { rejesha }",
    );
    let f = module.functions.iter().find(|x| x.name == "pass_test").unwrap().clone();
    let result = run_test_with_module(&module, &f);
    assert!(result.passed, "{}", result.message);
}

#[test]
fn run_test_with_module_fail_on_panic() {
    let module = parse_and_check_with_stdlib(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi fail_test() -> Tupu { paparika(\"x\") }",
    );
    let f = module.functions.iter().find(|x| x.name == "fail_test").unwrap().clone();
    let result = run_test_with_module(&module, &f);
    assert!(!result.passed);
    assert!(result.message.contains("x") || result.message.contains("paparika"));
}

#[test]
fn execute_tests_runs_multiple() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi t1() -> Tupu { }
         #[jaribio] kazi t2() -> Tupu { rejesha }",
    );
    let tests: Vec<(Module, asili_parser::Function)> = module
        .functions
        .iter()
        .filter(|f| f.is_test)
        .map(|f| (module.clone(), f.clone()))
        .collect();
    let results = execute_tests(&tests, false);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.passed));
}

#[test]
fn recursion_depth_limit_eval() {
    use asili_parser::{Block, Expr, Function, Module, Stmt, TypeExpr};
    // Exceed MAX_EVAL_DEPTH (100) so we get "undani mno" before stack overflow
    let mut inner = Expr::Number("1".into());
    for _ in 0..101 {
        inner = Expr::Group(Box::new(inner));
    }
    let block = Block {
        statements: vec![Stmt::Expr {
            expr: inner,
            line: 1,
        }],
    };
    let deep_fn = Function {
        name: "deep".into(),
        params: vec![],
        return_type: TypeExpr {
            name: "Namba".into(),
        },
        body: block,
        is_test: false,
        is_public: false,
        line: 1,
        attrs: vec![],
    };
    let kuu = Function {
        name: "kuu".into(),
        params: vec![asili_parser::Param {
            name: "hoja".into(),
            ty: TypeExpr {
                name: "Orodha<Neno>".into(),
            },
            line: 1,
        }],
        return_type: TypeExpr {
            name: "Tupu".into(),
        },
        body: Block {
            statements: vec![],
        },
        is_test: false,
        is_public: false,
        line: 1,
        attrs: vec![],
    };
    let module = Module {
        imports: vec![],
        functions: vec![kuu, deep_fn],
        structs: vec![],
        traits: vec![],
        impls: vec![],
        constants: vec![],
    };
    let result = run_function(&module, "deep", vec![]);
    assert!(result.is_err(), "eval should fail when recursion depth exceeded");
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("undani mno"),
        "expected 'undani mno' in error, got: {}",
        msg
    );
}

#[test]
fn run_main_executes_kuu() {
    let module = parse_and_check_with_stdlib(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { chapisha(\"hello\") }",
    );
    run_main(&module, vec!["a".into(), "b".into()]).expect("run_main");
}

#[test]
fn mfumo_builtins_majira_and_chapisha() {
    let src = r#"leta mfumo
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka t = majira()
  chapisha("ok")
}"#;
    let module = parse_only(src);
    run_main(&module, vec![]).expect("run_main with majira and chapisha");
}

#[test]
fn chapisha_onyo_makosa_streams() {
    let src = r#"leta mfumo
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("signal")
  onyo("warn")
  makosa("err")
}"#;
    let module = parse_only(src);
    run_main(&module, vec![]).expect("chapisha, onyo, makosa run without panic");
}

#[test]
fn labeled_break_exits_outer_loop() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi labeled_break() -> Namba {
           weka j = 0
           lebo 'nje: kwa i kutoka 0 hadi 10 {
             kwa k kutoka 0 hadi 5 {
               weka j = i + k
               vunja 'nje
             }
           }
           rejesha j
         }",
    );
    let v = run_function(&module, "labeled_break", vec![]).expect("run");
    assert_eq!(v, Value::Namba(0.0), "break 'nje should exit outer after first inner iter");
}

#[test]
fn neno_urefu() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi len() -> Namba { weka s = \"hello\" rejesha s.urefu() }",
    );
    let v = run_function(&module, "len", vec![]).expect("run");
    assert_eq!(v, Value::Namba(5.0), "string content 'hello' has 5 graphemes");
}

#[test]
fn neno_urefu_grapheme() {
    let e_acute = "e\u{0301}";
    let src = format!(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu {{ }}
         kazi g() -> Namba {{ weka s = "{}" rejesha s.urefu() }}"#,
        e_acute
    );
    let module = parse_and_check(&src);
    let v = run_function(&module, "g", vec![]).expect("run");
    assert_eq!(v, Value::Namba(1.0), "string content is one grapheme (e with acute)");
}

#[test]
fn neno_biti_ngapi() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi bytes() -> Namba { weka s = \"ab\" rejesha s.biti_ngapi() }
         kazi bytes_utf8() -> Namba { weka s = \"é\" rejesha s.biti_ngapi() }",
    );
    let v = run_function(&module, "bytes", vec![]).expect("run");
    assert_eq!(v, Value::Namba(2.0), "string content 'ab' is 2 bytes");
    let v2 = run_function(&module, "bytes_utf8", vec![]).expect("run");
    assert_eq!(v2, Value::Namba(2.0), "string content 'é' is 2 bytes in UTF-8");
}

#[test]
fn orodha_ongeza() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi push_one() -> Namba { weka a = orodha(1, 2) a.ongeza(3) rejesha a.urefu() }",
    );
    let v = run_function(&module, "push_one", vec![]).expect("run");
    assert_eq!(v, Value::Namba(3.0));
}

#[test]
fn kipeo_and_mizizi() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi pow() -> Namba { weka t = kipeo(2, 3)? rejesha t }
         kazi sqrt() -> Namba { weka t = mizizi(4)? rejesha t }",
    );
    let v = run_function(&module, "pow", vec![]).expect("run");
    assert_eq!(v, Value::Namba(8.0));
    let v2 = run_function(&module, "sqrt", vec![]).expect("run");
    assert_eq!(v2, Value::Namba(2.0));
}

#[test]
fn cast_to_biti8_fallible() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi cast_ok() -> Namba { rejesha 100 kama Biti8 }
         kazi cast_fail() -> Namba { rejesha 1000 kama Biti8 }",
    );
    let v = run_function(&module, "cast_ok", vec![]).expect("run");
    assert_eq!(v, Value::Chaguo(Some(Box::new(Value::Namba(100.0)))));
    let v2 = run_function(&module, "cast_fail", vec![]).expect("run");
    assert_eq!(v2, Value::Chaguo(None), "1000 kama Biti8 should yield Chaguo(None)");
}

#[test]
fn wakati_milele_labeled_break() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi count_to_three() -> Namba {
           weka n = 0
           lebo 'out: wakati milele {
             n = n + 1
             ikiwa n >= 3 { vunja 'out }
           }
           rejesha n
         }",
    );
    let v = run_function(&module, "count_to_three", vec![]).expect("run");
    assert_eq!(v, Value::Namba(3.0));
}

#[test]
fn bitwise_and_shift_eval() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi bit_and() -> Namba { rejesha 3 na_biti 5 }
         kazi bit_shl() -> Namba { rejesha 1 sogeza_kushoto 4 }
         kazi bit_not() -> Namba { rejesha siyo_biti 0 }",
    );
    let v = run_function(&module, "bit_and", vec![]).expect("run");
    assert_eq!(v, Value::Namba(1.0), "3 & 5 == 1");
    let v = run_function(&module, "bit_shl", vec![]).expect("run");
    assert_eq!(v, Value::Namba(16.0), "1 << 4 == 16");
    let v = run_function(&module, "bit_not", vec![]).expect("run");
    assert_eq!(v, Value::Namba(-1.0), "!0 == -1 (i64)");
}

#[test]
fn linganisha_hamna_kweli() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi match_hamna() -> Namba { weka x = Hamna linganisha x { Hamna => { rejesha 1 } _ => { rejesha 0 } } }
         kazi match_kweli() -> Namba { weka x = kweli linganisha x { kweli => { rejesha 2 } si_kweli => { rejesha 0 } _ => { rejesha 0 } } }",
    );
    let v = run_function(&module, "match_hamna", vec![]).expect("run");
    assert_eq!(v, Value::Namba(1.0));
    let v = run_function(&module, "match_kweli", vec![]).expect("run");
    assert_eq!(v, Value::Namba(2.0));
}

#[test]
fn neno_unganisha_kata_tafuta() {
    let module = parse_only(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
           kazi join() -> Neno { weka s = "a" rejesha s.unganisha("-") }
           kazi slice() -> Neno { weka s = "hello" rejesha s.kata(1, 4) }
           kazi find_ok() -> Namba { weka s = "hello" weka c = s.tafuta("hello") rejesha c kama Namba }
           kazi find_none() -> Namba { weka s = "hi" weka c = s.tafuta("x") rejesha 0 }"#,
    );
    let v = run_function(&module, "join", vec![]).expect("run");
    // String content is "a", separator "-"; unganisha yields "a" + "-" = "a-"
    assert_eq!(v, Value::Neno("a-".to_string()));
    let v = run_function(&module, "slice", vec![]).expect("run");
    // "hello" content; kata(1, 4) => bytes 1..4 => "ell"
    assert_eq!(v, Value::Neno("ell".to_string()));
    let v = run_function(&module, "find_ok", vec![]).expect("run");
    // "hello".tafuta("hello") finds at index 0
    assert_eq!(v, Value::Namba(0.0));
    let v = run_function(&module, "find_none", vec![]).expect("run");
    assert_eq!(v, Value::Namba(0.0));
}

#[test]
fn orodha_ondoa_kila_mmoja() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi remove_at() -> Namba { weka a = orodha(10, 20, 30) a.ondoa(1) rejesha a.urefu() }
         kazi each_stub() -> Tupu { weka a = orodha(1, 2) a.kila_mmoja() rejesha }",
    );
    let v = run_function(&module, "remove_at", vec![]).expect("run");
    assert_eq!(v, Value::Namba(2.0), "after ondoa(1) list has 2 elements");
    let _ = run_function(&module, "each_stub", vec![]).expect("run");
}

#[test]
fn struct_literal_and_field_access() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         umbo Pika { x: Namba, y: Neno }
         kazi get_x() -> Namba { weka p = Pika { x: 3, y: \"a\" } rejesha p.x }
         kazi get_y() -> Neno { weka p = Pika { x: 1, y: \"hi\" } rejesha p.y }",
    );
    let v = run_function(&module, "get_x", vec![]).expect("run");
    assert_eq!(v, Value::Namba(3.0));
    let v = run_function(&module, "get_y", vec![]).expect("run");
    assert_eq!(v, Value::Neno("hi".to_string())); // string literal content is "hi"
}

#[test]
fn impl_method_dispatch() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         umbo Pika { x: Namba }
         shughuli ya Pika { kazi ongeza(self: Pika, d: Namba) -> Namba { rejesha self.x + d } }
         kazi run() -> Namba { weka p = Pika { x: 10 } rejesha p.ongeza(5) }",
    );
    let v = run_function(&module, "run", vec![]).expect("run");
    assert_eq!(v, Value::Namba(15.0));
}

#[test]
fn orodha_index() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi first() -> Namba { weka a = orodha(10, 20, 30) rejesha a[0]? }
         kazi second() -> Namba { weka a = orodha(5, 15, 25) rejesha a[1]? }",
    );
    assert_eq!(run_function(&module, "first", vec![]).unwrap(), Value::Namba(10.0));
    assert_eq!(run_function(&module, "second", vec![]).unwrap(), Value::Namba(15.0));
}

#[test]
fn orodha_index_returns_tokeo() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi in_bounds() -> Namba { weka a = orodha(10, 20) weka r = a[0] rejesha r? }
         kazi out_of_bounds() -> Namba { weka a = orodha(10, 20) weka r = a[10] rejesha r? }",
    );
    let v = run_function(&module, "in_bounds", vec![]).expect("run");
    assert_eq!(v, Value::Namba(10.0));
    // r? on Tokeo(Err) propagates, so the function returns that Err value
    let v = run_function(&module, "out_of_bounds", vec![]).expect("run");
    assert!(matches!(v, Value::Tokeo(Err(_))));
    if let Value::Tokeo(Err(inner)) = v {
        assert!(matches!(*inner, Value::Struct(ref n, _) if n == "KosaMipaka"));
    }
}

#[test]
fn kamusi_ingiza_pata() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi run() -> Namba { weka m = kamusi_tupu() m.ingiza(\"a\", 1) m.ingiza(\"b\", 2) weka c = m.pata(\"a\") rejesha c kama Namba }
         kazi run_none() -> Ukweli { weka m = kamusi_tupu() m.ingiza(\"x\", 10) weka c = m.pata(\"y\") linganisha c { Hamna => { rejesha kweli } _ => { rejesha si_kweli } } }",
    );
    let v = run_function(&module, "run", vec![]).expect("run");
    assert_eq!(v, Value::Namba(1.0));
    let v = run_function(&module, "run_none", vec![]).expect("run");
    assert_eq!(v, Value::Ukweli(true));
}

#[test]
fn herufi_and_jozi() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi char_val() -> Herufi { rejesha 'a' }
         kazi jozi_vals() -> Namba { weka p = jozi(1, 2) rejesha p.kwanza() + p.pili() }",
    );
    let v = run_function(&module, "char_val", vec![]).expect("run");
    assert_eq!(v, Value::Herufi('a'));
    let v = run_function(&module, "jozi_vals", vec![]).expect("run");
    assert_eq!(v, Value::Namba(3.0));
}

#[test]
fn pattern_struct_and_jozi() {
    let module = parse_only(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         umbo Pika { x: Namba }
         kazi match_struct() -> Namba { weka p = Pika { x: 42 } linganisha p { Pika { x: n } => { rejesha n } _ => { rejesha 0 } } }
         kazi match_jozi() -> Namba { weka p = jozi(10, 20) linganisha p { (a, b) => { rejesha a + b } _ => { rejesha 0 } } }",
    );
    let v = run_function(&module, "match_struct", vec![]).expect("run");
    assert_eq!(v, Value::Namba(42.0));
    let v = run_function(&module, "match_jozi", vec![]).expect("run");
    assert_eq!(v, Value::Namba(30.0));
}

#[test]
fn ordering_neno_lexicographic() {
    let module = parse_and_check(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi lt() -> Ukweli { rejesha "a" < "b" }
         kazi gt() -> Ukweli { rejesha "a" > "b" }
         kazi le() -> Ukweli { rejesha "a" <= "b" }
         kazi ge() -> Ukweli { rejesha "a" >= "b" }
         kazi eq_le() -> Ukweli { rejesha "x" <= "x" }
         kazi eq_ge() -> Ukweli { rejesha "x" >= "x" }"#,
    );
    assert_eq!(run_function(&module, "lt", vec![]).unwrap(), Value::Ukweli(true));
    assert_eq!(run_function(&module, "gt", vec![]).unwrap(), Value::Ukweli(false));
    assert_eq!(run_function(&module, "le", vec![]).unwrap(), Value::Ukweli(true));
    assert_eq!(run_function(&module, "ge", vec![]).unwrap(), Value::Ukweli(false));
    assert_eq!(run_function(&module, "eq_le", vec![]).unwrap(), Value::Ukweli(true));
    assert_eq!(run_function(&module, "eq_ge", vec![]).unwrap(), Value::Ukweli(true));
}

#[test]
fn short_circuit_or_does_not_eval_right() {
    let module = parse_only(
        r#"leta mfumo
kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
kazi or_short() -> Ukweli { rejesha kweli au paparika("right") }"#,
    );
    let v = run_function(&module, "or_short", vec![]).expect("kweli au ... must not evaluate right");
    assert_eq!(v, Value::Ukweli(true));
}

#[test]
fn short_circuit_and_does_not_eval_right() {
    let module = parse_only(
        r#"leta mfumo
kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
kazi and_short() -> Ukweli { rejesha si_kweli na paparika("right") }"#,
    );
    let v = run_function(&module, "and_short", vec![]).expect("si_kweli na ... must not evaluate right");
    assert_eq!(v, Value::Ukweli(false));
}

#[test]
fn add_assign_neno_concatenates() {
    let module = parse_and_check(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi concat() -> Neno { weka s = "a" s += "b" rejesha s }"#,
    );
    let v = run_function(&module, "concat", vec![]).expect("run");
    match &v {
        Value::Neno(s) => assert!(s.contains("a") && s.contains("b"), "+= should concatenate: {:?}", s),
        _ => panic!("expected Neno, got {:?}", v),
    }
}

/// Unary operators pushed to their limits: neg, not, bitnot, jaribu, borrow, precedence.
#[test]
fn unary_ops_stress() {
    let module = parse_only(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         kazi neg_double() -> Namba { rejesha - - 1 }
         kazi neg_zero() -> Namba { rejesha - 0 }
         kazi neg_ukomo() -> Namba { rejesha - Ukomo }
         kazi neg_siyo_namba() -> Namba { rejesha - Siyo_Namba }
         kazi neg_precedence() -> Namba { rejesha - 1 + 10 }
         kazi not_double() -> Ukweli { rejesha siyo siyo kweli }
         kazi not_triple() -> Ukweli { rejesha siyo siyo siyo kweli }
         kazi bitnot_zero() -> Namba { rejesha siyo_biti 0 }
         kazi bitnot_neg_one() -> Namba { rejesha siyo_biti (0 - 1) }
         kazi bitnot_three() -> Namba { rejesha siyo_biti 3 }
         kazi jaribu_tokeo_ok() -> Namba { rejesha jaribu gawio(10, 2) }
         kazi jaribu_chaguo_some() -> Namba { rejesha jaribu (100 kama Biti8) }
         kazi precedence_binary_minus() -> Namba { rejesha 0 - 1 }
         kazi precedence_unary_neg() -> Namba { rejesha - 1 }
         kazi borrow_imm() -> Namba { weka x = 7 rejesha azima x }
         kazi borrow_mut() -> Namba { weka x = 8 rejesha azima_tenda x }"#,
    );
    assert_eq!(run_function(&module, "neg_double", vec![]).unwrap(), Value::Namba(1.0), "- - 1");
    assert_eq!(run_function(&module, "neg_zero", vec![]).unwrap(), Value::Namba(0.0), "- 0");
    let v = run_function(&module, "neg_ukomo", vec![]).unwrap();
    assert!(matches!(v, Value::Namba(x) if x.is_infinite() && x < 0.0), "- Ukomo");
    let v = run_function(&module, "neg_siyo_namba", vec![]).unwrap();
    assert!(matches!(v, Value::Namba(x) if x.is_nan()), "- Siyo_Namba");
    assert_eq!(run_function(&module, "neg_precedence", vec![]).unwrap(), Value::Namba(9.0), "- 1 + 10");
    assert_eq!(run_function(&module, "not_double", vec![]).unwrap(), Value::Ukweli(true), "siyo siyo kweli");
    assert_eq!(run_function(&module, "not_triple", vec![]).unwrap(), Value::Ukweli(false), "siyo siyo siyo kweli");
    assert_eq!(run_function(&module, "bitnot_zero", vec![]).unwrap(), Value::Namba(-1.0), "siyo_biti 0");
    assert_eq!(run_function(&module, "bitnot_neg_one", vec![]).unwrap(), Value::Namba(0.0), "siyo_biti (-1)");
    assert_eq!(run_function(&module, "bitnot_three", vec![]).unwrap(), Value::Namba(-4.0), "siyo_biti 3 => !3 i64");
    assert_eq!(run_function(&module, "jaribu_tokeo_ok", vec![]).unwrap(), Value::Namba(5.0), "jaribu gawio(10,2)");
    assert_eq!(run_function(&module, "jaribu_chaguo_some", vec![]).unwrap(), Value::Namba(100.0), "jaribu (100 kama Biti8)");
    assert_eq!(run_function(&module, "precedence_binary_minus", vec![]).unwrap(), Value::Namba(-1.0), "0 - 1");
    assert_eq!(run_function(&module, "precedence_unary_neg", vec![]).unwrap(), Value::Namba(-1.0), "- 1");
    assert_eq!(run_function(&module, "borrow_imm", vec![]).unwrap(), Value::Namba(7.0), "azima x");
    assert_eq!(run_function(&module, "borrow_mut", vec![]).unwrap(), Value::Namba(8.0), "azima_tenda x");
}

// ── Trait dispatch ────────────────────────────────────────────────────────────

/// Two separate `shughuli ya` blocks on the same struct — both must be reachable.
/// Previously only the first block was searched.
#[test]
fn multi_impl_blocks_both_reachable() {
    let module = parse_only(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
           umbo Duara { r: Namba }
           shughuli ya Duara { kazi eneo(self: Duara) -> Namba { rejesha 3 * self.r * self.r } }
           shughuli ya Duara { kazi mzingo(self: Duara) -> Namba { rejesha 6 * self.r } }
           kazi run_eneo() -> Namba { weka d = Duara { r: 2 } rejesha d.eneo() }
           kazi run_mzingo() -> Namba { weka d = Duara { r: 2 } rejesha d.mzingo() }"#,
    );
    assert_eq!(
        run_function(&module, "run_eneo", vec![]).expect("eneo"),
        Value::Namba(12.0),
        "eneo kutoka block ya kwanza"
    );
    assert_eq!(
        run_function(&module, "run_mzingo", vec![]).expect("mzingo"),
        Value::Namba(12.0),
        "mzingo kutoka block ya pili"
    );
}

/// A trait impl (`shughuli ya Foo: Sifa`) alongside an inherent impl — both callable.
#[test]
fn trait_impl_method_reachable() {
    let module = parse_only(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
           sifa Onyesheka { }
           umbo Mtu { jina: Neno }
           shughuli ya Mtu { kazi salamu(self: Mtu) -> Neno { rejesha "habari " + self.jina } }
           shughuli ya Mtu: Onyesheka { kazi onyesha(self: Mtu) -> Neno { rejesha self.jina } }
           kazi run_salamu() -> Neno { weka m = Mtu { jina: "Amani" } rejesha m.salamu() }
           kazi run_onyesha() -> Neno { weka m = Mtu { jina: "Amani" } rejesha m.onyesha() }"#,
    );
    assert_eq!(
        run_function(&module, "run_salamu", vec![]).expect("salamu"),
        Value::Neno("habari Amani".into()),
        "njia ya kawaida"
    );
    assert_eq!(
        run_function(&module, "run_onyesha", vec![]).expect("onyesha"),
        Value::Neno("Amani".into()),
        "njia ya sifa"
    );
}

/// Inherent method takes priority over a same-named trait method.
#[test]
fn inherent_impl_takes_priority_over_trait() {
    let module = parse_only(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
           sifa Sifa { }
           umbo Kitu { thamani: Namba }
           shughuli ya Kitu { kazi pata(self: Kitu) -> Namba { rejesha self.thamani + 1 } }
           shughuli ya Kitu: Sifa { kazi pata(self: Kitu) -> Namba { rejesha self.thamani + 100 } }
           kazi run() -> Namba { weka k = Kitu { thamani: 5 } rejesha k.pata() }"#,
    );
    // inherent (+1) must win over trait (+100)
    assert_eq!(
        run_function(&module, "run", vec![]).expect("pata"),
        Value::Namba(6.0),
        "inherent lazima iwe ya kwanza"
    );
}

/// Multiple methods, multiple args, single impl block.
#[test]
fn impl_multi_method_multi_arg() {
    let module = parse_only(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
           umbo Hisabati { msingi: Namba }
           shughuli ya Hisabati {
             kazi ongeza(self: Hisabati, n: Namba) -> Namba { rejesha self.msingi + n }
             kazi zidisha(self: Hisabati, n: Namba) -> Namba { rejesha self.msingi * n }
             kazi toa(self: Hisabati, n: Namba) -> Namba { rejesha self.msingi - n }
           }
           kazi run() -> Namba {
             weka h = Hisabati { msingi: 10 }
             rejesha h.ongeza(5) + h.zidisha(3) + h.toa(2)
           }"#,
    );
    // (10+5) + (10*3) + (10-2) = 15 + 30 + 8 = 53
    assert_eq!(
        run_function(&module, "run", vec![]).expect("run"),
        Value::Namba(53.0)
    );
}

/// Calling an unknown method on a struct with an impl gives a clear Swahili error.
#[test]
fn unknown_impl_method_swahili_error() {
    let module = parse_only(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
           umbo Chombo { }
           shughuli ya Chombo { kazi kazi_moja(self: Chombo) -> Tupu { } }
           kazi run() -> Tupu { weka c = Chombo { } rejesha c.haipo() }"#,
    );
    let err = run_function(&module, "run", vec![]).expect_err("inapaswa kushindwa");
    let msg = err.to_string();
    assert!(
        msg.contains("haipo") || msg.contains("njia"),
        "ujumbe wa kosa unapaswa kutaja njia: {msg}"
    );
}

// ── si_kweli fix ──────────────────────────────────────────────────────────────

#[test]
fn ukweli_false_kama_neno_is_si_kweli() {
    let module = parse_only(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
           kazi run() -> Neno { rejesha si_kweli kama Neno }"#,
    );
    assert_eq!(
        run_function(&module, "run", vec![]).expect("run"),
        Value::Neno("si_kweli".into()),
        "si_kweli kama Neno lazima iwe 'si_kweli' si 'sikweli'"
    );
}

#[test]
fn ukweli_true_kama_neno_is_kweli() {
    let module = parse_only(
        r#"kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
           kazi run() -> Neno { rejesha kweli kama Neno }"#,
    );
    assert_eq!(
        run_function(&module, "run", vec![]).expect("run"),
        Value::Neno("kweli".into())
    );
}
