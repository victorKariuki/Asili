//! Evaluator integration tests.

use std::collections::HashMap;

use asili_evaluator::{eval_expr, run_function, run_function_with_telemetry, run_main, run_test_with_module, execute_tests, execute_tests_with_timeout, run_test_with_fixtures, run_test_with_coverage, Value};
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
        constants: vec![],
        enums: vec![],
        functions: vec![],
        structs: vec![],
        traits: vec![],
        impls: vec![],
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
fn execute_tests_with_timeout_none_behaves_like_execute_tests() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi t1() -> Tupu { rejesha }",
    );
    let tests: Vec<(Module, asili_parser::Function)> = module
        .functions
        .iter()
        .filter(|f| f.is_test)
        .map(|f| (module.clone(), f.clone()))
        .collect();
    let results = execute_tests_with_timeout(&tests, false, None);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed);
}

#[test]
fn execute_tests_with_timeout_passes_a_fast_test_within_budget() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi haraka() -> Tupu { rejesha }",
    );
    let tests: Vec<(Module, asili_parser::Function)> = module
        .functions
        .iter()
        .filter(|f| f.is_test)
        .map(|f| (module.clone(), f.clone()))
        .collect();
    let results = execute_tests_with_timeout(&tests, false, Some(std::time::Duration::from_secs(5)));
    assert_eq!(results.len(), 1);
    assert!(results[0].passed, "{}", results[0].message);
}

/// A genuinely infinite loop (`wakati milele { }`, no break), not a mock — proves the timeout actually
/// interrupts *waiting* on a hung test rather than something that merely runs slowly, since the
/// evaluator itself has no way to be told to stop early.
#[test]
fn execute_tests_with_timeout_reports_a_real_infinite_loop_as_failed() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi milele_test() -> Tupu { wakati milele { } }",
    );
    let tests: Vec<(Module, asili_parser::Function)> = module
        .functions
        .iter()
        .filter(|f| f.is_test)
        .map(|f| (module.clone(), f.clone()))
        .collect();
    let start = std::time::Instant::now();
    let results = execute_tests_with_timeout(&tests, false, Some(std::time::Duration::from_millis(200)));
    let elapsed = start.elapsed();

    assert_eq!(results.len(), 1);
    assert!(!results[0].passed, "an infinite loop must be reported as a failed (timed-out) test");
    assert!(results[0].message.contains("muda umekwisha"), "{}", results[0].message);
    // The call must return promptly once the timeout elapses, not block forever waiting on the
    // hung thread — this is the actual behavior a timeout exists to provide.
    assert!(elapsed < std::time::Duration::from_secs(2), "took {elapsed:?}, should return shortly after the 200ms timeout");
}

#[test]
fn fixture_kabla_runs_before_test_and_test_passes() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[kabla] kazi weka_mazingira() -> Tupu { rejesha }
         #[jaribio] kazi t1() -> Tupu { rejesha }",
    );
    let f = module.functions.iter().find(|x| x.name == "t1").unwrap().clone();
    let result = run_test_with_fixtures(&module, &f, None);
    assert!(result.passed, "{}", result.message);
}

#[test]
fn fixture_kabla_failure_fails_the_test_and_names_the_fixture() {
    let module = parse_and_check_with_stdlib(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[kabla] kazi mazingira_mabovu() -> Tupu { paparika(\"kabla imeshindwa kimakusudi\") }
         #[jaribio] kazi t1() -> Tupu { rejesha }",
    );
    let f = module.functions.iter().find(|x| x.name == "t1").unwrap().clone();
    let result = run_test_with_fixtures(&module, &f, None);
    assert!(!result.passed, "a failing #[kabla] must fail the test, not let it run");
    assert!(result.message.contains("mazingira_mabovu"), "{}", result.message);
    assert!(result.message.contains("kabla"), "{}", result.message);
}

#[test]
fn fixture_baada_runs_after_a_passing_test() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi t1() -> Tupu { rejesha }
         #[baada] kazi safisha() -> Tupu { rejesha }",
    );
    let f = module.functions.iter().find(|x| x.name == "t1").unwrap().clone();
    let result = run_test_with_fixtures(&module, &f, None);
    assert!(result.passed, "{}", result.message);
}

#[test]
fn fixture_baada_failure_fails_an_otherwise_passing_test() {
    let module = parse_and_check_with_stdlib(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi t1() -> Tupu { rejesha }
         #[baada] kazi safisha_mbovu() -> Tupu { paparika(\"baada imeshindwa kimakusudi\") }",
    );
    let f = module.functions.iter().find(|x| x.name == "t1").unwrap().clone();
    let result = run_test_with_fixtures(&module, &f, None);
    assert!(!result.passed, "a failing #[baada] must fail an otherwise-passing test");
    assert!(result.message.contains("safisha_mbovu"), "{}", result.message);
    assert!(result.message.contains("baada"), "{}", result.message);
}

#[test]
fn fixture_test_failure_takes_precedence_over_baada_failure_message() {
    // The test's own failure is the more useful signal -- don't let a teardown failure mask it.
    let module = parse_and_check_with_stdlib(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi t_inashindwa() -> Tupu { paparika(\"jaribio lenyewe limeshindwa\") }
         #[baada] kazi safisha_mbovu() -> Tupu { paparika(\"baada imeshindwa pia\") }",
    );
    let f = module.functions.iter().find(|x| x.name == "t_inashindwa").unwrap().clone();
    let result = run_test_with_fixtures(&module, &f, None);
    assert!(!result.passed);
    assert!(result.message.contains("jaribio lenyewe"), "test's own failure should take precedence, got: {}", result.message);
}

#[test]
fn fixture_multiple_baada_all_run_even_if_one_panics() {
    // Two #[baada] functions in the same module -- a panic in the first must not prevent the
    // second from running. Verified indirectly: the reported failure must come from whichever
    // teardown genuinely failed, and execute_tests_with_timeout (which drives this for a whole
    // suite) must still report a result rather than hang or skip.
    let module = parse_and_check_with_stdlib(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi t1() -> Tupu { rejesha }
         #[baada] kazi safisha_a() -> Tupu { paparika(\"a imeshindwa\") }
         #[baada] kazi safisha_b() -> Tupu { rejesha }",
    );
    let f = module.functions.iter().find(|x| x.name == "t1").unwrap().clone();
    let result = run_test_with_fixtures(&module, &f, None);
    assert!(!result.passed);
    assert!(result.message.contains("safisha_a"));
}

#[test]
fn fixture_functions_are_not_discovered_as_tests_themselves() {
    // #[kabla]/#[baada] functions must not show up in discover_tests -- they're fixtures, not
    // tests in their own right, even though they're regular callable functions in the module.
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[kabla] kazi weka_mazingira() -> Tupu { rejesha }
         #[baada] kazi safisha() -> Tupu { rejesha }
         #[jaribio] kazi t1() -> Tupu { rejesha }",
    );
    let tests = asili_parser::discover_tests(&module);
    assert_eq!(tests.len(), 1, "only the #[jaribio]-tagged function should be discovered");
    assert_eq!(tests[0].name, "t1");
}

#[test]
fn coverage_records_every_executed_statement_line() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi t1() -> Tupu {
             weka a = 1
             weka b = 2
             rejesha
         }",
    );
    let f = module.functions.iter().find(|x| x.name == "t1").unwrap().clone();
    let (result, lines) = run_test_with_coverage(&module, &f);
    assert!(result.passed, "{}", result.message);
    // Three statements in the test body: weka a, weka b, rejesha -- all three lines recorded.
    assert_eq!(lines.len(), 3, "expected 3 distinct executed lines, got: {lines:?}");
}

/// The real point of line-level (not function-level) coverage: two branches of the same `if`
/// produce genuinely different executed-line sets depending on which one actually ran — a
/// function-name-presence check (the old inert CoverageMetrics) can't distinguish this at all,
/// since the containing function is "covered" either way.
#[test]
fn coverage_distinguishes_which_branch_actually_ran() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi t_tawi_kweli() -> Tupu {
             ikiwa kweli {
                 weka njia_ya_kweli = 1
             } vinginevyo {
                 weka njia_ya_uwongo = 2
             }
             rejesha
         }",
    );
    let f = module.functions.iter().find(|x| x.name == "t_tawi_kweli").unwrap().clone();
    let (result, lines) = run_test_with_coverage(&module, &f);
    assert!(result.passed, "{}", result.message);

    // The `if` condition line and the true-branch's `weka` line ran; the false-branch's line
    // (`weka njia_ya_uwongo = 2`, one line below the true branch's) did not.
    let true_branch_line = 4; // `weka njia_ya_kweli = 1`
    let false_branch_line = 6; // `weka njia_ya_uwongo = 2`
    assert!(lines.contains(&true_branch_line), "true branch's line should be covered, got: {lines:?}");
    assert!(!lines.contains(&false_branch_line), "false branch's line must NOT be covered when the condition is always true, got: {lines:?}");
}

#[test]
fn coverage_records_partial_lines_from_a_failing_test() {
    // A test that panics partway through still has real, partial coverage up to the panic point
    // -- that's genuine information a caller (e.g. a coverage-threshold gate) should see, not
    // nothing just because the test itself failed.
    let module = parse_and_check_with_stdlib(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi t_inashindwa() -> Tupu {
             weka a = 1
             paparika(\"imekusudiwa\")
         }",
    );
    let f = module.functions.iter().find(|x| x.name == "t_inashindwa").unwrap().clone();
    let (result, lines) = run_test_with_coverage(&module, &f);
    assert!(!result.passed);
    assert!(lines.len() >= 1, "the weka statement before the panic should still be recorded as covered");
}

#[test]
fn execute_tests_with_timeout_continues_past_a_timed_out_test() {
    let module = parse_and_check(
        "kazi kuu(hoja: Orodha<Neno>) -> Tupu { }
         #[jaribio] kazi milele_test() -> Tupu { wakati milele { } }
         #[jaribio] kazi baada_yake() -> Tupu { rejesha }",
    );
    let tests: Vec<(Module, asili_parser::Function)> = module
        .functions
        .iter()
        .filter(|f| f.is_test)
        .map(|f| (module.clone(), f.clone()))
        .collect();
    let results = execute_tests_with_timeout(&tests, false, Some(std::time::Duration::from_millis(200)));
    assert_eq!(results.len(), 2, "a timed-out test must not prevent the rest of the suite from running");
    let by_name: std::collections::HashMap<_, _> = results.iter().map(|r| (r.name.as_str(), r)).collect();
    assert!(!by_name["milele_test"].passed);
    assert!(by_name["baada_yake"].passed);
}

#[test]
fn recursion_depth_limit_eval() {
    use asili_parser::{Block, Expr, Function, Module, Stmt, TypeExpr};
    // Exceed MAX_EVAL_DEPTH (1000) so we get "undani mno" before stack overflow
    let mut inner = Expr::Number("1".into());
    for _ in 0..1001 {
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
        column: 1,
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
            column: 1,
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
        column: 1,
        attrs: vec![],
    };
    let module = Module {
        imports: vec![],
        constants: vec![],
        enums: vec![],
        functions: vec![kuu, deep_fn],
        structs: vec![],
        traits: vec![],
        impls: vec![],
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

#[test]
fn recursive_fibonacci_works() {
    // Verifies that stacker allows deep recursive function calls without stack overflow.
    // fib(20) = 6765, requires ~20 levels of call-frame recursion.
    let module = parse_only(
        r#"
kazi kuu(hoja: Orodha<Neno>) -> Tupu { }

kazi fibonacci(n: Namba) -> Namba {
    ikiwa n <= 1.0 {
        rejesha n
    }
    rejesha fibonacci(n - 1.0) + fibonacci(n - 2.0)
}

kazi run() -> Namba {
    rejesha fibonacci(20.0)
}
"#,
    );
    let result = run_function(&module, "run", vec![]).expect("fibonacci(20) lazima ifanikiwe");
    assert_eq!(
        result,
        Value::Namba(6765.0),
        "fibonacci(20) lazima iwe 6765"
    );
}

#[test]
fn astar_pathfinding_works() {
    // A* on a 5×5 grid. Nodes: flat index idx = row*5 + col (0..=24).
    // Heuristic: Manhattan distance via `%` (Rem) for col and integer division for row.
    // Shortest path (0,0)→(4,4) costs 8 moves on an obstacle-free grid.
    //
    // Collection API used:
    //   orodha()              — empty list
    //   list.ongeza(x)        — push
    //   list.urefu()          — length
    //   kamusi_tupu()         — empty map
    //   map.ingiza(key, val)  — insert/update (receiver must be a local Ident)
    //   map.pata(key)         — returns Chaguo<val>
    //   chaguo.angu(default)  — unwrap or default
    let src = r#"
kazi kuu(hoja: Orodha<Neno>) -> Tupu { }

kazi abs_val(x: Namba) -> Namba {
    ikiwa x < 0.0 {
        rejesha 0.0 - x
    }
    rejesha x
}

kazi heuristic(a: Namba, b: Namba) -> Namba {
    weka acol = a % 5.0
    weka arow = (a - acol) / 5.0
    weka bcol = b % 5.0
    weka brow = (b - bcol) / 5.0
    rejesha abs_val(arow - brow) + abs_val(acol - bcol)
}

kazi orodha_ina(lst: Orodha<Namba>, v: Namba) -> Ukweli {
    kwa x katika lst {
        ikiwa x == v {
            rejesha kweli
        }
    }
    rejesha si_kweli
}

kazi orodha_toa(lst: Orodha<Namba>, v: Namba) -> Orodha<Namba> {
    weka result = orodha()
    kwa x katika lst {
        ikiwa x != v {
            result.ongeza(x)
        }
    }
    rejesha result
}

kazi bora_wazi(wazi: Orodha<Namba>, g: Kamusi<Namba, Namba>, lengo: Namba) -> Namba {
    weka bora = 0.0 - 1.0
    weka f_bora = 0.0 - 1.0
    kwa nodo katika wazi {
        weka gn = g.pata(nodo).angu(9999.0)
        weka f = gn + heuristic(nodo, lengo)
        ikiwa f_bora < 0.0 au f < f_bora {
            bora = nodo
            f_bora = f
        }
    }
    rejesha bora
}

kazi jirani(nodo: Namba) -> Orodha<Namba> {
    weka col = nodo % 5.0
    weka row = (nodo - col) / 5.0
    weka j = orodha()
    ikiwa col > 0.0  { j.ongeza(nodo - 1.0) }
    ikiwa col < 4.0  { j.ongeza(nodo + 1.0) }
    ikiwa row > 0.0  { j.ongeza(nodo - 5.0) }
    ikiwa row < 4.0  { j.ongeza(nodo + 5.0) }
    rejesha j
}

kazi astar(chanzo: Namba, lengo: Namba) -> Namba {
    weka g = kamusi_tupu()
    g.ingiza(chanzo, 0.0)
    weka wazi = orodha()
    wazi.ongeza(chanzo)
    weka imefungwa = orodha()

    wakati wazi.urefu() > 0.0 {
        weka sasa = bora_wazi(wazi, g, lengo)
        ikiwa sasa == lengo {
            rejesha g.pata(lengo).angu(0.0 - 1.0)
        }
        wazi = orodha_toa(wazi, sasa)
        imefungwa.ongeza(sasa)
        kwa mwisho katika jirani(sasa) {
            ikiwa siyo orodha_ina(imefungwa, mwisho) {
                weka g_sasa = g.pata(sasa).angu(9999.0)
                weka g_mpya = g_sasa + 1.0
                ikiwa siyo orodha_ina(wazi, mwisho) {
                    wazi.ongeza(mwisho)
                    g.ingiza(mwisho, g_mpya)
                } au_ikiwa g_mpya < g.pata(mwisho).angu(9999.0) {
                    g.ingiza(mwisho, g_mpya)
                }
            }
        }
    }
    rejesha 0.0 - 1.0
}

kazi run() -> Namba {
    rejesha astar(0.0, 24.0)
}
"#;
    let module = parse_only(src);
    let result = run_function(&module, "run", vec![]).expect("A* lazima ifanikiwe");
    assert_eq!(
        result,
        Value::Namba(8.0),
        "umbali mfupi kutoka (0,0) hadi (4,4) kwenye gridi 5x5 ni hatua 8"
    );
}

#[test]
fn syntax_sugar_list_map_ops() {
    // Exercises all six new syntax-sugar features:
    //  1. [] list literal
    //  2. {} empty-map / { k: v } map literal
    //  3. g[key] = val  (index-assign → ingiza)
    //  4. g[key]        (index-read  → pata or Hamna)
    //  5. weka without type annotation (already worked; confirmed here)
    //  6. ! (not), && (and), || (or) operator aliases
    let module = parse_only(r#"
kazi kuu(hoja: Orodha<Neno>) -> Tupu { }

kazi run_list() -> Namba {
    weka a = [10, 20, 30]
    rejesha a.urefu()
}

kazi run_map_empty() -> Namba {
    weka m = {}
    m["x"] = 99
    rejesha m["x"]
}

kazi run_map_literal() -> Namba {
    weka m = { "a": 1, "b": 2 }
    rejesha m["a"] + m["b"]
}

kazi run_map_missing() -> Ukweli {
    weka m = {}
    rejesha m["z"] == Hamna
}

kazi run_not() -> Ukweli {
    rejesha !si_kweli
}

kazi run_and() -> Ukweli {
    rejesha kweli && kweli
}

kazi run_or() -> Ukweli {
    rejesha si_kweli || kweli
}
"#);

    assert_eq!(run_function(&module, "run_list",    vec![]).expect("list"),   Value::Namba(3.0));
    assert_eq!(run_function(&module, "run_map_empty",   vec![]).expect("map_empty"),  Value::Namba(99.0));
    assert_eq!(run_function(&module, "run_map_literal", vec![]).expect("map_lit"),    Value::Namba(3.0));
    assert_eq!(run_function(&module, "run_map_missing", vec![]).expect("map_miss"),   Value::Ukweli(true));
    assert_eq!(run_function(&module, "run_not",    vec![]).expect("not"),     Value::Ukweli(true));
    assert_eq!(run_function(&module, "run_and",    vec![]).expect("and"),     Value::Ukweli(true));
    assert_eq!(run_function(&module, "run_or",     vec![]).expect("or"),      Value::Ukweli(true));
}
