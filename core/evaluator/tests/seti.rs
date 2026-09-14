//! Seti<T> (set): seti()/seti_tupu() constructors, .ongeza/.ina/.ondoa/.urefu/.clona/.orodha.
//! Always in scope via msingi, like Orodha/Kamusi — no `leta` needed.

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, extern_env_from_imports, Module};

fn compile(src: &str) -> Module {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    semantic_check_with_env(&module, false, fns, consts).expect("semantic check");
    module
}

#[test]
fn no_leta_needed_it_is_always_in_scope() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka s = seti_tupu()
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    let result = semantic_check_with_env(&module, true, fns, consts);
    assert!(result.is_ok(), "seti_tupu should be reachable with no `leta` at all");
}

#[test]
fn ongeza_and_urefu() {
    let src = r#"
        kazi jaribu() -> Namba {
            weka s = seti_tupu()
            s.ongeza(1)
            s.ongeza(2)
            s.ongeza(1)
            rejesha s.urefu()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(2.0), "duplicate insert should not grow the set");
}

#[test]
fn ina_checks_membership() {
    let src = r#"
        kazi jaribu() -> Ukweli {
            weka s = seti_tupu()
            s.ongeza("nairobi")
            rejesha s.ina("nairobi") na (siyo s.ina("mombasa"))
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn ondoa_removes_and_reports_presence() {
    let src = r#"
        kazi jaribu() -> Ukweli {
            weka s = seti_tupu()
            s.ongeza(5)
            weka kwanza2 = s.ondoa(5)
            weka pili2 = s.ondoa(5)
            rejesha kwanza2 na (siyo pili2)
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(
        result,
        Value::Ukweli(true),
        "removing a present member returns kweli, removing again returns si_kweli"
    );
}

#[test]
fn seti_constructor_takes_initial_members() {
    let src = r#"
        kazi jaribu() -> Namba {
            weka s = seti(1, 2, 3, 2)
            rejesha s.urefu()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(3.0));
}

#[test]
fn orodha_converts_to_a_list() {
    let src = r#"
        kazi jaribu() -> Namba {
            weka s = seti(10)
            weka l = s.orodha()
            rejesha l.urefu()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(1.0));
}
