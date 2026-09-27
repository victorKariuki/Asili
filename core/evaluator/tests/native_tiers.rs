//! Differential tests: every snippet must produce bit-identical results on the register VM's
//! interpreter, the Cranelift JIT, and the LLVM AOT library (when `clang` is available).
//! The snippets target the places where native code could diverge from `f64` semantics:
//! -0.0, NaN, infinities, integers beyond 2^53 (speculation/deoptimization), remainders and
//! floor division of negatives, out-of-range shifts, and out-of-bounds list access.

use asili_evaluator::aot::{build_library, AotError, NativeLibrary};
use asili_evaluator::{compile_module, run_bytecode_function_on, Engine, Value};
use asili_lexer::tokenize;
use asili_parser::parse_tokens;

/// Canonical text for a value; numbers by bit pattern so -0.0 is distinguished. NaN sign and
/// payload bits are not observable from Asili (and not specified by Rust or LLVM), so every
/// NaN compares equal.
fn canon(v: &Value) -> String {
    match v {
        Value::Namba(n) if n.is_nan() => "NaN".to_string(),
        Value::Namba(n) => format!("N{:016x}", n.to_bits()),
        Value::Orodha(items) => {
            let inner: Vec<String> = items.iter().map(canon).collect();
            format!("[{}]", inner.join(","))
        }
        other => format!("{other:?}"),
    }
}

fn check(name: &str, source: &str, functions: &[&str]) {
    let tokens = tokenize(source).expect("tokenize");
    let module = parse_tokens(&tokens).expect("parse");
    let program = compile_module(&module).expect("subset should lower to bytecode");
    let dir = std::env::temp_dir().join(format!("asili-aot-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let lib = match build_library(&program, &dir, name) {
        Ok(path) => Some(NativeLibrary::load(&path, &program).expect("load AOT library")),
        Err(AotError::Unavailable(why)) => {
            eprintln!("{name}: AOT skipped ({why})");
            None
        }
        Err(AotError::Failed(why)) => panic!("{name}: AOT build failed: {why}"),
    };
    for function in functions {
        let run = |engine| {
            run_bytecode_function_on(engine, &program, function, vec![])
                .map(|v| canon(&v))
                .unwrap_or_else(|e| format!("ERR {e}"))
        };
        let interpreted = run(Engine::Interpreter);
        assert_eq!(
            run(Engine::Jit),
            interpreted,
            "{name}::{function}: JIT differs"
        );
        if let Some(lib) = &lib {
            assert_eq!(
                run(Engine::Aot(lib)),
                interpreted,
                "{name}::{function}: AOT differs"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn signed_zero_remainders_and_floor_division() {
    check(
        "zero",
        r#"
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            weka z: Namba = 0
            weka m: Namba = - 1
            r.ongeza(- z)
            r.ongeza(z * m)
            r.ongeza((0 - 4) % 2)
            r.ongeza(7 % (0 - 3))
            r.ongeza((0 - 7) % 3)
            r.ongeza(5.5 % 2)
            r.ongeza(sakafu((0 - 7) / 2))
            r.ongeza(sakafu(7 / 2))
            r.ongeza(dari(0 - 0.5))
            kwa i kutoka 0 hadi 20 {
                r.ongeza(sakafu(i / 3) * 3 + i % 3)
                r.ongeza((i - 10) % 4)
                r.ongeza(sakafu((i - 10) / 4))
            }
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn integers_beyond_two_pow_53_deoptimize_correctly() {
    check(
        "big",
        r#"
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            weka x: Namba = 9007199254740990
            kwa i kutoka 0 hadi 6 {
                x += 1
                r.ongeza(x)
            }
            weka m: Namba = 1
            kwa i kutoka 0 hadi 70 {
                m = m * 3
            }
            r.ongeza(m)
            weka k: Namba = 0
            weka hatua: Namba = 1
            wakati k < 10 {
                hatua = hatua * 1024
                k += 1
            }
            r.ongeza(hatua)
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn shifts_and_bitwise_operators() {
    check(
        "bits",
        r#"
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            kwa s kutoka 0 hadi 70 {
                weka k: Namba = s - 3
                r.ongeza(1 << k)
                r.ongeza((0 - 8) >> k)
                r.ongeza((k * 7) | 5)
                r.ongeza((k * 7) & 12)
                r.ongeza((k * 7) ^ 9)
            }
            r.ongeza(5.9 | 0)
            r.ongeza((0 - 5.9) & 255)
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn nan_and_infinities() {
    check(
        "nan",
        r#"
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            weka z: Namba = 0
            weka n: Namba = z / z
            weka i: Namba = 1 / z
            r.ongeza(i)
            r.ongeza(- i)
            r.ongeza(i - i)
            ikiwa n == n {
                r.ongeza(1)
            } vinginevyo {
                r.ongeza(2)
            }
            ikiwa n != n {
                r.ongeza(3)
            }
            ikiwa n < 1 {
                r.ongeza(4)
            } vinginevyo {
                r.ongeza(5)
            }
            r.ongeza(n | 0)
            r.ongeza(i | 0)
            r.ongeza(1 << n)
            r.ongeza(sakafu(i))
            r.ongeza(i % 2)
            r.ongeza(2 % z)
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn list_access_errors_and_value_semantics() {
    check(
        "lists",
        r#"
        kazi nje() -> Namba {
            weka b: Orodha<Namba> = [1, 2, 3]
            rejesha b[5]?
        }
        kazi hasi() -> Namba {
            weka b: Orodha<Namba> = [1, 2, 3]
            rejesha b[0 - 2]?
        }
        kazi weka_nje() -> Namba {
            weka b: Orodha<Namba> = [1, 2, 3]
            b[3] = 9
            rejesha 0
        }
        kazi badili(b: Orodha<Namba>) -> Namba {
            b[0] = 100
            b.ongeza(7)
            rejesha b.urefu()
        }
        kazi thamani() -> Orodha<Namba> {
            weka b: Orodha<Namba> = orodha_rudia(4, 3)
            weka n: Namba = badili(b)
            b.ongeza(n)
            b.ondoa(0)
            b.ondoa(10)
            rejesha b
        }
        "#,
        &["nje", "hasi", "weka_nje", "thamani"],
    );
}

#[test]
fn control_flow_calls_and_recursion() {
    check(
        "flow",
        r#"
        kazi fib(n: Namba) -> Namba {
            ikiwa n < 2 {
                rejesha n
            }
            rejesha fib(n - 1) + fib(n - 2)
        }
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            r.ongeza(fib(20))
            weka jumla: Namba = 0
            lebo 'nje: kwa i kutoka 0 hadi 10 {
                kwa j kutoka 0 hadi 10 {
                    ikiwa j > i {
                        endelea 'nje
                    }
                    ikiwa i * j > 40 {
                        vunja 'nje
                    }
                    jumla += i * j
                }
            }
            r.ongeza(jumla)
            weka k: Namba = 0
            wakati kweli {
                k += 3
                ikiwa k > 50 na k % 2 == 0 {
                    vunja
                }
            }
            r.ongeza(k)
            r.ongeza(ikiwa k > 3 { 1 } vinginevyo { 2 })
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn generic_values_mixed_with_native_loops() {
    check(
        "mixed",
        r#"
        kazi mstari(b: Orodha<Namba>) -> Neno {
            rejesha b.kwa_neno().jiunge(" ")
        }
        kazi t() -> Neno {
            weka maandishi: Neno = ""
            weka b: Orodha<Namba> = []
            kwa i kutoka 0 hadi 12 {
                b.ongeza(i * i)
                ikiwa i % 4 == 0 {
                    maandishi = maandishi + (i kama Neno) + ","
                }
            }
            rejesha maandishi + b.vipande(4).ramani("mstari").jiunge("|")
        }
        "#,
        &["t"],
    );
}

#[test]
fn sudoku_example_matches() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sudoku/src/kuu.as"
    ))
    .expect("read sudoku example");
    // Turn the entry point into a function returning the attempt/backtrack counts.
    let source = source
        .replace(
            "kazi kuu(hoja: Orodha<Neno>) -> Tupu {",
            "kazi tatua() -> Orodha<Namba> {",
        )
        .replace(
            "    ikiwa imekamilika {\n        onyesha(b)",
            "    rejesha [majaribio, marudio]\n    ikiwa imekamilika {\n        onyesha(b)",
        );
    assert!(source.contains("rejesha [majaribio, marudio]"));
    check("sudoku", &source, &["tatua"]);
}
