//! Differential tests: the tree-walking evaluator and every bytecode tier (the VM interpreter
//! and the native `nguvu` code) must agree on each snippet — values
//! and error messages alike. Operator, cast, method, unwrapping, iteration and display
//! semantics are shared code (`eval::ops`, `eval::methods`), and these tests keep it that way.

use asili_evaluator::{
    compile_module_explained, run_bytecode_function_on, run_function, Engine, Value,
};
use asili_lexer::tokenize;
use asili_parser::parse_tokens;

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

fn agree(name: &str, source: &str, functions: &[&str]) {
    let tokens = tokenize(source).expect("tokenize");
    let module = parse_tokens(&tokens).unwrap_or_else(|e| panic!("{name}: parse: {e:?}"));
    let program = compile_module_explained(&module)
        .unwrap_or_else(|why| panic!("{name}: expected bytecode lowering, blocked at {why}"));
    // The in-house native backend, where the host supports it.
    let own = asili_evaluator::nguvu::supported()
        .then(|| asili_evaluator::nguvu::compile(&program).expect("nguvu compile"));
    let show = |r: Result<Value, asili_evaluator::EvalError>| match r {
        Ok(v) => canon(&v),
        Err(e) => format!("ERR {e}"),
    };
    for function in functions {
        let tree = show(run_function(&module, function, vec![]));
        let vm = |engine| show(run_bytecode_function_on(engine, &program, function, vec![]));
        assert_eq!(
            vm(Engine::Interpreter),
            tree,
            "{name}::{function}: VM vs tree-walker"
        );
        if let Some(own) = &own {
            assert_eq!(
                vm(Engine::Native(own)),
                tree,
                "{name}::{function}: nguvu vs tree-walker"
            );
        }
    }
}

#[test]
fn operators_on_every_type() {
    agree(
        "ops",
        r#"
        kazi nambari() -> Orodha<Namba> {
            weka a: Namba = 7.5
            weka b: Namba = - 2
            weka r: Orodha<Namba> = [a + b, a - b, a * b, a / b, a % b, a ** 2]
            r.ongeza(a << 2)
            r.ongeza(a >> 1)
            r.ongeza(a & 3)
            r.ongeza(a | 8)
            r.ongeza(a ^ 5)
            r.ongeza(1 << 70)
            rejesha r
        }
        kazi maneno() -> Orodha<Ukweli> {
            weka x: Neno = "papai"
            weka y: Neno = "embe"
            rejesha [x < y, x > y, x <= "papai", x >= "zzz", x == "papai", x != y, (x + y) == "papaiembe"]
        }
        kazi mantiki() -> Orodha<Ukweli> {
            weka t: Ukweli = kweli
            weka f: Ukweli = si_kweli
            rejesha [t na f, t au f, siyo t, [1, 2] == [1, 2], "a" == "b"]
        }
        kazi unganisha() -> Neno {
            rejesha "n=" + (3.25 kama Neno) + " b=" + (kweli kama Neno)
        }
        kazi kosa_la_aina() -> Namba {
            weka x: Neno = "a"
            rejesha (x kama Namba) - 1
        }
        "#,
        &["nambari", "maneno", "mantiki", "unganisha", "kosa_la_aina"],
    );
}

#[test]
fn casts_and_number_display() {
    agree(
        "casts",
        r#"
        kazi maandishi() -> Orodha<Neno> {
            weka z: Namba = 0
            weka n: Namba = z / z
            weka i: Namba = 1 / z
            weka r: Orodha<Neno> = [(n kama Neno), (i kama Neno), ((- i) kama Neno), ((0.1 + 0.2) kama Neno)]
            weka xs: Orodha<Namba> = [n, i, 2.5]
            r.ongeza(xs.jiunge(","))
            r.ongeza(xs.kwa_neno().jiunge("|"))
            rejesha r
        }
        kazi nambari() -> Orodha<Namba> {
            rejesha [("42" kama Namba), ("x" kama Namba), (kweli kama Namba), ('A' kama Namba)]
        }
        "#,
        &["maandishi", "nambari"],
    );
}

#[test]
fn unwrapping_with_question_mark_and_jaribu() {
    agree(
        "unwrap",
        r#"
        kazi sawa() -> Namba {
            weka b: Orodha<Namba> = [1, 2]
            rejesha b[1]? + 1
        }
        kazi kosa() -> Namba {
            weka b: Orodha<Namba> = [1, 2]
            weka x: Namba = b[9]?
            rejesha x
        }
        kazi jaribu_kosa() -> Namba {
            weka b: Orodha<Namba> = [1, 2]
            rejesha jaribu b[9]
        }
        kazi chaguo() -> Namba {
            weka b: Orodha<Namba> = [5]
            rejesha b.pata(0)? + b.pata(3).angu(10)
        }
        "#,
        &["sawa", "kosa", "jaribu_kosa", "chaguo"],
    );
}

#[test]
fn collection_methods_mutation_and_callbacks() {
    agree(
        "methods",
        r#"
        kazi mara_mbili(x: Namba) -> Namba {
            rejesha x * 2
        }
        kazi ni_kubwa(x: Namba) -> Ukweli {
            rejesha x > 2
        }
        kazi orodha() -> Orodha<Namba> {
            weka b: Orodha<Namba> = [1, 2, 3, 4]
            b.ongeza(5)
            b.ingiza(0, 9)
            b.ondoa(1)
            b.badilisha(0, 7)
            weka r: Orodha<Namba> = b.ramani("mara_mbili")
            r.ongeza(b.hesabu("ni_kubwa"))
            r.ongeza(b.chuja("ni_kubwa").urefu())
            rejesha r
        }
        kazi maneno() -> Orodha<Neno> {
            weka s: Neno = "Habari Dunia"
            rejesha [s.kwa_herufi_ndogo(), s.badilisha("Dunia", "Asili"), s.rudia(2), s.gawanya(" ").jiunge("+")]
        }
        kazi kamusi_na_seti() -> Orodha<Namba> {
            weka k = kamusi()
            k.ingiza("a", 1)
            k.weka_key("b", 2)
            weka s = seti()
            s.ongeza(3)
            s.ongeza(3)
            s.ongeza(4)
            weka r: Orodha<Namba> = [k.idadi(), s.urefu()]
            ikiwa k.vipo("a") {
                r.ongeza(k.pata("b").angu(0))
            }
            ikiwa s.ina(4) {
                r.ongeza(1)
            }
            rejesha r
        }
        kazi nje_ya_mipaka() -> Namba {
            weka b: Orodha<Namba> = [1]
            b.ingiza(4, 2)
            rejesha 0
        }
        "#,
        &["orodha", "maneno", "kamusi_na_seti", "nje_ya_mipaka"],
    );
}

#[test]
fn loops_iteration_and_ranges() {
    agree(
        "loops",
        r#"
        kazi masafa() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            kwa i kutoka 0.5 hadi 3.9 {
                r.ongeza(i)
            }
            kwa i kutoka 3 hadi 1 {
                r.ongeza(100)
            }
            kwa x katika [10, 20, 30] {
                r.ongeza(x)
            }
            rejesha r
        }
        kazi kamusi_jumla() -> Namba {
            weka k = kamusi()
            k.ingiza("a", 1)
            k.ingiza("b", 2)
            k.ingiza("c", 4)
            weka jumla: Namba = 0
            kwa jozi katika k {
                jumla += jozi.pili()
            }
            rejesha jumla
        }
        kazi lebo() -> Namba {
            weka n: Namba = 0
            lebo 'nje: kwa i kutoka 0 hadi 5 {
                kwa j kutoka 0 hadi 5 {
                    ikiwa j == 3 {
                        endelea 'nje
                    }
                    ikiwa i == 4 {
                        vunja 'nje
                    }
                    n += 1
                }
            }
            rejesha n
        }
        "#,
        &["masafa", "kamusi_jumla", "lebo"],
    );
}

#[test]
fn indexing_without_question_mark() {
    agree(
        "index",
        r#"
        kazi soma() -> Namba {
            weka b: Orodha<Namba> = [4, 5, 6]
            rejesha b[0] + b[2] * 10
        }
        kazi nje() -> Namba {
            weka b: Orodha<Namba> = [4, 5, 6]
            rejesha b[3]
        }
        kazi nje_hasi() -> Namba {
            weka b: Orodha<Namba> = [4, 5, 6]
            rejesha b[0 - 1] + b[7]
        }
        kazi na_swali() -> Namba {
            weka b: Orodha<Namba> = [4, 5, 6]
            rejesha b[3]?
        }
        kazi jaribu_nje() -> Namba {
            weka b: Orodha<Namba> = [4, 5, 6]
            rejesha jaribu b[3]
        }
        kazi maneno() -> Neno {
            weka m: Orodha<Neno> = ["a", "b"]
            rejesha m[1] + m[0]
        }
        kazi maneno_nje() -> Neno {
            weka m: Orodha<Neno> = ["a", "b"]
            rejesha m[2]
        }
        kazi kamusi_pata() -> Namba {
            weka k = kamusi()
            k.ingiza("a", 3)
            rejesha k["a"]
        }
        "#,
        &[
            "soma",
            "nje",
            "nje_hasi",
            "na_swali",
            "jaribu_nje",
            "maneno",
            "maneno_nje",
            "kamusi_pata",
        ],
    );
}

#[test]
fn number_list_representations_match_the_tree_walker() {
    // Lists switch between integer and float words; values (including -0.0 and huge
    // integers) must read back exactly as the evaluator's `Orodha` holds them.
    agree(
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
