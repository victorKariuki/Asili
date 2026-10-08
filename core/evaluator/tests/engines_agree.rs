//! Differential tests: the tree-walking evaluator and the native `nguvu` code built from the
//! program's bytecode must agree on each snippet — values and error messages alike. Operator, cast, method, unwrapping, iteration and display
//! semantics are shared code (`eval::ops`, `eval::methods`), and these tests keep it that way.

use asili_evaluator::{
    compile_module, compile_module_explained, run_bytecode_function_on, run_function, Engine,
    Opcode, Value,
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
    agree_program(name, &module, &program, functions);
}

/// Like `agree`, for a program whose `kazi` are only partly lowered (mixed mode: the rest run
/// on the tree-walker, called from and calling into bytecode).
fn agree_mixed(name: &str, source: &str, functions: &[&str]) {
    let tokens = tokenize(source).expect("tokenize");
    let module = parse_tokens(&tokens).unwrap_or_else(|e| panic!("{name}: parse: {e:?}"));
    let program = compile_module(&module).expect("mixed program");
    assert!(
        program
            .functions
            .iter()
            .any(|f| matches!(f.code.first(), Some(Opcode::Interpreted { .. }))),
        "{name}: expected some kazi left to the tree-walker"
    );
    agree_program(name, &module, &program, functions);
}

fn agree_program(
    name: &str,
    module: &asili_parser::Module,
    program: &asili_evaluator::BytecodeProgram,
    functions: &[&str],
) {
    let (module, program) = (module, program);
    // The in-house native backend, where the host supports it.
    let own = asili_evaluator::nguvu::supported()
        .then(|| asili_evaluator::nguvu::compile(program).expect("nguvu compile"));
    let show = |r: Result<Value, asili_evaluator::EvalError>| match r {
        Ok(v) => canon(&v),
        Err(e) => format!("ERR {e}"),
    };
    for function in functions {
        let tree = show(run_function(module, function, vec![]));
        let host = |engine| show(run_bytecode_function_on(engine, program, function, vec![]));
        assert_eq!(
            host(Engine::Tree),
            tree,
            "{name}::{function}: the program's own syntax tree vs tree-walker"
        );
        if let Some(own) = &own {
            assert_eq!(
                host(Engine::Native(own)),
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

#[test]
fn mixed_programs_agree() {
    agree_mixed(
        "mixed",
        r#"
        umbo Nukta {
            x: Namba,
            y: Namba,
        }
        kazi urefu_wa(p: Nukta) -> Namba {
            # a pattern `weka`: left to the tree-walker
            weka (a, b) = jozi(p.x, p.y)
            rejesha mraba(p.x) + mraba(p.y)
        }
        kazi mraba(n: Namba) -> Namba {
            # compiled, called from the tree-walker
            weka s: Namba = 0
            kwa i kutoka 0 hadi n {
                s += n
            }
            rejesha s
        }
        kazi aina(n: Namba) -> Neno {
            linganisha n {
                0 => { rejesha "sifuri" }
                1 => { rejesha "moja" }
                _ => { rejesha "nyingi" }
            }
            rejesha ""
        }
        kazi jumla_ya_aina(k: Namba) -> Neno {
            # compiled, calling the tree-walker in a loop
            weka r: Neno = ""
            kwa i kutoka 0 hadi k {
                r = r + aina(i)
            }
            rejesha r
        }
        kazi fib(n: Namba) -> Namba {
            ikiwa n < 2 {
                rejesha n
            }
            rejesha fib(n - 1) + fib(n - 2)
        }
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            weka p = Nukta { x: 3, y: 4 }
            r.ongeza(urefu_wa(p))
            r.ongeza(fib(15))
            rejesha r
        }
        kazi aina_tano() -> Neno {
            rejesha jumla_ya_aina(5)
        }
        kazi kosa() -> Namba {
            weka p = Nukta { x: 1, y: 2 }
            rejesha urefu_wa(p) + (1 / 0) * 0
        }
        "#,
        &["t", "aina_tano", "kosa"],
    );
}

#[test]
fn structs_enums_maps_and_global_constants() {
    agree(
        "values",
        r#"
        umbo Nukta {
            x: Namba,
            y: Namba,
            jina: Neno,
        }
        jenum Rangi {
            Nyekundu,
            Kijani(Namba),
        }
        kazi mraba(p: Nukta) -> Namba {
            rejesha p.x * p.x + p.y * p.y
        }
        kazi t() -> Orodha<Namba> {
            weka r: Orodha<Namba> = []
            # fields given out of declaration order
            weka p = Nukta { jina: "a", y: 4, x: 3 }
            r.ongeza(mraba(p))
            r.ongeza(p.x + p.y)
            r.ongeza(Ukomo)
            r.ongeza(- Ukomo)
            r.ongeza(PI)
            ikiwa Siyo_Namba == Siyo_Namba {
                r.ongeza(1)
            }
            rejesha r
        }
        kazi maandishi() -> Neno {
            weka p = Nukta { x: 1, y: 2, jina: "nukta" }
            weka k = Rangi::Kijani(7)
            weka m = { "a": 1, "b": p.jina, "a": 3 }
            # (not the whole map: its iteration order is unspecified)
            rejesha (p kama Neno) + " " + p.jina + " " + (k kama Neno) + " " + (Rangi::Nyekundu kama Neno) + " " + (m.idadi() kama Neno) + " " + (m.pata("a") kama Neno) + " " + (m.pata("b") kama Neno)
        }
        kazi uga_mbaya() -> Namba {
            weka p = Nukta { x: 1, y: 2, jina: "n" }
            rejesha p.z
        }
        kazi si_umbo() -> Namba {
            weka n = 5
            rejesha n.x
        }
        "#,
        &["t", "maandishi", "uga_mbaya", "si_umbo"],
    );
}

#[test]
fn linganisha_patterns() {
    agree(
        "match",
        r#"
        umbo Sanduku {
            upana: Namba,
            ndani: Nukta,
        }
        umbo Nukta {
            x: Namba,
            y: Namba,
        }
        jenum Umbo {
            Duara(Namba),
            Mraba(Namba),
            Tupu,
        }
        kazi aina(v: Namba) -> Neno {
            linganisha v {
                0 => { rejesha "sifuri" }
                1 => { rejesha "moja" }
                n => { rejesha "nyingine " + (n kama Neno) }
            }
            rejesha "hakuna"
        }
        kazi eneo(u: Umbo) -> Namba {
            linganisha u {
                Umbo::Duara(r) => { rejesha r * r * 3 }
                Umbo::Mraba(s) => { rejesha s * s }
                Umbo::Tupu => { rejesha 0 }
            }
            rejesha - 1
        }
        kazi t() -> Orodha<Neno> {
            weka r: Orodha<Neno> = []
            kwa i kutoka 0 hadi 4 {
                r.ongeza(aina(i))
            }
            r.ongeza(aina(0.30000000000000004))
            r.ongeza(eneo(Umbo::Duara(2)) kama Neno)
            r.ongeza(eneo(Umbo::Mraba(3)) kama Neno)
            r.ongeza(eneo(Umbo::Tupu) kama Neno)
            # strings, chars, booleans, Hamna, and no arm matching
            kwa neno katika ["a", "b", "c"] {
                linganisha neno {
                    "a" => { r.ongeza("ni a") }
                    "b" => { r.ongeza("ni b") }
                }
            }
            linganisha 'x' {
                'y' => { r.ongeza("y") }
                'x' => { r.ongeza("x") }
            }
            linganisha kweli {
                si_kweli => { r.ongeza("uongo") }
                kweli => { r.ongeza("kweli") }
            }
            weka h = Hamna
            linganisha h {
                Hamna => { r.ongeza("hamna") }
            }
            # nested struct patterns, pairs, and a binding that shadows an outer name
            weka x = "nje"
            weka s = Sanduku { upana: 5, ndani: Nukta { x: 1, y: 2 } }
            linganisha s {
                Sanduku { upana: 4, ndani: Nukta { x: x, y: _ } } => { r.ongeza("nne " + (x kama Neno)) }
                Sanduku { upana: w, ndani: Nukta { x: x, y: y } } => {
                    r.ongeza((w + x + y) kama Neno)
                }
            }
            r.ongeza(x)
            linganisha jozi(1, "b") {
                (1, b) => { r.ongeza(b) }
                _ => { r.ongeza("?") }
            }
            # Chaguo from a builtin (not a constructor)
            weka m = { "k": 9 }
            linganisha m.pata("k") {
                Chaguo::Kuna(v) => { r.ongeza("kuna " + (v kama Neno)) }
                Chaguo::Hamna => { r.ongeza("hamna") }
            }
            linganisha m.pata("z") {
                Chaguo::Kuna(v) => { r.ongeza("kuna " + (v kama Neno)) }
                Chaguo::Hamna => { r.ongeza("hamna z") }
            }
            # vunja out of a loop from inside an arm
            weka k = 0
            wakati kweli {
                linganisha k {
                    3 => { vunja }
                    _ => { k += 1 }
                }
            }
            r.ongeza(k kama Neno)
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn user_methods_named_like_builtins_stay_user_methods() {
    // `ongeza`/`urefu` are also builtin method names: a call on a `umbo` value must reach the
    // user's `shughuli ya` method on every tier.
    agree_mixed(
        "impl_clash",
        r#"
        umbo Mfuko {
            vitu: Namba,
        }
        shughuli ya Mfuko {
            kazi ongeza(self: Mfuko, n: Namba) -> Namba {
                rejesha self.vitu + n * 10
            }
            kazi urefu(self: Mfuko) -> Namba {
                rejesha self.vitu * 2
            }
        }
        kazi t() -> Orodha<Namba> {
            weka m = Mfuko { vitu: 3 }
            weka r: Orodha<Namba> = []
            r.ongeza(m.ongeza(4))
            r.ongeza(m.urefu())
            rejesha r
        }
        "#,
        &["t"],
    );
}

const METHODS: &str = r#"
    sifa Eneo {
        kazi eneo(self: Self) -> Namba
    }
    umbo Mstatili {
        upana: Namba,
        urefu_wake: Namba,
    }
    shughuli ya Mstatili {
        kazi mzunguko(self: Mstatili) -> Namba {
            rejesha 2 * (self.upana + self.urefu_wake)
        }
        kazi jumla_ya_hatua(self: Mstatili, n: Namba) -> Namba {
            weka s: Namba = 0
            kwa i kutoka 0 hadi n {
                s += self.mzunguko() + i
            }
            rejesha s
        }
        # inherent and trait methods of the same name: inherent wins
        kazi eneo(self: Mstatili) -> Namba {
            rejesha self.upana * self.urefu_wake
        }
    }
    shughuli ya Mstatili: Eneo {
        kazi eneo(self: Mstatili) -> Namba {
            rejesha - 1
        }
    }
    umbo Duara {
        nusu: Namba,
    }
    shughuli ya Duara: Eneo {
        kazi eneo(self: Duara) -> Namba {
            rejesha self.nusu * self.nusu * 3
        }
    }
"#;

#[test]
fn methods_dispatch_like_the_tree_walker() {
    agree(
        "methods",
        &format!(
            "{METHODS}{}",
            r#"
            kazi t() -> Orodha<Namba> {
                weka m = Mstatili { upana: 3, urefu_wake: 4 }
                weka d: Duara = Duara { nusu: 2 }
                weka r: Orodha<Namba> = []
                # a method call as a statement (result discarded)
                m.mzunguko()
                r.ongeza(m.eneo())
                r.ongeza(m.mzunguko())
                r.ongeza(m.jumla_ya_hatua(5))
                r.ongeza(d.eneo())
                rejesha r
            }
            "#
        ),
        &["t"],
    );
    agree_mixed(
        "methods_mixed",
        &format!(
            "{METHODS}{}",
            r#"
            kazi t() -> Orodha<Namba> {
                # a pattern `weka` keeps this kazi on the tree-walker, which calls compiled
                # methods
                weka m = Mstatili { upana: 3, urefu_wake: 4 }
                weka (a, b) = jozi(1, 2)
                weka r: Orodha<Namba> = []
                r.ongeza(m.eneo())
                r.ongeza(m.jumla_ya_hatua(5))
                rejesha r
            }
            "#
        ),
        &["t"],
    );
}

#[test]
fn tupa_releases_bindings() {
    agree(
        "tupa",
        r#"
        kazi achilia() -> Namba {
            weka x: Namba = 1
            weka jumla: Namba = 0
            kwa i kutoka 0 hadi 4 {
                weka b: Orodha<Namba> = [i, i * 2]
                weka n: Namba = b[1]
                weka s = "neno" + (i kama Neno)
                jumla = jumla + n + s.urefu()
                tupa b
                tupa n
                tupa s
            }
            ikiwa kweli {
                weka x: Namba = 40
                jumla = jumla + x
                tupa x
                jumla = jumla + x
            }
            rejesha jumla
        }
        "#,
        &["achilia"],
    );
}

#[test]
fn typed_struct_fields() {
    agree(
        "fields",
        r#"
        umbo Nukta {
            jina: Neno,
            x: Namba,
            y: Namba,
        }
        umbo Sanduku {
            ndani: Nukta,
            upana: Namba,
        }
        kazi songa(p: Nukta, dx: Namba) -> Nukta {
            weka x = p.x + dx
            ikiwa x > 10 {
                x = x - 20
            }
            rejesha Nukta { jina: p.jina + "'", x: x, y: p.y * 2 }
        }
        kazi hesabu() -> Orodha<Namba> {
            weka s = Sanduku { ndani: Nukta { jina: "a", x: 1.5, y: -2 }, upana: 3 }
            weka r: Orodha<Namba> = []
            weka p: Nukta = s.ndani
            kwa i kutoka 0 hadi 5 {
                p = songa(p, s.upana + i)
                r.ongeza(p.x)
                r.ongeza(p.y % 7)
                r.ongeza(p.jina.urefu())
            }
            r.ongeza(s.ndani.x * s.upana)
            rejesha r
        }
        "#,
        &["hesabu"],
    );
}

#[test]
fn shared_lists_keep_value_semantics() {
    agree(
        "cow",
        r#"
        kazi badili(m: Orodha<Neno>) -> Namba {
            m.ongeza("ndani")
            m.badilisha(0, "x")
            rejesha m.urefu()
        }
        kazi nakala() -> Orodha<Neno> {
            weka a: Orodha<Neno> = ["moja", "mbili"]
            weka b = a.clona()
            b.ongeza("tatu")
            weka c = b.clona()
            c.ondoa(0)
            weka n = badili(a.clona())
            kwa w katika a {
                a.ongeza(w + "!")
            }
            weka r: Orodha<Neno> = []
            r.ongeza(a.urefu() kama Neno)
            r.ongeza(b.urefu() kama Neno)
            r.ongeza(c.urefu() kama Neno)
            r.ongeza(n kama Neno)
            r.ongeza(a[0])
            r.ongeza(b[0])
            r.ongeza(c[0])
            r.ongeza(a[3])
            rejesha r
        }
        "#,
        &["nakala"],
    );
}

#[test]
fn kwa_katika_over_generic_lists() {
    // Native code keeps the loop index in a machine register: `IterItem` must be told it
    // reads it (`native::num_reads`), or every pass sees the first item.
    agree(
        "iter",
        r#"
        umbo Nukta {
            x: Namba,
            y: Namba,
        }
        kazi urefu_wote(maneno: Orodha < Neno >) -> Namba {
            weka s: Namba = 0
            kwa w katika maneno {
                s += w.urefu()
            }
            rejesha s
        }
        kazi jumla(ns: Orodha<Nukta>) -> Namba {
            weka s: Namba = 0
            kwa n katika ns {
                s += n.x * 10 + n.y
            }
            rejesha s
        }
        kazi maneno() -> Namba {
            weka m: Orodha<Neno> = []
            kwa i kutoka 0 hadi 40 {
                m.ongeza("neno" + (i kama Neno))
            }
            rejesha urefu_wote(m.clona())
        }
        kazi nukta() -> Namba {
            weka ns: Orodha<Nukta> = []
            kwa i kutoka 0 hadi 40 {
                ns.ongeza(Nukta { x: i, y: 40 - i })
            }
            rejesha jumla(ns)
        }
        "#,
        &["maneno", "nukta"],
    );
}

#[test]
fn own_kazi_shadow_builtins() {
    let source = r#"
        kazi jumla(a: Namba, b: Namba) -> Namba {
            rejesha a * 100 + b
        }
        kazi sakafu(n: Namba) -> Namba {
            rejesha n + 0.25
        }
        kazi t() -> Namba {
            rejesha jumla(2, 3) + sakafu(1.5)
        }
        "#;
    agree("shadow", source, &["t"]);
    // Agreeing is not enough: every engine used to call the builtins instead.
    let module = parse_tokens(&tokenize(source).unwrap()).unwrap();
    assert_eq!(
        canon(&run_function(&module, "t", vec![]).unwrap()),
        canon(&Value::Namba(204.75))
    );
}

#[test]
fn list_literal_element_types() {
    agree(
        "literal_types",
        r#"
        kazi t() -> Orodha<Namba> {
            weka maneno = ["simba", "tembo", "chui"]
            weka k: Kamusi<Neno, Namba> = { "simba": 0 }
            weka r: Orodha<Namba> = []
            kwa i kutoka 0 hadi 9 {
                weka neno = maneno[i % 3]
                weka nakala = neno.clona()
                r.ongeza(nakala.urefu())
                linganisha k.pata(neno.clona()) {
                    Chaguo::Kuna(n) => { k.ingiza(neno.clona(), n + 1) }
                    Chaguo::Hamna => { k.ingiza(neno.clona(), 1) }
                }
            }
            weka m = maneno.clona()
            m.ongeza("x")
            r.ongeza(maneno.urefu())
            r.ongeza(m.urefu())
            r.ongeza(k.idadi())
            rejesha r
        }
        "#,
        &["t"],
    );
}

#[test]
fn copies_of_maps_and_sets_stay_independent() {
    // `Kamusi` and `Seti` share storage between copies until one is written (copy on write).
    agree(
        "cow",
        r#"
        kazi nakala() -> Orodha<Namba> {
            weka a: Kamusi<Neno, Namba> = {}
            a["x"] = 1
            weka b = a.clona()
            b["y"] = 2
            b["x"] = 5
            weka s = seti()
            s.ongeza(1)
            weka t = s.clona()
            t.ongeza(2)
            rejesha [a.idadi(), b.idadi(), a.pata("x").angu(0), b.pata("x").angu(0), s.urefu(), t.urefu()]
        }
        "#,
        &["nakala"],
    );
}

#[test]
fn values_stay_small() {
    // Every `Value` is moved and copied constantly; keep it at four words.
    assert!(std::mem::size_of::<asili_evaluator::Value>() <= 32);
}
