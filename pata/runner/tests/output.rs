//! Program output through the standalone runner when stdout is not a terminal: block-buffered
//! (one `write` per 32 KiB, not per line), yet complete and in order with stderr and exits.

use std::process::{Command, Stdio};

fn artifact(dir: &std::path::Path, source: &str) -> std::path::PathBuf {
    let tokens = asili_lexer::tokenize(source).expect("tokenize");
    let module = asili_parser::parse_tokens(&tokens).expect("parse");
    let path = dir.join("p.asb");
    std::fs::write(&path, asili_evaluator::emit_asb(&module, source)).unwrap();
    path
}

/// Run `tenda` with stdout and stderr sharing one file, as `> log 2>&1` does.
fn run(asb: &std::path::Path, args: &[&str]) -> (i32, String) {
    let log = asb.with_extension("log");
    let file = std::fs::File::create(&log).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_tenda"))
        .arg(asb)
        .args(args)
        .env("ASILI_AOT", "0")
        .stdin(Stdio::null())
        .stdout(file.try_clone().unwrap())
        .stderr(file)
        .status()
        .unwrap();
    (
        status.code().unwrap_or(-1),
        std::fs::read_to_string(&log).unwrap(),
    )
}

#[test]
fn buffered_output_stays_complete_and_ordered() {
    let dir = std::env::temp_dir().join(format!("asili-runner-output-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let asb = artifact(
        &dir,
        r#"
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    kwa i kutoka 0 hadi 20000 {
        chapisha(i kama Neno)
    }
    onyo("onyo")
    chapisha("baada")
    ikiwa hoja.urefu() > 0 {
        toka(3)
    }
    weka a: Orodha<Namba> = [1]
    chapisha(a[5] kama Neno)
}
"#,
    );
    let numbers: String = (0..20000).map(|i| format!("{i}\n")).collect();

    // A runtime error: everything printed before it, then the error.
    let (code, out) = run(&asb, &[]);
    assert_ne!(code, 0);
    let printed = format!("{numbers}onyo\nbaada\n");
    assert!(
        out.starts_with(&printed),
        "{}",
        &out[out.len().saturating_sub(200)..]
    );
    assert!(
        out[printed.len()..].contains("fahirisi"),
        "{}",
        &out[printed.len()..]
    );

    // `toka`: the output is flushed before the process exits.
    let (code, out) = run(&asb, &["x"]);
    assert_eq!(code, 3);
    assert_eq!(out, format!("{numbers}onyo\nbaada\n"));
    let _ = std::fs::remove_dir_all(&dir);
}
