//! Differential fuzzing of the optimizer: random programs, built from a small grammar of the
//! numeric subset (arithmetic, bitwise operators, comparisons, `ikiwa`, counted and `wakati`
//! loops, list reads and writes, calls), compiled with and without `nguvu`'s optimizer, must
//! give the same results bit for bit — or the same error. Every optimizer pass is also checked
//! by the IR verifier as it runs (debug builds). A failure prints the program and its seed.
//!
//! `ASILI_FUZZ_PROGRAMS` sets how many programs to try (default 300), `ASILI_FUZZ_SEED` the
//! first seed (default 1), for longer runs: `ASILI_FUZZ_PROGRAMS=100000 cargo test --release
//! -p asili-evaluator --test fuzz`.

use asili_evaluator::nguvu::{self, Options};
use asili_evaluator::{compile_module_explained, run_bytecode_function_on, Value};

/// xorshift64*: a reproducible stream from a seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len() as u64) as usize]
    }
}

const VARS: [&str; 3] = ["a", "b", "c"];
const LIST_LEN: u64 = 8;

struct Gen {
    rng: Rng,
    /// Loop counters in scope.
    counters: Vec<String>,
    out: String,
    indent: usize,
    /// Statements left to emit (keeps programs small).
    budget: u32,
    /// Generating the helper `g`: its only variable is its parameter `x`, and it calls nothing.
    helper: bool,
}

impl Gen {
    fn literal(&mut self) -> String {
        match self.rng.below(10) {
            0 => format!("{}.5", self.rng.below(10)),
            1 => "9007199254740990".to_string(),
            2 => format!("(0 - {})", self.rng.below(20)),
            _ => self.rng.below(20).to_string(),
        }
    }

    fn atom(&mut self) -> String {
        if self.helper {
            return if self.rng.below(2) == 0 {
                "x".to_string()
            } else {
                self.literal()
            };
        }
        match self.rng.below(6) {
            0 | 1 => self.rng.pick(&VARS).to_string(),
            2 if !self.counters.is_empty() => {
                let i = self.rng.below(self.counters.len() as u64) as usize;
                self.counters[i].clone()
            }
            3 => format!("l[{}]", self.index()),
            _ => self.literal(),
        }
    }

    /// An index that is usually in bounds (an out-of-bounds read is an error both must agree on).
    fn index(&mut self) -> String {
        match self.counters.last() {
            Some(i) if self.rng.below(2) == 0 => format!("{i} % {LIST_LEN}"),
            _ => self.rng.below(LIST_LEN + 1).min(LIST_LEN - 1).to_string(),
        }
    }

    fn expr(&mut self, depth: u32) -> String {
        if depth == 0 || self.rng.below(3) == 0 {
            return self.atom();
        }
        let a = self.expr(depth - 1);
        match self.rng.below(12) {
            0 => format!("sakafu({a})"),
            1 => format!("dari({a} / 2)"),
            2 if !self.helper => format!("g({a})"),
            _ => {
                let b = self.expr(depth - 1);
                let op = self.rng.pick(&[
                    "+", "-", "*", "+", "-", "/", "%", "//", "&", "|", "^", "<<", ">>",
                ]);
                // Keep shifts mostly in range and divisors mostly non-zero, so a program
                // usually runs to its end rather than stopping at its first oddity.
                match op {
                    "<<" | ">>" => format!("({a} {op} ({b} & 15))"),
                    "/" | "%" | "//" if self.rng.below(4) != 0 => {
                        format!("({a} {op} (({b} & 7) + 1))")
                    }
                    _ => format!("({a} {op} {b})"),
                }
            }
        }
    }

    fn cond(&mut self) -> String {
        let a = self.expr(2);
        let b = self.expr(1);
        let op = self.rng.pick(&["<", "<=", ">", ">=", "==", "!="]);
        match self.rng.below(5) {
            0 => format!("{a} {op} {b} && {} > 3", self.atom()),
            1 => format!("{a} & {} == 0", 1 << self.rng.below(6)),
            _ => format!("{a} {op} {b}"),
        }
    }

    fn line(&mut self, text: &str) {
        self.out.push_str(&"    ".repeat(self.indent));
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn block(&mut self, statements: u64) {
        self.indent += 1;
        for _ in 0..statements {
            self.stmt();
        }
        self.indent -= 1;
    }

    fn stmt(&mut self) {
        if self.budget == 0 {
            return;
        }
        self.budget -= 1;
        let nested = self.indent < 4;
        match self.rng.below(10) {
            0 | 1 => {
                let v = self.rng.pick(&VARS);
                let e = self.expr(3);
                self.line(&format!("{v} = {e}"));
            }
            2 => {
                let v = self.rng.pick(&VARS);
                let op = self.rng.pick(&["+=", "-=", "*=", "&=", "|=", "^="]);
                let e = self.expr(2);
                self.line(&format!("{v} {op} {e}"));
            }
            3 => {
                let i = self.index();
                let e = self.expr(2);
                self.line(&format!("l[{i}] = {e}"));
            }
            4 | 5 => {
                let e = self.expr(3);
                self.line(&format!("r.ongeza({e})"));
            }
            6 if nested => {
                let c = self.cond();
                self.line(&format!("ikiwa ({c}) {{"));
                let n = 1 + self.rng.below(3);
                self.block(n);
                if self.rng.below(2) == 0 {
                    self.line("} vinginevyo {");
                    let n = 1 + self.rng.below(2);
                    self.block(n);
                }
                self.line("}");
            }
            7 if nested => {
                let i = format!("i{}", self.counters.len());
                let from = self.rng.below(3);
                let to = from + self.rng.below(10);
                self.line(&format!("kwa {i} kutoka {from} hadi {to} {{"));
                self.counters.push(i);
                let n = 1 + self.rng.below(4);
                self.block(n);
                self.counters.pop();
                self.line("}");
            }
            8 if nested => {
                // A `wakati` loop with its own counter: bounded however the body behaves.
                let k = format!("k{}", self.indent);
                let limit = 1 + self.rng.below(8);
                self.line(&format!("weka {k}: Namba = 0"));
                let c = self.cond();
                self.line(&format!("wakati {k} < {limit} && ({c}) {{"));
                self.indent += 1;
                self.line(&format!("{k} += 1"));
                self.indent -= 1;
                let n = 1 + self.rng.below(3);
                self.block(n);
                self.line("}");
            }
            _ => {
                let e = self.expr(2);
                self.line(&format!("r.ongeza({e})"));
            }
        }
    }
}

fn program(seed: u64) -> String {
    let mut gen = Gen {
        rng: Rng::new(seed),
        counters: Vec::new(),
        out: String::new(),
        indent: 1,
        budget: 24,
        helper: true,
    };
    let helper = gen.expr(3);
    gen.helper = false;
    gen.out.push_str("kazi t() -> Orodha<Namba> {\n");
    gen.line("weka r: Orodha<Namba> = []");
    let l: Vec<String> = (0..LIST_LEN).map(|_| gen.literal()).collect();
    gen.line(&format!("weka l: Orodha<Namba> = [{}]", l.join(", ")));
    for v in VARS {
        let e = gen.literal();
        gen.line(&format!("weka {v}: Namba = {e}"));
    }
    let n = 4 + gen.rng.below(8);
    for _ in 0..n {
        gen.stmt();
    }
    gen.line("r.ongeza(a)");
    gen.line("r.ongeza(b)");
    gen.line("r.ongeza(c)");
    gen.line("kwa x kutoka 0 hadi 8 { r.ongeza(l[x]) }");
    gen.line("rejesha r");
    gen.out.push_str("}\n");
    format!(
        "kazi g(x: Namba) -> Namba {{\n    rejesha {helper}\n}}\n\n{}",
        gen.out
    )
}

/// Canonical text for a result; numbers by bit pattern, every NaN alike (as in `native_tiers`).
fn canon(v: &Value) -> String {
    match v {
        Value::Namba(n) if n.is_nan() => "NaN".to_string(),
        Value::Namba(n) => format!("{:016x}", n.to_bits()),
        Value::Orodha(items) => items.iter().map(canon).collect::<Vec<_>>().join(","),
        other => format!("{other:?}"),
    }
}

fn run(source: &str, options: Options) -> Result<String, String> {
    let tokens = asili_lexer::tokenize(source).map_err(|e| format!("lex: {e:?}"))?;
    let module = asili_parser::parse_tokens(&tokens).map_err(|e| format!("parse: {e:?}"))?;
    let program = compile_module_explained(&module).map_err(|e| format!("bytecode: {e}"))?;
    let library = nguvu::compile_with(&program, options)?;
    Ok(run_bytecode_function_on(&library, &program, "t", vec![])
        .map(|v| canon(&v))
        .unwrap_or_else(|e| format!("ERR {e}")))
}

#[test]
fn optimized_code_computes_what_unoptimized_code_does() {
    if !nguvu::supported() {
        return;
    }
    let count: u64 = std::env::var("ASILI_FUZZ_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let first: u64 = std::env::var("ASILI_FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let mut ran_to_end = 0;
    for seed in first..first + count {
        let source = program(seed);
        if std::env::var_os("ASILI_FUZZ_SHOW").is_some() {
            eprintln!("{source}");
        }
        let plain = run(&source, Options { optimize: false });
        let optimized = run(&source, Options { optimize: true });
        let plain = plain.unwrap_or_else(|e| panic!("seed {seed}: {e}\n{source}"));
        let optimized = optimized.unwrap_or_else(|e| panic!("seed {seed}: {e}\n{source}"));
        assert_eq!(
            plain, optimized,
            "seed {seed}: optimized code differs\n{source}"
        );
        if !plain.starts_with("ERR") {
            ran_to_end += 1;
        }
    }
    // The generator is useful only if most programs run to the end.
    assert!(
        ran_to_end * 2 > count,
        "only {ran_to_end} of {count} programs ran to the end"
    );
}

/// The same source always builds the same bytes — bytecode and machine code — however the
/// parallel code generator schedules its work, so a build can be reproduced and checked.
#[test]
fn builds_are_reproducible() {
    if !nguvu::supported() {
        return;
    }
    let sudoku = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sudoku/src/kuu.as"
    ))
    .expect("sudoku source");
    let sources: Vec<String> = std::iter::once(sudoku)
        .chain((1..=20).map(program))
        .collect();
    for source in &sources {
        let build = || {
            let tokens = asili_lexer::tokenize(source).expect("tokenize");
            let module = asili_parser::parse_tokens(&tokens).expect("parse");
            let asb = asili_evaluator::emit_asb(&module, source).expect("asb");
            let program = compile_module_explained(&module).expect("bytecode");
            let image = nguvu::generate(&program).expect("image").to_bytes(&program);
            (asb, image)
        };
        let first = build();
        for _ in 0..3 {
            assert!(build() == first, "two builds differ:\n{source}");
        }
    }
}

/// With `ASILI_FUZZ_WRITE=<dir>`, writes the fuzz programs as projects (`<dir>/<seed>/`) whose
/// `kuu` prints `t()`, for checks on other targets: `driver/wasm/tests/agree.sh` runs them as
/// native code and as a wasm module and compares the output.
#[test]
fn write_programs_for_other_targets() {
    let Some(dir) = std::env::var_os("ASILI_FUZZ_WRITE") else {
        return;
    };
    let count: u64 = std::env::var("ASILI_FUZZ_PROGRAMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    for seed in 1..=count {
        let project = std::path::Path::new(&dir).join(seed.to_string());
        std::fs::create_dir_all(project.join("src")).expect("project dir");
        std::fs::write(
            project.join("pata.toml"),
            format!(
                "[jumla]\njina = \"fuzz{seed}\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n\
                 [chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n"
            ),
        )
        .expect("pata.toml");
        let source = format!(
            "{}\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {{\n    kwa v katika t() {{ chapisha(v kama Neno) }}\n}}\n",
            program(seed)
        );
        std::fs::write(project.join("src/kuu.as"), source).expect("kuu.as");
    }
}
