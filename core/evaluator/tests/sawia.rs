//! `sawia kazi` / `subiri`: tasks on one thread whose waits (lala, njia, mtandao) overlap.
//! See core/evaluator/src/kazi_sawia.rs.

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{extern_env_from_imports, parse_tokens, semantic_check_with_env, Module};
use std::time::Instant;

const PRELUDE: &str = r#"
    leta sambamba
    leta majira
    leta matumizi

    sawia kazi baada(sekunde: Namba, jina: Neno) -> Neno {
        lala(sekunde)
        rejesha jina
    }

    sawia kazi mraba(n: Namba) -> Namba {
        rejesha n * n
    }
"#;

fn compile(body: &str) -> Result<Module, String> {
    let src = format!("{PRELUDE}\n{body}");
    let toks = tokenize(&src).map_err(|e| format!("{e:?}"))?;
    let module = parse_tokens(&toks).map_err(|e| format!("{e:?}"))?;
    let (fns, consts) = extern_env_from_imports(&module);
    semantic_check_with_env(&module, false, fns, consts).map_err(|e| format!("{e:?}"))?;
    Ok(module)
}

fn run(body: &str) -> Value {
    let module = compile(body).expect("compiles");
    run_function(&module, "jaribu", vec![]).expect("runs")
}

fn text(v: Value) -> String {
    match v {
        Value::Neno(s) => s.to_string(),
        other => panic!("expected Neno, got {other:?}"),
    }
}

#[test]
fn tasks_overlap_their_waits() {
    let start = Instant::now();
    let out = run(r#"
        kazi jaribu() -> Neno {
            weka a = baada(0.3, "a")
            weka b = baada(0.3, "b")
            weka c = baada(0.3, "c")
            rejesha subiri a + subiri b + subiri c
        }
    "#);
    assert_eq!(text(out), "abc");
    assert!(
        start.elapsed().as_secs_f64() < 0.55,
        "{:?}",
        start.elapsed()
    );

    let start = Instant::now();
    let out = run(r#"
        kazi jaribu() -> Namba {
            weka zote = []
            kwa i kutoka 0 hadi 100 {
                zote.ongeza(baada(0.2, "x"))
            }
            rejesha subiri_zote(zote).urefu()
        }
    "#);
    assert_eq!(out, Value::Namba(100.0));
    assert!(start.elapsed().as_secs_f64() < 1.5, "{:?}", start.elapsed());

    let out = run(r#"
        kazi jaribu() -> Neno {
            weka j = subiri_yoyote([baada(0.3, "pole"), baada(0.01, "haraka")])
            rejesha (j.kwanza() kama Neno) + " " + (j.pili() kama Neno)
        }
    "#);
    assert_eq!(text(out), "1 haraka");
}

#[test]
fn values_errors_cancellation_and_timeouts() {
    assert_eq!(
        run("kazi jaribu() -> Namba { rejesha subiri mraba(7) }"),
        Value::Namba(49.0)
    );

    let module = compile(
        r#"
        sawia kazi vunja() -> Namba { paparika("imevunjika") }
        kazi jaribu() -> Namba { rejesha subiri vunja() }
    "#,
    )
    .unwrap();
    let err = run_function(&module, "jaribu", vec![]).unwrap_err();
    assert!(format!("{err:?}").contains("imevunjika"), "{err:?}");

    let out = run(r#"
        sawia kazi tokeo_la(n: Namba) -> Tokeo<Namba, Neno> {
            ikiwa n < 0 { rejesha Tokeo::Kosa("hasi") }
            rejesha Tokeo::Sawa(n)
        }
        kazi jaribu() -> Neno {
            linganisha subiri tokeo_la(-1) {
                Tokeo::Sawa(_) => { rejesha "sawa" }
                Tokeo::Kosa(e) => { rejesha e }
            }
        }
    "#);
    assert_eq!(text(out), "hasi");

    let start = Instant::now();
    let out = run(r#"
        kazi jaribu() -> Ukweli {
            weka a = baada(5, "polepole")
            a.ghairi()
            rejesha a.imekwisha()
        }
    "#);
    assert_eq!(out, Value::Ukweli(false));
    assert!(start.elapsed().as_secs_f64() < 1.0, "{:?}", start.elapsed());

    let out = run(r#"
        kazi jaribu() -> Neno {
            linganisha muda_kikomo(baada(1, "kuchelewa"), 0.05) {
                Chaguo::Kuna(v) => { rejesha v kama Neno }
                Chaguo::Hamna => { rejesha "muda umekwisha" }
            }
        }
    "#);
    assert_eq!(text(out), "muda umekwisha");
}

#[test]
fn channels_between_tasks_and_draining() {
    let out = run(r#"
        sawia kazi zalisha(tx: NjiaTxBounded<Namba>) -> Tupu {
            kwa i kutoka 0 hadi 1000 { jaribu (tx.tuma(i)) }
        }
        sawia kazi jumlisha(rx: NjiaRxBounded<Namba>) -> Namba {
            weka jumla = 0
            kwa i kutoka 0 hadi 1000 { jumla = jumla + (jaribu (rx.pokea()) kama Namba) }
            rejesha jumla
        }
        kazi jaribu() -> Namba {
            weka p = njia_na_kikomo(4)
            weka z = zalisha(p.kwanza())
            weka j = jumlisha(p.pili())
            subiri z
            rejesha subiri j
        }
    "#);
    assert_eq!(out, Value::Namba(499500.0));

    let start = Instant::now();
    run(r#"
        kazi jaribu() -> Tupu {
            baada(0.05, "peke")
        }
    "#);
    assert!(
        start.elapsed().as_secs_f64() >= 0.04,
        "{:?}",
        start.elapsed()
    );

    let out = run(r#"
        sawia kazi ndani(n: Namba) -> Namba {
            ikiwa n == 0 { rejesha 0 }
            rejesha n + subiri ndani(n - 1)
        }
        kazi jaribu() -> Namba { rejesha subiri ndani(200) }
    "#);
    assert_eq!(out, Value::Namba(20100.0));
}

#[test]
fn analyzer_rules() {
    let err = compile("kazi jaribu() -> Namba { rejesha subiri 3 }").unwrap_err();
    assert!(err.contains("SEM107"), "{err}");
    let err = compile("sawia kazi kuu() -> Tupu { }").unwrap_err();
    assert!(err.contains("SEM106"), "{err}");
}
