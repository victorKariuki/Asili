//! Random strict (`#[salama]`) functions for the device tests: counted loops with constant
//! bounds, `ikiwa`, arithmetic and bitwise operators, list reads and writes at in-range indices,
//! and calls to a strict helper — the subset a device runs.

/// xorshift64*: a reproducible stream from a seed.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    pub fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len() as u64) as usize]
    }
}

/// An argument value: mostly small integers, some fractions, large and special values.
pub fn value(rng: &mut Rng) -> f64 {
    match rng.below(12) {
        0 => rng.below(1000) as f64 / 8.0 - 60.0,
        1 => 9007199254740990.0 - rng.below(4) as f64,
        2 => -(rng.below(5_000_000_000) as f64),
        3 => [f64::NAN, f64::INFINITY, -0.0, 1e300][rng.below(4) as usize],
        _ => rng.below(40) as f64 - 10.0,
    }
}

struct Gen {
    rng: Rng,
    counters: Vec<String>,
    out: String,
    indent: usize,
    budget: u32,
    /// Generating the helper `g<seed>` (its only variable is `x`, and it calls nothing).
    helper: bool,
    seed: u64,
}

const VARS: [&str; 3] = ["a", "b", "c"];

impl Gen {
    fn literal(&mut self) -> String {
        match self.rng.below(8) {
            0 => format!("{}.25", self.rng.below(10)),
            1 => "4294967296".to_string(),
            2 => format!("(0 - {})", self.rng.below(20)),
            _ => self.rng.below(20).to_string(),
        }
    }

    fn index(&mut self) -> String {
        match self.counters.last() {
            Some(i) if self.rng.below(2) == 0 => format!("{i} % 8"),
            _ => self.rng.below(8).to_string(),
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

    fn expr(&mut self, depth: u32) -> String {
        if depth == 0 || self.rng.below(3) == 0 {
            return self.atom();
        }
        let a = self.expr(depth - 1);
        match self.rng.below(12) {
            0 => format!("sakafu({a})"),
            1 => format!("dari({a} / 2)"),
            2 if !self.helper => format!("g{}({a})", self.seed),
            _ => {
                let b = self.expr(depth - 1);
                let op = self.rng.pick(&[
                    "+", "-", "*", "+", "-", "/", "%", "//", "&", "|", "^", "<<", ">>",
                ]);
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
        format!("{a} {op} {b}")
    }

    fn line(&mut self, text: &str) {
        self.out.push_str(&"    ".repeat(self.indent));
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn block(&mut self, n: u64) {
        self.indent += 1;
        for _ in 0..n {
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
        match self.rng.below(8) {
            0..=2 => {
                let v = self.rng.pick(&VARS);
                let e = self.expr(3);
                self.line(&format!("{v} = {e}"));
            }
            3 => {
                let v = self.rng.pick(&VARS);
                let op = self.rng.pick(&["+=", "-=", "*="]);
                let e = self.expr(2);
                self.line(&format!("{v} {op} {e}"));
            }
            4 => {
                let i = self.index();
                let e = self.expr(2);
                self.line(&format!("l[{i}] = {e}"));
            }
            5 if nested => {
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
            6 if nested => {
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
            _ => {
                let v = self.rng.pick(&VARS);
                let e = self.expr(2);
                self.line(&format!("{v} = {v} + {e}"));
            }
        }
    }
}

/// A strict function `f<seed>(l: Orodha<Namba>, a: Namba, b: Namba) -> Namba` (with its strict
/// helper `g<seed>`) as source, and its name.
pub fn function(seed: u64) -> (String, String) {
    let mut gen = Gen {
        rng: Rng::new(seed),
        counters: Vec::new(),
        out: String::new(),
        indent: 1,
        budget: 20,
        helper: true,
        seed,
    };
    let helper = gen.expr(3);
    gen.helper = false;
    let name = format!("f{seed}");
    gen.line("weka c: Namba = a - b");
    let n = 3 + gen.rng.below(8);
    for _ in 0..n {
        gen.stmt();
    }
    gen.line("rejesha a + b * 3 + c * 7 + l[0] + l[7]");
    let body = gen.out;
    let text = format!(
        "#[salama]\nkazi g{seed}(x: Namba) -> Namba {{\n    rejesha {helper}\n}}\n\n\
         #[salama]\nkazi {name}(l: Orodha<Namba>, a: Namba, b: Namba) -> Namba {{\n{body}}}\n\n"
    );
    (text, name)
}
