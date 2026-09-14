//! njia_na_kikomo: bounded njia channel. See docs/design/concurrency-design.md's njia section
//! and sambamba.rs's `njia_na_kikomo` builtin — same .tuma()/.pokea() contract as the unbounded
//! `njia()`, except `.tuma()` blocks once the bound is full instead of growing memory without
//! limit.

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
fn njia_na_kikomo_send_and_receive_within_one_function() {
    let src = r#"
        leta sambamba
        leta matumizi

        kazi jaribu() -> Neno {
            weka p = njia_na_kikomo(2.0)
            weka tx = p.kwanza()
            weka rx = p.pili()
            jaribu (tx.tuma("moja"))
            rejesha jaribu (rx.pokea())
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("moja".to_string()));
}

#[test]
fn njia_na_kikomo_send_across_a_real_spawned_thread() {
    let src = r#"
        leta sambamba
        leta matumizi

        kazi mtoaji(tx: NjiaTxBounded<Neno>) -> Tupu {
            jaribu (tx.tuma("habari kutoka kwa uzi"))
        }

        kazi jaribu() -> Neno {
            weka p = njia_na_kikomo(1.0)
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

/// Proves the bound is actually enforced end-to-end (not just that the constructor accepts a
/// capacity argument): a producer thread sends 3 items into a bound-1 channel faster than the
/// consumer drains (an artificial `lala` delay between each `.pokea()`); every item must still
/// arrive, in order, exactly once. `std::sync::mpsc::SyncSender::send` is what actually provides
/// the blocking guarantee (well-tested in Rust's own standard library) — what this test verifies
/// is that Asili's `njia_na_kikomo`/`.tuma()`/`.pokea()` wiring doesn't drop, reorder, or corrupt
/// items when the producer is forced to wait, which a naive/incorrect implementation could get
/// wrong (e.g. losing an item on a would-block path).
#[test]
fn njia_na_kikomo_delivers_every_item_in_order_under_backpressure() {
    let src = r#"
        leta sambamba
        leta matumizi
        leta majira

        kazi mfanyakazi_wa_kutuma(tx: NjiaTxBounded<Namba>) -> Tupu {
            jaribu (tx.tuma(1.0))
            jaribu (tx.tuma(2.0))
            jaribu (tx.tuma(3.0))
        }

        kazi jaribu() -> Neno {
            weka p = njia_na_kikomo(1.0)
            weka tx = p.kwanza()
            weka rx = p.pili()

            weka id = jaribu (tenda("mfanyakazi_wa_kutuma", tx))

            // Slow consumer: the producer can send at most one unread item ahead at a time on a
            // bound-1 channel, so this delay forces the producer to actually block on .tuma()
            // for its 2nd and 3rd sends rather than buffering all three up front.
            weka matokeo = ""
            weka i = 0
            wakati i < 3 {
                weka v = jaribu (rx.pokea())
                matokeo = matokeo + (v kama Neno)
                lala(0.03)
                i = i + 1
            }
            jaribu (subiri_tenda(id))
            rejesha matokeo
        }
    "#;
    let module = compile(src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("123".to_string()), "all 3 items must arrive, in order, exactly once");
}
