//! Kumbukumbu<T> (heap box): kumbukumbu_unda/.pata(). Plain owning indirection, no OS resource,
//! available via msingi (always in scope, no `leta` needed) — see
//! docs/design/faili-mkondo-design.md.

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
fn pata_returns_the_boxed_value() {
    let src = r#"
        kazi jaribu() -> Namba {
            weka k = kumbukumbu_unda(42.0)
            rejesha (k.pata()) kama Namba
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(42.0));
}

#[test]
fn no_leta_needed_it_is_always_in_scope() {
    // Unlike kasha_gc (opt-in), kumbukumbu_unda lives in msingi and needs no import at all.
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka k = kumbukumbu_unda(1.0)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    let result = semantic_check_with_env(&module, true, fns, consts);
    assert!(result.is_ok(), "kumbukumbu_unda should be reachable with no `leta` at all");
}

#[test]
fn reassigning_the_binding_does_not_affect_a_prior_pata_snapshot() {
    let src = r#"
        kazi jaribu() -> Ukweli {
            weka k = kumbukumbu_unda(1.0)
            weka kabla = k.pata()
            weka k2 = kumbukumbu_unda(2.0)
            rejesha (kabla kama Namba) == 1.0
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}
