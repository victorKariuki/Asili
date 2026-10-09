//! Differential tests: every snippet's native `nguvu` code (loaded through its on-disk image)
//! must reproduce the language's reference results bit for bit (`tests/golden/native_tiers.txt`,
//! recorded from the tree-walking evaluator before it was removed).
//! The snippets target the places where native code could diverge from `f64` semantics:
//! -0.0, NaN, infinities, integers beyond 2^53 (which stay floats), remainders and
//! floor division of negatives, out-of-range shifts, and out-of-bounds list access.

use asili_evaluator::{compile_module, run_bytecode_function_on, Value};
use asili_lexer::tokenize;
use asili_parser::parse_tokens;

#[path = "support/golden.rs"]
mod golden;

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
    // Through the on-disk image, as `pata jenga` + `pata tenda` run it.
    let own = asili_evaluator::nguvu::supported().then(|| {
        let path = asili_evaluator::nguvu::write_image(&program, &dir, name).expect("nguvu image");
        asili_evaluator::nguvu::load_image(&path, &program).expect("load nguvu image")
    });
    if let Some(own) = &own {
        for function in functions {
            let got = run_bytecode_function_on(own, &program, function, vec![])
                .map(|v| canon(&v))
                .unwrap_or_else(|e| format!("ERR {e}"));
            let key = format!("{name}::{function}");
            assert_eq!(
                got,
                golden::expected("native_tiers", &key, &got),
                "{key}: nguvu differs"
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
fn integers_beyond_two_pow_53_stay_exact() {
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

#[test]
fn nguvu_image_rejects_other_programs_and_corruption() {
    use asili_evaluator::nguvu::{load_image, supported, write_image};
    if !supported() {
        return;
    }
    let program = |src: &str| {
        let tokens = tokenize(src).expect("tokenize");
        compile_module(&parse_tokens(&tokens).expect("parse")).expect("bytecode")
    };
    let a = program(
        "kazi t() -> Namba {\n weka s: Namba = 0\n kwa i kutoka 0 hadi 5 { s += i }\n rejesha s\n}",
    );
    let b = program(
        "kazi t() -> Namba {\n weka s: Namba = 1\n kwa i kutoka 0 hadi 5 { s += i }\n rejesha s\n}",
    );
    let dir = std::env::temp_dir().join(format!("asili-nguvu-image-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = write_image(&a, &dir, "a").expect("write");
    assert!(load_image(&path, &a).is_ok());
    assert!(
        load_image(&path, &b).is_err(),
        "image built from other bytecode"
    );
    let mut bytes = std::fs::read(&path).expect("read");
    bytes.truncate(bytes.len() - 1);
    std::fs::write(&path, &bytes).expect("write");
    assert!(load_image(&path, &a).is_err(), "truncated image");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unrolled_loops_if_conversion_and_bit_tests() {
    check(
        "unroll",
        r#"
        kazi hesabu(mask: Namba) -> Namba {
            weka n: Namba = 0
            kwa v kutoka 1 hadi 10 {
                ikiwa mask & (1 << (v - 1)) == 0 {
                    n += 1
                }
            }
            rejesha n
        }
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            kwa m kutoka 0 hadi 40 {
                r.ongeza(hesabu(m * 37))
            }
            # the same count on a proven integer: single-bit tests summed (popcount)
            kwa m kutoka 0 hadi 40 {
                weka mask = (m * 37) % 1024
                weka n: Namba = 0
                weka z: Namba = 0
                kwa v kutoka 1 hadi 10 {
                    ikiwa mask & (1 << (v - 1)) == 0 {
                        n += 1
                    }
                }
                kwa v kutoka 0 hadi 6 {
                    ikiwa mask & (1 << (v + 2)) != 0 {
                        z += 1
                    }
                }
                r.ongeza(n * 100 + z)
            }
            # break and continue inside a fully unrolled loop
            weka s: Namba = 0
            kwa i kutoka 0 hadi 8 {
                ikiwa i == 2 { endelea }
                ikiwa i == 6 { vunja }
                s += i * 10
            }
            r.ongeza(s)
            # nested constant loops, both unrolled
            weka p: Namba = 0
            kwa i kutoka 0 hadi 3 {
                kwa j kutoka 0 hadi 4 {
                    p = p * 3 + i - j
                }
            }
            r.ongeza(p)
            # a counter that leaves the exact-integer range inside an unrolled copy (stays a float)
            weka x: Namba = 9007199254740985
            kwa i kutoka 0 hadi 12 {
                x += 1
            }
            r.ongeza(x)
            # empty and negative trip counts
            weka e: Namba = 5
            kwa i kutoka 3 hadi 3 { e += 1 }
            kwa i kutoka 4 hadi 1 { e += 1 }
            r.ongeza(e)
            # conditional minimum tracking (if-converted select)
            weka bora: Namba = -1
            weka idadi: Namba = 100
            kwa k kutoka 0 hadi 9 {
                weka c = (k * 7) % 5
                ikiwa c < idadi {
                    bora = k
                    idadi = c
                }
            }
            r.ongeza(bora)
            r.ongeza(idadi)
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn division_by_constants_over_small_ranges() {
    check(
        "smalldiv",
        r#"
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            weka s: Namba = 0
            kwa i kutoka 0 hadi 300 {
                weka a = i // 9
                weka b = i % 9
                weka c = (i // 3) * 3 + b // 3
                weka d = i // 7 + i % 11 + i // 100
                s = s + a * 1000 + b * 100 + c * 10 + d
                ikiwa i % 37 == 0 {
                    r.ongeza(a)
                    r.ongeza(b)
                    r.ongeza(c)
                    r.ongeza(d)
                }
            }
            r.ongeza(s)
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn division_by_large_constants() {
    check(
        "bigdiv",
        r#"
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            weka s: Namba = 0
            kwa i kutoka 0 hadi 200000 {
                s = (s + i * 7 + i // 3) % 1000003
            }
            r.ongeza(s)
            # both signs, up to 2^53, with remainder d - 1 (the worst case for a multiply by a
            # rounded reciprocal), through quotients and remainders by large divisors
            kwa k kutoka 0 hadi 40 {
                weka y1000003 = 9007199254516698 - k * 1000003
                r.ongeza(y1000003 // 1000003)
                r.ongeza(y1000003 % 1000003)
                weka z1000003 = 0 - y1000003
                r.ongeza(z1000003 // 1000003)
                r.ongeza(z1000003 % 1000003)
                weka y2147483647 = 9007199250546687 - k * 2147483647
                r.ongeza(y2147483647 // 2147483647)
                r.ongeza(y2147483647 % 2147483647)
                weka z2147483647 = 0 - y2147483647
                r.ongeza(z2147483647 // 2147483647)
                r.ongeza(z2147483647 % 2147483647)
                weka y2049 = 9007199254740479 - k * 2049
                r.ongeza(y2049 // 2049)
                r.ongeza(y2049 % 2049)
                weka z2049 = 0 - y2049
                r.ongeza(z2049 // 2049)
                r.ongeza(z2049 % 2049)
                weka y65537 = 9007199254675486 - k * 65537
                r.ongeza(y65537 // 65537)
                r.ongeza(y65537 % 65537)
                weka z65537 = 0 - y65537
                r.ongeza(z65537 // 65537)
                r.ongeza(z65537 % 65537)
                weka y999999937 = 9007198432546462 - k * 999999937
                r.ongeza(y999999937 // 999999937)
                r.ongeza(y999999937 % 999999937)
                weka z999999937 = 0 - y999999937
                r.ongeza(z999999937 // 999999937)
                r.ongeza(z999999937 % 999999937)
            }
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn integer_and_float_list_representations() {
    check(
        "numlist",
        r#"
        kazi kusanya_zote(l: Orodha<Namba>) -> Namba {
            weka s: Namba = 0
            kwa i kutoka 0 hadi l.urefu() {
                s = s + l[i]
            }
            rejesha s
        }
        kazi t() -> Orodha<Namba> {
            # built from integers, read and written as integers in native code
            weka a: Orodha<Namba> = orodha_rudia(0, 16)
            kwa i kutoka 0 hadi 16 {
                a[i] = i * i - 7
            }
            weka r: Orodha<Namba> = []
            kwa i kutoka 0 hadi 16 {
                r.ongeza(a[15 - i] // 2)
            }
            # -0.0 into a list still held as integers: the sign must survive
            weka z: Orodha<Namba> = [1, 2]
            z[0] = -0.0
            z.ongeza(-0.0)
            r.ongeza(1 / z[0])
            r.ongeza(1 / z[2])
            # a list that turns into floats part-way
            weka b: Orodha<Namba> = [1, 2, 3]
            b.ongeza(0.5)
            b.ongeza(-0.0)
            b.ongeza(9007199254740993)
            b.ongeza(10 ** 300)
            b[0] = -0.0
            kwa i kutoka 0 hadi b.urefu() {
                r.ongeza(b[i])
            }
            # passed to another function and back
            r.ongeza(kusanya_zote(a))
            r.ongeza(kusanya_zote(b))
            weka c: Orodha<Namba> = [4, 5, 6]
            c.ondoa(1)
            c.ongeza(-3)
            r.ongeza(kusanya_zote(c))
            r.ongeza(c[1])
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn direct_native_calls_keep_interpreter_semantics() {
    check(
        "direct",
        r#"
        kazi fib(n: Namba) -> Namba {
            ikiwa n < 2 {
                rejesha n
            }
            rejesha fib(n - 1) + fib(n - 2)
        }
        kazi ngazi_chini(n: Namba) -> Namba {
            ikiwa n == 0 {
                rejesha 0
            }
            rejesha 1 + ngazi_chini(n - 1)
        }
        kazi ni_shufwa_moja(n: Namba) -> Namba {
            ikiwa n == 0 {
                rejesha 1
            }
            rejesha ni_witiri_moja(n - 1)
        }
        kazi ni_witiri_moja(n: Namba) -> Namba {
            ikiwa n == 0 {
                rejesha 0
            }
            rejesha ni_shufwa_moja(n - 1)
        }
        kazi vuka_kikomo(n: Namba) -> Namba {
            # a counter pushed past 2^53 inside a directly called function
            weka x: Namba = 9007199254740980
            kwa i kutoka 0 hadi n {
                x += 1
            }
            rejesha x
        }
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            r.ongeza(fib(20))
            r.ongeza(ni_shufwa_moja(1001))
            r.ongeza(ni_witiri_moja(1001))
            r.ongeza(vuka_kikomo(5))
            r.ongeza(vuka_kikomo(40))
            r.ongeza(ngazi_chini(9000))
            rejesha r
        }
        kazi mno() -> Namba {
            # deeper than the 10,000-call limit: the same error on every engine
            rejesha ngazi_chini(20000)
        }
        kazi ukingoni() -> Namba {
            rejesha ngazi_chini(9998)
        }
        "#,
        &["t", "mno", "ukingoni"],
    );
}

#[test]
fn reused_values_and_forwarded_list_elements() {
    check(
        "reuse",
        r#"
        kazi panga(a: Orodha<Namba>) -> Orodha<Namba> {
            weka n: Namba = a.urefu()
            kwa i kutoka 0 hadi n {
                kwa j kutoka 0 hadi n - 1 - i {
                    ikiwa a[j] > a[j + 1] {
                        weka t = a[j]
                        a[j] = a[j + 1]
                        a[j + 1] = t
                    }
                }
            }
            rejesha a
        }
        kazi t() -> Orodha<Namba> {
            weka a: Orodha<Namba> = [5, -0.5, 3, 9, 1, 2.25, 7, 0, 4, 8]
            rejesha panga(a)
        }
        kazi fahirisi_sawa() -> Orodha<Namba> {
            # different index registers holding the same index: a store through one is seen
            # through the other
            weka a: Orodha<Namba> = [1, 2, 3, 4]
            weka r: Orodha<Namba> = []
            kwa i kutoka 0 hadi 4 {
                weka j = 3 - i
                weka k = i
                weka x = a[k]
                a[i] = x * 10 + 1
                ikiwa a[k] > 20 {
                    a[j] = a[k] + 0.5
                }
                r.ongeza(a[k])
                r.ongeza(a[i])
                r.ongeza(a[j])
            }
            rejesha r
        }
        kazi orodha_mbili() -> Orodha<Namba> {
            # two list variables, writes through one then reads through the other
            weka a: Orodha<Namba> = [1, 2, 3]
            weka b = a
            weka r: Orodha<Namba> = []
            kwa i kutoka 0 hadi 3 {
                weka x = a[i]
                b[i] = x + 100
                r.ongeza(a[i])
                r.ongeza(b[i])
                a.ongeza(x)
                r.ongeza(a[i])
            }
            rejesha r
        }
        "#,
        &["t", "fahirisi_sawa", "orodha_mbili"],
    );
}

#[test]
fn entry_guards_keep_parameters() {
    check(
        "entryguard",
        r#"
        kazi shuka(n: Namba, m: Namba) -> Namba {
            weka s: Namba = 0
            wakati n > 0 {
                s += n * m
                n -= 1
                m += 1
            }
            rejesha s
        }
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            r.ongeza(shuka(5, 2))
            r.ongeza(shuka(4.5, 2))
            r.ongeza(shuka(3, 0.25))
            r.ongeza(shuka(0 - 1.5, 3))
            r.ongeza(shuka(2.5, 1.5))
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn narrow_list_elements() {
    check(
        "narrow",
        r#"
        kazi chuja(n: Namba) -> Namba {
            # a sieve: flags proven 0 or 1, stored as bytes
            weka p: Orodha<Namba> = orodha_rudia(1, n + 1)
            p[0] = 0
            p[1] = 0
            weka i: Namba = 2
            wakati i * i <= n {
                ikiwa p[i] == 1 {
                    weka j = i * i
                    wakati j <= n {
                        p[j] = 0
                        j += i
                    }
                }
                i += 1
            }
            weka s: Namba = 0
            kwa k kutoka 0 hadi n + 1 {
                s += p[k]
            }
            rejesha s
        }
        kazi mipaka() -> Orodha<Namba> {
            # values at each width's edges, written and read back natively
            weka a: Orodha<Namba> = orodha_rudia(0, 8)
            weka b: Orodha<Namba> = orodha_rudia(0, 8)
            weka c: Orodha<Namba> = orodha_rudia(0, 8)
            kwa k kutoka 0 hadi 8 {
                a[k] = k * 36 - 128
                b[k] = k * 9362 - 32768
                c[k] = k * 613566756 - 2147483648
            }
            # and unsigned edges: up to 255, 65535, 2^32 - 1
            weka u: Orodha<Namba> = orodha_rudia(0, 8)
            weka v: Orodha<Namba> = orodha_rudia(0, 8)
            weka w: Orodha<Namba> = orodha_rudia(0, 8)
            kwa k kutoka 0 hadi 8 {
                u[k] = 255 - k * 36
                v[k] = 65535 - k * 9362
                w[k] = 4294967295 - k * 613566756
            }
            weka r: Orodha<Namba> = []
            kwa k kutoka 0 hadi 8 {
                r.ongeza(a[k])
                r.ongeza(b[k])
                r.ongeza(c[k])
                r.ongeza(u[k])
                r.ongeza(v[k])
                r.ongeza(w[k])
            }
            rejesha r
        }
        kazi ongeza_moja(xs: Orodha<Namba>) -> Namba {
            weka s: Namba = 0
            kwa k kutoka 0 hadi xs.urefu() {
                xs[k] = xs[k] + 1
                s += xs[k]
            }
            rejesha s
        }
        kazi pana() -> Orodha<Namba> {
            # a byte list handed to code that widens it, then read again
            weka d: Orodha<Namba> = orodha_rudia(7, 5)
            weka r: Orodha<Namba> = []
            r.ongeza(ongeza_moja(d))
            d[2] = 1000000
            d[3] = 0.5
            r.ongeza(ongeza_moja(d))
            kwa k kutoka 0 hadi 5 {
                r.ongeza(d[k])
            }
            weka e: Orodha<Namba> = orodha_rudia(0, 3)
            e[1] = 0 - 0.0
            r.ongeza(e[1])
            e.ongeza(70000)
            r.ongeza(e[3])
            rejesha r
        }
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            r.ongeza(chuja(100000))
            r.ongeza(chuja(2))
            rejesha r
        }
        "#,
        &["t", "mipaka", "pana"],
    );
}
