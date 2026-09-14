//! Sambamba (concurrency): tenda/subiri_tenda (real OS-thread spawn/join), njia (channel),
//! fungo (mutex). See docs/design/concurrency-design.md.

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
fn tenda_spawns_and_subiri_tenda_joins() {
    let src = r#"
        leta sambamba

        kazi kazi_ya_pili() -> Tupu { }

        kazi jaribu() -> Ukweli {
            weka id = jaribu (tenda("kazi_ya_pili"))
            linganisha subiri_tenda(id) {
                Tokeo::Sawa(_) => { rejesha kweli }
                Tokeo::Kosa(_) => { rejesha si_kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn tenda_unknown_function_returns_kosa() {
    let src = r#"
        leta sambamba

        kazi jaribu() -> Ukweli {
            linganisha tenda("haipo_kabisa") {
                Tokeo::Sawa(_) => { rejesha si_kweli }
                Tokeo::Kosa(_) => { rejesha kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn subiri_tenda_on_unknown_id_returns_kosa() {
    let src = r#"
        leta sambamba

        kazi jaribu() -> Ukweli {
            linganisha subiri_tenda(999999) {
                Tokeo::Sawa(_) => { rejesha si_kweli }
                Tokeo::Kosa(_) => { rejesha kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn njia_send_and_receive_within_one_function() {
    // njia() returns a plain Jozi<NjiaTx, NjiaRx> (not Tokeo — constructing a channel can't
    // fail), so no `jaribu` around the constructor itself; .tuma()/.pokea() are the fallible
    // ones (Tokeo, since the other side may have closed).
    let src = r#"
        leta sambamba
        leta matumizi

        kazi jaribu() -> Neno {
            weka p = njia()
            weka tx = p.kwanza()
            weka rx = p.pili()
            jaribu (tx.tuma("moja kwa moja"))
            rejesha jaribu (rx.pokea())
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("moja kwa moja".to_string()));
}

#[test]
fn njia_send_and_receive_across_a_real_spawned_thread() {
    let src = r#"
        leta sambamba
        leta matumizi

        kazi mtoaji(tx: NjiaTx<Neno>) -> Tupu {
            jaribu (tx.tuma("habari kutoka kwa uzi"))
        }

        kazi jaribu() -> Neno {
            weka p = njia()
            weka tx = p.kwanza()
            weka rx = p.pili()
            weka id = jaribu (tenda("mtoaji", tx))
            weka ujumbe = jaribu (rx.pokea())
            jaribu (subiri_tenda(id))
            rejesha ujumbe
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("habari kutoka kwa uzi".to_string()));
}

#[test]
fn njia_pokea_on_closed_sender_returns_kosa() {
    // `njia()` returns Jozi<NjiaTx, NjiaRx>; the Sender only actually drops (releasing the
    // channel) once every Value holding an Arc to it is gone — Asili's evaluator deep-clones
    // Values freely and has no implicit end-of-scope drop before a function returns, so `p`
    // (which still contains the tx half via the Jozi) must be explicitly `tupa`'d, not just
    // "never read again," for the sender side to actually close. Verified live via a scratch
    // pata-cli project that omitting this `tupa` genuinely hangs .pokea() forever, matching
    // Rust's own mpsc semantics exactly (recv() blocks until every Sender is dropped).
    let src = r#"
        leta sambamba
        leta matumizi

        kazi jaribu() -> Ukweli {
            weka p = njia()
            weka rx = p.pili()
            tupa p
            linganisha rx.pokea() {
                Tokeo::Sawa(_) => { rejesha si_kweli }
                Tokeo::Kosa(_) => { rejesha kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn fungo_pata_weka_round_trip() {
    let src = r#"
        leta sambamba

        kazi jaribu() -> Namba {
            weka f = jaribu (fungo(1.0))
            f.weka(42.0)
            rejesha f.pata() kama Namba
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(42.0));
}

#[test]
fn fungo_funga_fungua_round_trip() {
    let src = r#"
        leta sambamba

        kazi jaribu() -> Tupu {
            weka f = jaribu (fungo(1.0))
            f.funga()
            f.fungua()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]);
    assert!(result.is_ok(), "matched funga/fungua should not panic");
}

#[test]
fn fungo_fungua_without_funga_panics() {
    let src = r#"
        leta sambamba

        kazi jaribu() -> Tupu {
            weka f = jaribu (fungo(1.0))
            f.fungua()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]);
    assert!(result.is_err(), "fungua without a prior funga should be reported, not silently accepted");
}

#[test]
fn kasha_gc_cannot_cross_tenda() {
    let src = r#"
        leta sambamba
        leta kasha_gc

        kazi mfano_kazi(x: Kasha_GC<Namba>) -> Tupu { }

        kazi jaribu() -> Ukweli {
            weka g = kasha_gc_unda(1.0)
            linganisha tenda("mfano_kazi", g) {
                Tokeo::Sawa(_) => { rejesha si_kweli }
                Tokeo::Kosa(_) => { rejesha kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true), "a Kasha_GC value passed to tenda must be rejected, not silently allowed");
}

#[test]
fn sambamba_requires_leta() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka id = tenda("kuu")
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    let result = semantic_check_with_env(&module, true, fns, consts);
    assert!(result.is_err(), "tenda should be unknown without `leta sambamba`");
}
