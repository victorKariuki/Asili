//! The Cortex-M target computes what native code on the build machine computes, bit for bit:
//! strict functions are built for the device (`nguvu::device`), linked with a generated C
//! harness and the device runtime (`driver/kifaa`), run under `qemu-arm -cpu cortex-m7`, and
//! every result — or failure — is compared with the same call made through native code here.
//!
//! Needs `clang`, `ld.lld`, `qemu-arm-static` and the `thumbv7em-none-eabihf` Rust target;
//! skipped (with a note) where any is missing. CI installs them (`.github/workflows/ci.yml`).

use asili_evaluator::nguvu::{self, device};
use asili_evaluator::{compile_module_explained, run_bytecode_function_on, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "support/strict_gen.rs"]
mod strict_gen;

/// An argument of a test call.
#[derive(Clone)]
enum Arg {
    Num(f64),
    List(Vec<f64>),
}

fn tool(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The device runtime, built for Cortex-M in its own target directory (the test's own build
/// holds the workspace's lock).
fn runtime() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::temp_dir().join("asili-kifaa-target");
    let status = Command::new(env!("CARGO"))
        .args([
            "build",
            "--release",
            "-q",
            "-p",
            "asili-kifaa",
            "--target",
            "thumbv7em-none-eabihf",
        ])
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .status()
        .ok()?;
    status
        .success()
        .then(|| target.join("thumbv7em-none-eabihf/release/libasili_kifaa.a"))
}

/// What a call gives: the result's bits, or a failure.
fn host_result(
    library: &asili_evaluator::aot::NativeLibrary,
    program: &asili_evaluator::BytecodeProgram,
    name: &str,
    args: &[Arg],
) -> String {
    let values = args
        .iter()
        .map(|a| match a {
            Arg::Num(n) => Value::Namba(*n),
            Arg::List(items) => Value::Orodha(std::rc::Rc::new(
                items.iter().map(|&n| Value::Namba(n)).collect(),
            )),
        })
        .collect();
    match run_bytecode_function_on(library, program, name, values) {
        Ok(Value::Namba(n)) => format!("{:016x}", canon(n)),
        Ok(Value::Ukweli(b)) => format!("{:016x}", (b as u8 as f64).to_bits()),
        Ok(other) => format!("{other:?}"),
        Err(_) => "kosa".to_string(),
    }
}

/// Every NaN prints alike (their sign and payload bits are not observable in Asili).
fn canon(n: f64) -> u64 {
    if n.is_nan() {
        f64::NAN.to_bits()
    } else {
        n.to_bits()
    }
}

const SYS: &str = "
    .syntax unified
    .thumb
    .global sys3
    .type sys3, %function
    .thumb_func
sys3:
    push {r7, lr}
    mov r7, r0
    mov r0, r1
    mov r1, r2
    mov r2, r3
    svc #0
    pop {r7, pc}
";

/// Build `source`'s strict functions for the device, run `calls` there and here, and compare.
fn agree(name: &str, source: &str, calls: &[(&str, Vec<Arg>)]) {
    // CI sets ASILI_KIFAA_REQUIRE so a missing tool fails the job instead of skipping it.
    let skip = |why: String| {
        assert!(std::env::var_os("ASILI_KIFAA_REQUIRE").is_none(), "{why}");
        eprintln!("kifaa: {why}; jaribio limerukwa");
    };
    let tools = ["clang", "ld.lld", "qemu-arm-static"];
    if let Some(missing) = tools.iter().find(|t| !tool(t)) {
        return skip(format!("{missing} haipo"));
    }
    let Some(runtime) = runtime() else {
        return skip("lengo thumbv7em-none-eabihf halipo".into());
    };
    let tokens = asili_lexer::tokenize(source).expect("tokenize");
    let module = asili_parser::parse_tokens(&tokens).expect("parse");
    let program = compile_module_explained(&module).expect("bytecode");
    let strict: Vec<usize> = module
        .functions
        .iter()
        .filter(|f| asili_evaluator::salama::is_strict(f))
        .map(|f| {
            program
                .functions
                .iter()
                .position(|g| g.name == f.name)
                .expect("compiled")
        })
        .collect();
    let built = device::build(&program, &strict, name).unwrap_or_else(|e| panic!("{e}"));
    let library = nguvu::compile(&program).expect("native code");

    let dir = std::env::temp_dir().join(format!("asili-kifaa-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{name}-cortex-m.o")), &built.object).unwrap();
    std::fs::write(dir.join(format!("{name}.h")), &built.header).unwrap();
    std::fs::write(dir.join("sys.s"), SYS).unwrap();

    // The harness: each call with fresh copies of its lists, printing the result's bits.
    let mut c = format!(
        "#include \"{name}.h\"\n\
         long sys3(long n, long a, long b, long c);\n\
         static void put(const char *s, int n) {{ sys3(4, 1, (long)s, n); }}\n\
         static void hex(const void *p) {{\n\
             unsigned long long v = *(const unsigned long long *)p; char b[17];\n\
             for (int i = 15; i >= 0; i--) {{ b[i] = \"0123456789abcdef\"[v & 15]; v >>= 4; }}\n\
             b[16] = '\\n'; put(b, 17);\n\
         }}\n\
         static void result(int s, double out) {{\n\
             if (s != 0) {{ put(\"kosa\\n\", 5); return; }}\n\
             if (out != out) {{ out = __builtin_nan(\"\"); }}\n\
             hex(&out);\n\
         }}\n\
         void _start(void) {{\n"
    );
    let mut expected = String::new();
    for (k, (f, args)) in calls.iter().enumerate() {
        expected.push_str(&host_result(&library, &program, f, args));
        expected.push('\n');
        let mut fields = Vec::new();
        for (j, a) in args.iter().enumerate() {
            match a {
                Arg::Num(n) => {
                    fields.push(format!("__builtin_bit_cast(double, {}ULL)", n.to_bits()))
                }
                Arg::List(items) => {
                    let elems: Vec<String> = items
                        .iter()
                        .map(|n| format!("__builtin_bit_cast(double, {}ULL)", n.to_bits()))
                        .collect();
                    c.push_str(&format!(
                        "    static double l{k}_{j}[] = {{ {} }};\n",
                        if elems.is_empty() {
                            "0".to_string()
                        } else {
                            elems.join(", ")
                        }
                    ));
                    fields.push(format!("{{ l{k}_{j}, {} }}", items.len()));
                }
            }
        }
        let hoja = if args.is_empty() {
            "0".to_string()
        } else {
            c.push_str(&format!(
                "    asili_{f}_hoja h{k} = {{ {} }};\n",
                fields.join(", ")
            ));
            format!("&h{k}")
        };
        c.push_str(&format!(
            "    {{ double out = 0; int s = asili_{f}({hoja}, &out); result(s, out); }}\n"
        ));
    }
    c.push_str("    sys3(1, 0, 0, 0);\n}\n");
    std::fs::write(dir.join("main.c"), c).unwrap();

    let run = |cmd: &mut Command| {
        let out = cmd.current_dir(&dir).output().expect("tool runs");
        assert!(
            out.status.success(),
            "{cmd:?}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let flags = [
        "--target=thumbv7em-none-eabihf",
        "-mfpu=fpv5-d16",
        "-mfloat-abi=hard",
        "-O1",
        "-ffreestanding",
        "-c",
    ];
    run(Command::new("clang")
        .args(flags)
        .args(["main.c", "-o", "main.o"]));
    run(Command::new("clang")
        .args(flags)
        .args(["sys.s", "-o", "sys.o"]));
    run(Command::new("ld.lld")
        .args(["-static", "-e", "_start", "main.o", "sys.o"])
        .arg(format!("{name}-cortex-m.o"))
        .arg(&runtime)
        .args(["-o", "t.elf"]));
    let got = run(Command::new("qemu-arm-static").args(["-cpu", "cortex-m7", "t.elf"]));
    if std::env::var_os("ASILI_KIFAA_SHOW").is_some() {
        eprintln!("{name}:\n{got}");
    }
    for (i, (want, have)) in expected.lines().zip(got.lines()).enumerate() {
        assert_eq!(
            want, have,
            "{name}: call {i} ({}) differs on the device\n{source}",
            calls[i].0
        );
    }
    assert_eq!(expected.lines().count(), got.lines().count(), "{got}");
    let _ = std::fs::remove_dir_all(&dir);
}

fn n(x: f64) -> Arg {
    Arg::Num(x)
}

// Verifies: REQ-TGT-2, REQ-RUN-1
#[test]
fn the_controller_example_agrees() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/kidhibiti/src/kuu.as"
    ))
    .unwrap();
    let samples = |base: f64| Arg::List((0..8).map(|i| base + i as f64 * 1.5).collect());
    agree(
        "kidhibiti",
        &source,
        &[
            ("wastani", vec![samples(0.0)]),
            ("dhibiti", vec![samples(0.0), n(5.0)]),
            ("dhibiti", vec![samples(-30.0), n(5.0)]),
            ("dhibiti", vec![samples(40.0), n(5.0)]),
            ("dhibiti", vec![samples(1e300), n(-0.0)]),
            ("dhibiti", vec![samples(0.0), n(f64::NAN)]),
            // Too short a list: an index out of range fails on both.
            ("wastani", vec![Arg::List(vec![1.0, 2.0])]),
        ],
    );
}

// Verifies: REQ-TGT-2
#[test]
fn integers_bits_and_calls_agree() {
    let source = r#"
        #[salama]
        kazi hatua(x: Namba, k: Namba) -> Namba {
            weka jumla = 0
            kwa i kutoka 0 hadi 40 {
                jumla += (x * i) // (k + 1) + (x * i) % (k + 3)
                ikiwa jumla > 100000000000 { jumla -= 99999999999 }
            }
            rejesha jumla
        }

        #[salama]
        kazi biti(a: Namba, b: Namba) -> Namba {
            weka r = 0
            kwa s kutoka 0 hadi 64 {
                r = r ^ ((a << s) | (b >> s)) & 4294967295
                r += (a & b) + (a | 3) - (b ^ 5)
            }
            rejesha r
        }

        #[salama]
        kazi mraba(x: Namba) -> Namba {
            rejesha x * x
        }

        #[salama]
        kazi mchanganyiko(x: Namba, y: Namba) -> Namba {
            weka a = mraba(x) + mraba(y)
            ikiwa a > 50 && x < y {
                rejesha sakafu(a / 3) + dari(y / 7) + x % 4 + y ** 2
            }
            rejesha a - 1
        }

        #[salama]
        kazi ni_kubwa(x: Namba) -> Ukweli {
            rejesha x > 10
        }

        #[salama]
        kazi panga(l: Orodha<Namba>) -> Namba {
            weka jumla = 0
            kwa i kutoka 0 hadi 6 {
                kwa j kutoka 0 hadi 5 {
                    ikiwa l[j] > l[j + 1] {
                        weka t = l[j]
                        l[j] = l[j + 1]
                        l[j + 1] = t
                    }
                }
            }
            kwa i kutoka 0 hadi 6 { jumla = jumla * 10 + l[i] }
            rejesha jumla
        }
    "#;
    let big = 9007199254740990.0;
    let mut calls: Vec<(&str, Vec<Arg>)> = Vec::new();
    for (x, k) in [
        (3.0, 2.0),
        (-7.0, 4.0),
        (1e6, 0.0),
        (big, 7.0),
        (-big, 1.0),
        (0.5, 2.0),
    ] {
        calls.push(("hatua", vec![n(x), n(k)]));
    }
    for (a, b) in [
        (5.0, 9.0),
        (-1.0, 3.0),
        (4294967296.0, 65535.0),
        (-5.9, 123456789.0),
        (big, -big),
        (f64::NAN, 1.0),
        (f64::INFINITY, 2.0),
    ] {
        calls.push(("biti", vec![n(a), n(b)]));
    }
    for (x, y) in [
        (3.0, 9.0),
        (8.0, 2.0),
        (-4.5, 7.25),
        (1e10, 3e10),
        (0.0, -0.0),
    ] {
        calls.push(("mchanganyiko", vec![n(x), n(y)]));
    }
    for x in [3.0, 11.0, f64::NAN] {
        calls.push(("ni_kubwa", vec![n(x)]));
    }
    calls.push(("panga", vec![Arg::List(vec![5.0, 3.0, 9.0, 1.0, 4.0, 2.0])]));
    agree("hesabu", source, &calls);
}

// Verifies: REQ-TGT-2
#[test]
fn random_strict_programs_agree() {
    let count: u64 = std::env::var("ASILI_KIFAA_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40);
    let mut source = String::new();
    let mut calls: Vec<(String, Vec<Arg>)> = Vec::new();
    for seed in 1..=count {
        let (text, name) = strict_gen::function(seed);
        source.push_str(&text);
        let mut rng = strict_gen::Rng::new(seed ^ 0xABCD);
        for _ in 0..3 {
            let list: Vec<f64> = (0..8).map(|_| strict_gen::value(&mut rng)).collect();
            let a = strict_gen::value(&mut rng);
            let b = strict_gen::value(&mut rng);
            calls.push((name.clone(), vec![Arg::List(list), n(a), n(b)]));
        }
    }
    let calls: Vec<(&str, Vec<Arg>)> = calls.iter().map(|(f, a)| (f.as_str(), a.clone())).collect();
    agree("nasibu", &source, &calls);
}

/// Builds for the device only (no tools needed): what a device cannot run is a build error
/// naming the function and the reason.
// Verifies: REQ-TGT-3
#[test]
fn what_a_device_cannot_run_is_a_build_error() {
    let build = |source: &str| -> Result<(), String> {
        let tokens = asili_lexer::tokenize(source).expect("tokenize");
        let module = asili_parser::parse_tokens(&tokens).expect("parse");
        let program = compile_module_explained(&module).expect("bytecode");
        let strict: Vec<usize> = module
            .functions
            .iter()
            .filter(|f| asili_evaluator::salama::is_strict(f))
            .filter_map(|f| program.functions.iter().position(|g| g.name == f.name))
            .collect();
        device::build(&program, &strict, "t").map(|_| ())
    };
    // A callee that writes the list it receives (inlined on a device, it would write the
    // caller's list).
    let e = build(
        "#[salama]\nkazi weka_sifuri(l: Orodha<Namba>) -> Namba {\n    l[0] = 0\n    rejesha 1\n}\n\n\
         #[salama]\nkazi f(l: Orodha<Namba>) -> Namba {\n    rejesha weka_sifuri(l)\n}\n",
    )
    .unwrap_err();
    assert!(
        e.contains("kazi salama 'f' haiwezi kujengwa kwa Cortex-M")
            && e.contains("inayoandika kwenye orodha"),
        "{e}"
    );
    // Within the subset a device runs, the same shapes build.
    build(
        "#[salama]\nkazi soma(l: Orodha<Namba>) -> Namba {\n    rejesha l[0] * 2\n}\n\n\
         #[salama]\nkazi f(l: Orodha<Namba>, x: Namba) -> Namba {\n    rejesha soma(l) + x\n}\n",
    )
    .unwrap();
}
