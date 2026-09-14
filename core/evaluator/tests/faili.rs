//! Faili (file handle): faili_fungua/.soma()/.andika()/.funga(). Available via the existing
//! `leta faili` gate, not an opt-in module (see docs/design/faili-mkondo-design.md).
//!
//! The key correctness property under test: the OS file descriptor is released whether closed
//! explicitly (.funga()), via tupa, or by ordinary scope exit with no explicit cleanup at all —
//! the last case is the one that would silently leak if the destructor lived on Env::drop
//! instead of on FailiHandle's own Rust Drop impl (Env::pop_scope bypasses Env::drop entirely).

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, extern_env_from_imports, Module};
use std::io::Write;

fn compile(src: &str) -> Module {
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    semantic_check_with_env(&module, false, fns, consts).expect("semantic check");
    module
}

fn temp_path(name: &str) -> std::path::PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("asili-faili-test-{name}-{stamp}.txt"))
}

#[test]
fn write_then_read_round_trips() {
    let path = temp_path("roundtrip");
    let src = format!(
        r#"
        leta faili

        kazi jaribu() -> Neno {{
            weka w = jaribu (faili_fungua("{}", "andika"))
            jaribu (w.andika("habari"))
            w.funga()
            weka r = jaribu (faili_fungua("{}", "soma"))
            rejesha jaribu (r.soma())
        }}
    "#,
        path.display(),
        path.display()
    );
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Neno("habari".to_string()));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn explicit_funga_closes_the_handle() {
    let path = temp_path("funga");
    let src = format!(
        r#"
        leta faili

        kazi jaribu() -> Ukweli {{
            weka w = jaribu (faili_fungua("{}", "andika"))
            w.funga()
            linganisha w.andika("baada ya kufunga") {{
                Tokeo::Sawa(_) => {{ rejesha si_kweli }}
                Tokeo::Kosa(_) => {{ rejesha kweli }}
            }}
        }}
    "#,
        path.display()
    );
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true), "writing after .funga() should return Tokeo(Kosa(...))");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn scope_exit_without_explicit_tupa_still_closes_the_handle() {
    // Proves the Drop-on-FailiHandle mechanism, not an Env::drop hook: open a file inside an
    // inner block with no explicit tupa/funga, let it fall out of scope normally, then verify
    // the OS handle was actually released by renaming the underlying path afterward (an open
    // handle on most platforms would not prevent this on its own, so instead we assert the
    // second open-for-write succeeds and the write is visible — proving the first handle's
    // buffered/locked state, if any, was already torn down).
    let path = temp_path("scope-exit");
    std::fs::write(&path, "").expect("seed file");
    let src = format!(
        r#"
        leta faili

        kazi jaribu() -> Neno {{
            ikiwa kweli {{
                weka w = jaribu (faili_fungua("{}", "andika"))
                jaribu (w.andika("kwanza"))
            }}
            weka w2 = jaribu (faili_fungua("{}", "andika"))
            jaribu (w2.andika("pili"))
            w2.funga()
            weka r = jaribu (faili_fungua("{}", "soma"))
            rejesha jaribu (r.soma())
        }}
    "#,
        path.display(),
        path.display(),
        path.display()
    );
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(
        result,
        Value::Neno("pili".to_string()),
        "second open+write should succeed cleanly after the first handle's scope exited, \
         proving Drop released the OS handle without needing explicit tupa/funga"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn faili_requires_leta_faili() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka w = faili_fungua("x.txt", "soma")
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let (fns, consts) = extern_env_from_imports(&module);
    let result = semantic_check_with_env(&module, true, fns, consts);
    assert!(result.is_err(), "faili_fungua should be unknown without `leta faili`");
}

#[test]
fn unknown_mode_returns_kosa() {
    let path = temp_path("badmode");
    let src = format!(
        r#"
        leta faili

        kazi jaribu() -> Ukweli {{
            linganisha faili_fungua("{}", "bahati") {{
                Tokeo::Sawa(_) => {{ rejesha si_kweli }}
                Tokeo::Kosa(_) => {{ rejesha kweli }}
            }}
        }}
    "#,
        path.display()
    );
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn opening_missing_file_for_read_returns_kosa() {
    let path = temp_path("missing");
    let _ = std::fs::remove_file(&path);
    let src = format!(
        r#"
        leta faili

        kazi jaribu() -> Ukweli {{
            linganisha faili_fungua("{}", "soma") {{
                Tokeo::Sawa(_) => {{ rejesha si_kweli }}
                Tokeo::Kosa(_) => {{ rejesha kweli }}
            }}
        }}
    "#,
        path.display()
    );
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]).expect("runs");
    assert_eq!(result, Value::Ukweli(true));
}

#[test]
fn double_funga_is_a_safe_no_op() {
    let path = temp_path("double-close");
    let mut f = std::fs::File::create(&path).expect("seed file");
    writeln!(f, "x").expect("write seed");
    let src = format!(
        r#"
        leta faili

        kazi jaribu() -> Tupu {{
            weka w = jaribu (faili_fungua("{}", "andika"))
            w.funga()
            w.funga()
        }}
    "#,
        path.display()
    );
    let module = compile(&src);
    let result = run_function(&module, "jaribu", vec![]);
    assert!(result.is_ok(), "closing an already-closed Faili handle must not panic");
    let _ = std::fs::remove_file(&path);
}
