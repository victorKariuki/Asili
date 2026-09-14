//! Kasha_GC<T>: reference-counted shared wrapper (opt-in `leta kasha_gc`).
//!
//! Sharing is explicit via `.shirikisha()` (mirrors Rust's `Rc::clone`) rather than bare
//! `weka b = a`, which the analyzer's move-checker still treats as consuming `a` — consistent
//! with the rest of the ownership model (Kasha_GC is "layered on top of," not a replacement for,
//! move semantics; see spec's roadmap).

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
fn two_handles_share_a_mutation() {
    let src = r#"
        leta kasha_gc

        kazi thamani() -> Namba {
            weka a = kasha_gc_unda(1.0)
            weka b = a.shirikisha()
            b.weka(42.0)
            rejesha (a.pata()) kama Namba
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "thamani", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(42.0), "mutation through b should be visible through a");
}

#[test]
fn refcount_increases_on_share_and_decreases_on_drop() {
    let src = r#"
        leta kasha_gc

        kazi baada_ya_kutupa() -> Namba {
            weka a = kasha_gc_unda(1.0)
            weka b = a.shirikisha()
            tupa b
            rejesha a.idadi()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "baada_ya_kutupa", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(1.0), "refcount should return to 1 after the shared handle is dropped");
}

#[test]
fn refcount_reflects_two_live_shares() {
    let src = r#"
        leta kasha_gc

        kazi idadi_ya_kushiriki() -> Namba {
            weka a = kasha_gc_unda(1.0)
            weka b = a.shirikisha()
            weka c = a.shirikisha()
            rejesha a.idadi()
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "idadi_ya_kushiriki", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(3.0), "a, b, and c should all count toward the same refcount");
}

#[test]
fn pata_returns_a_snapshot_not_a_live_view() {
    let src = r#"
        leta kasha_gc

        kazi picha() -> Ukweli {
            weka a = kasha_gc_unda(1.0)
            weka kabla = a.pata()
            a.weka(99.0)
            rejesha (kabla kama Namba) == (a.pata() kama Namba)
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "picha", vec![]).expect("runs");
    assert_eq!(
        result,
        Value::Ukweli(false),
        ".pata() should snapshot, not alias, the inner value — the pre-mutation snapshot must differ from the post-mutation read"
    );
}

#[test]
fn kasha_gc_requires_explicit_import() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka a = kasha_gc_unda(1.0)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    let result = semantic_check_with_env(&module, true, fns, consts);
    assert!(result.is_err(), "kasha_gc_unda should be unknown without `leta kasha_gc`");
}

#[test]
fn equality_is_by_shared_identity_not_contents() {
    let src = r#"
        leta kasha_gc

        kazi sawa_kwa_kumbukumbu() -> Ukweli {
            weka a = kasha_gc_unda(1.0)
            weka b = a.shirikisha()
            weka c = kasha_gc_unda(1.0)
            rejesha (a == b) na (siyo (a == c))
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "sawa_kwa_kumbukumbu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true), "== should compare shared identity, not contents");
}

#[test]
fn weka_mutates_through_any_live_handle() {
    let src = r#"
        leta kasha_gc

        kazi hesabu() -> Namba {
            weka a = kasha_gc_unda(0.0)
            weka b = a.shirikisha()
            weka c = b.shirikisha()
            c.weka(7.0)
            rejesha ((a.pata()) kama Namba) + ((b.pata()) kama Namba)
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "hesabu", vec![]).expect("runs");
    assert_eq!(result, Value::Namba(14.0), "a and b should both see c's mutation");
}

#[test]
fn dhaifu_imarisha_upgrades_while_strong_handle_is_alive() {
    // Pattern-destructuring Chaguo::Kuna(kgc) and then calling a KashaGC method on `kgc` hits a
    // pre-existing, unrelated analyzer limitation (standard_enums()'s Chaguo<T> declares Kuna's
    // payload as the literal placeholder type T, not substituted against the real inner type at
    // the pattern site — SEM039 "aina 'T' haina njia") — so this checks upgrade success/failure
    // structurally instead of destructuring the payload, which exercises .imarisha() itself
    // without depending on that separate gap.
    let src = r#"
        leta kasha_gc

        kazi jaribu() -> Ukweli {
            weka a = kasha_gc_unda(9.0)
            weka d = kasha_gc_dhaifu(a)
            linganisha d.imarisha() {
                Hamna => { rejesha si_kweli }
                _ => { rejesha kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true), ".imarisha() should upgrade to Kuna(...) while a is still alive");
}

#[test]
fn dhaifu_imarisha_fails_once_every_strong_handle_is_dropped() {
    let src = r#"
        leta kasha_gc

        kazi jaribu() -> Ukweli {
            weka a = kasha_gc_unda(1.0)
            weka d = kasha_gc_dhaifu(a)
            tupa a
            linganisha d.imarisha() {
                Hamna => { rejesha kweli }
                _ => { rejesha si_kweli }
            }
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true), ".imarisha() must return Hamna once the strong count hits zero");
}

#[test]
fn weak_reference_does_not_count_toward_strong_refcount() {
    // The motivating scenario for kasha_gc_dhaifu(): a "child" holds a strong reference forward,
    // and a "parent" would normally hold a strong reference back — but a strong/strong cycle
    // leaks permanently (no collector exists). This test proves the load-bearing property that
    // makes the weak back-reference actually break such a cycle: downgrading to a Dhaifu must
    // NOT itself increment the strong count the way .shirikisha() (a second strong handle)
    // would — otherwise "weak" would be nominal only, and the cycle would still never reach zero.
    let src = r#"
        leta kasha_gc

        kazi jaribu() -> Ukweli {
            weka a = kasha_gc_unda(1.0)
            weka b = a.shirikisha()
            weka idadi_kabla = a.idadi()
            weka dhaifu = kasha_gc_dhaifu(b)
            weka idadi_baada = a.idadi()
            rejesha idadi_kabla == idadi_baada
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true), "downgrading to Dhaifu must not increment the strong count");
}
