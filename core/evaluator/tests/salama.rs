//! Strict code (`#[salama]`): programs that keep to the subset build with a worst-case bound,
//! and each rule broken is a build error naming the function and line.

use asili_evaluator::{compile_module_explained, run_function, salama, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, Module};

fn module(source: &str) -> Module {
    parse_tokens(&tokenize(source).expect("tokenize")).expect("parse")
}

fn errors(source: &str) -> String {
    compile_module_explained(&module(source)).expect_err("strict rules broken")
}

const CONTROLLER: &str = r#"
    thabiti N: Namba = 8

    #[salama]
    kazi wastani(sampuli: Orodha<Namba>) -> Namba {
        weka jumla = 0
        kwa i kutoka 0 hadi N {
            jumla += sampuli[i]
        }
        rejesha jumla / N
    }

    #[salama]
    kazi dhibiti(sampuli: Orodha<Namba>, lengo: Namba) -> Namba {
        weka kosa = lengo - wastani(sampuli)
        ikiwa kosa > 10 {
            rejesha 10
        } au_ikiwa kosa < -10 {
            rejesha -10
        }
        rejesha kosa
    }

    kazi jaribu() -> Namba {
        weka s = [0; 8]
        kwa i kutoka 0 hadi 8 {
            s[i] = i
        }
        rejesha dhibiti(s, 5)
    }
"#;

#[test]
fn strict_functions_build_with_bounds_and_run() {
    let m = module(CONTROLLER);
    let program = compile_module_explained(&m).expect("strict program builds");
    let report = salama::kagua(&m, &program).expect("within the subset");
    let names: Vec<&str> = report.iter().map(|r| r.kazi.as_str()).collect();
    assert_eq!(names, ["dhibiti", "wastani"]);
    let wastani = &report[1];
    let dhibiti = &report[0];
    // Eight passes of the loop body bound `wastani`; `dhibiti` adds it to its own.
    assert!(wastani.hatua >= 8 * 3, "{report:?}");
    assert!(dhibiti.hatua > wastani.hatua, "{report:?}");
    assert!(dhibiti.kumbukumbu >= wastani.kumbukumbu, "{report:?}");
    // 0..7 averages 3.5; 5 - 3.5 = 1.5.
    assert_eq!(
        run_function(&m, "jaribu", vec![]).unwrap(),
        Value::Namba(1.5)
    );
}

#[test]
fn unbounded_loops_are_rejected() {
    let e = errors(
        "#[salama]\nkazi f(n: Namba) -> Namba {\n    weka i = 0\n    wakati kweli {\n        i += 1\n    }\n    rejesha i\n}\n",
    );
    assert!(
        e.contains("kazi salama 'f', mstari 4") && e.contains("wakati"),
        "{e}"
    );
    let e = errors("#[salama]\nkazi f(n: Namba) -> Namba {\n    kwa i kutoka 0 hadi (n) {\n    }\n    rejesha 0\n}\n");
    assert!(e.contains("mipaka ya kitanzi"), "{e}");
    let e = errors("#[salama]\nkazi f(a: Orodha<Namba>) -> Namba {\n    kwa x katika (a) {\n    }\n    rejesha 0\n}\n");
    assert!(e.contains("katika"), "{e}");
    let e = errors(
        "#[salama]\nkazi f() -> Namba {\n    kwa i kutoka 0 hadi 4 {\n        i = 0\n    }\n    rejesha 0\n}\n",
    );
    assert!(e.contains("kigeuzi cha kitanzi 'i'"), "{e}");
}

#[test]
fn recursion_and_calls_out_of_the_subset_are_rejected() {
    let e = errors("#[salama]\nkazi f(n: Namba) -> Namba {\n    rejesha f(n - 1)\n}\n");
    assert!(e.contains("kujiita"), "{e}");
    let e = errors(
        "#[salama]\nkazi f(n: Namba) -> Namba {\n    rejesha g(n)\n}\n#[salama]\nkazi g(n: Namba) -> Namba {\n    rejesha f(n)\n}\n",
    );
    assert!(e.contains("kujiita"), "{e}");
    let e = errors("kazi g(n: Namba) -> Namba {\n    rejesha n\n}\n#[salama]\nkazi f(n: Namba) -> Namba {\n    rejesha g(n)\n}\n");
    assert!(e.contains("inaita 'g', ambayo si kazi salama"), "{e}");
}

#[test]
fn allocation_and_other_types_are_rejected() {
    let e = errors("#[salama]\nkazi f(a: Orodha<Namba>) -> Tupu {\n    a.ongeza(1)\n}\n");
    assert!(e.contains("operesheni"), "{e}");
    let e = errors("#[salama]\nkazi f(jina: Neno) -> Namba {\n    rejesha 0\n}\n");
    assert!(e.contains("aina ya 'jina' ni 'Neno'"), "{e}");
    let e = errors("#[salama]\nkazi f() -> Namba {\n    chapisha(\"x\")\n    rejesha 0\n}\n");
    assert!(e.contains("operesheni"), "{e}");
}

#[test]
fn the_safe_state_takes_nothing_and_there_is_one() {
    let e = errors("#[hali_salama]\nkazi zima(x: Namba) -> Tupu {\n}\n");
    assert!(e.contains("haichukui hoja"), "{e}");
    let e =
        errors("#[hali_salama]\nkazi a() -> Tupu {\n}\n#[hali_salama]\nkazi b() -> Tupu {\n}\n");
    assert!(e.contains("kazi 'b'") || e.contains("'b'"), "{e}");
    assert!(e.contains("kazi moja tu"), "{e}");
}
