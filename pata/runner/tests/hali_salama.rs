//! Failure discipline through the standalone runner: a program's `#[hali_salama]` `kazi` runs
//! once before it stops — after an error leaves `kuu`, when the watchdog expires, when the
//! memory limit is passed — and the process ends with an error code.

use std::process::{Command, Stdio};

fn run_source(name: &str, source: &str) -> (i32, String) {
    let dir = std::env::temp_dir().join(format!("asili-hali-salama-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let tokens = asili_lexer::tokenize(source).expect("tokenize");
    let module = asili_parser::parse_tokens(&tokens).expect("parse");
    let asb = dir.join("p.asb");
    std::fs::write(&asb, asili_evaluator::emit_asb(&module, source).unwrap()).unwrap();
    let log = dir.join("p.log");
    let file = std::fs::File::create(&log).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_tenda"))
        .arg(&asb)
        .stdin(Stdio::null())
        .stdout(file.try_clone().unwrap())
        .stderr(file)
        .status()
        .unwrap();
    let out = std::fs::read_to_string(&log).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    (status.code().unwrap_or(-1), out)
}

const SAFE: &str = r#"
#[hali_salama]
kazi zima_salama() -> Tupu {
    chapisha("vali zimefungwa")
}
"#;

// Verifies: REQ-FAIL-1
#[test]
fn an_error_leaving_kuu_enters_the_safe_state() {
    let (code, out) = run_source(
        "kosa",
        &format!(
            "{SAFE}
kazi kuu(hoja: Orodha<Neno>) -> Tupu {{
    weka a = [1, 2]
    chapisha(\"kabla\")
    chapisha(a[5] kama Neno)
}}
"
        ),
    );
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("kabla"), "{out}");
    assert!(out.contains("hali salama:"), "{out}");
    assert_eq!(out.matches("vali zimefungwa").count(), 1, "{out}");
}

// Verifies: REQ-FAIL-2
#[test]
fn the_watchdog_enters_the_safe_state_when_not_fed() {
    let (code, out) = run_source(
        "mlinzi",
        &format!(
            "{SAFE}
kazi kuu(hoja: Orodha<Neno>) -> Tupu {{
    jaribu mlinzi_anza(50)
    kwa i kutoka 0 hadi 5 {{
        mlinzi_lisha()
    }}
    chapisha(\"inaendelea\")
    weka n = 0
    wakati kweli {{
        n += 1
    }}
}}
"
        ),
    );
    assert_eq!(code, 5, "{out}");
    assert!(out.contains("inaendelea"), "{out}");
    assert!(out.contains("mlinzi"), "{out}");
    assert_eq!(out.matches("vali zimefungwa").count(), 1, "{out}");
}

// Verifies: REQ-FAIL-3
#[test]
fn passing_the_memory_limit_enters_the_safe_state() {
    let (code, out) = run_source(
        "kumbukumbu",
        &format!(
            "{SAFE}
kazi kuu(hoja: Orodha<Neno>) -> Tupu {{
    kikomo_kumbukumbu(8000000)
    weka s = \"\"
    kwa i kutoka 0 hadi 1000000 {{
        s = s + \"habari dunia \"
    }}
    chapisha(\"haifiki\")
}}
"
        ),
    );
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("kikomo cha kumbukumbu"), "{out}");
    assert!(!out.contains("haifiki"), "{out}");
    assert_eq!(out.matches("vali zimefungwa").count(), 1, "{out}");
}
