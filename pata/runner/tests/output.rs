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

/// `tenda` threads run the program's compiled `kazi` (bytecode and native code) and report back
/// over `njia`, whether the spawning `kazi` is compiled or left to the tree-walker.
#[test]
fn threads_run_compiled_kazi() {
    let dir = std::env::temp_dir().join(format!("asili-runner-threads-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let asb = artifact(
        &dir,
        r#"
leta sambamba
leta matumizi

umbo Kazi { n: Namba }

kazi fib(n: Namba) -> Namba {
    ikiwa n < 2 {
        rejesha n
    }
    rejesha fib(n - 1) + fib(n - 2)
}

kazi mfanyakazi(tx: NjiaTx<Namba>, n: Namba) -> Tupu {
    weka k = Kazi { n: n }
    jaribu (tx.tuma(fib(k.n)))
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka p = njia()
    weka tx = p.kwanza()
    weka rx = p.pili()
    kwa i kutoka 0 hadi 4 {
        jaribu (tenda("mfanyakazi", tx, 20 + i))
    }
    weka jumla: Namba = 0
    kwa i kutoka 0 hadi 4 {
        weka x: Namba = jaribu (rx.pokea())
        jumla += x
    }
    chapisha(jumla kama Neno)
}
"#,
    );
    for aot in ["0", "1"] {
        let out = Command::new(env!("CARGO_BIN_EXE_tenda"))
            .arg(&asb)
            .env("ASILI_AOT", aot)
            .output()
            .unwrap();
        // fib(20) + fib(21) + fib(22) + fib(23)
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "64079\n",
            "ASILI_AOT={aot}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A standalone executable — this runner with a program appended (`bundle::assemble`) — runs
/// that program directly, with its native image, every argument going to `kuu`.
#[test]
fn standalone_executable_runs_its_program() {
    let dir = std::env::temp_dir().join(format!("asili-runner-bundle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = r#"
kazi mraba(n: Namba) -> Namba {
    rejesha n * n
}
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka s: Namba = 0
    kwa i kutoka 0 hadi 1000 {
        s += mraba(i)
    }
    chapisha((s kama Neno) + " " + hoja[0] + " " + (hoja.urefu() kama Neno))
}
"#;
    let tokens = asili_lexer::tokenize(source).expect("tokenize");
    let module = asili_parser::parse_tokens(&tokens).expect("parse");
    let asb = asili_evaluator::emit_asb(&module, source);
    let program = asili_evaluator::load_asb_bytecode(&asb).expect("bytecode");
    let image = asili_evaluator::nguvu::supported()
        .then(|| asili_evaluator::nguvu::write_image(&program, &dir, "p").ok())
        .flatten()
        .map(|path| std::fs::read(path).unwrap());
    let runner = std::fs::read(env!("CARGO_BIN_EXE_tenda")).unwrap();
    let exe = dir.join(format!("p{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(
        &exe,
        asili_evaluator::bundle::assemble(&runner, &asb, image.as_deref()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let out = Command::new(&exe).args(["habari", "x"]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "332833500 habari 2\n");
    assert!(out.status.success());
    let _ = std::fs::remove_dir_all(&dir);
}
