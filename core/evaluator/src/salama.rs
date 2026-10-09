//! Strict code (`#[salama]`): a subset of the language whose running time and memory are known
//! when the program is built. A `kazi` marked `#[salama]` must
//!
//! - take only `Namba`, `Ukweli` and `Orodha<Namba>` parameters and return `Namba`, `Ukweli` or
//!   nothing (lists are made by ordinary code at start-up and passed in);
//! - loop only with `kwa i kutoka a hadi b` where `a` and `b` are known when building (number
//!   literals, `thabiti` number constants, arithmetic on them), never assigning `i` in the body —
//!   no `wakati`, no `kwa … katika`;
//! - call only other `#[salama]` functions, with no recursion (direct or through others);
//! - compile to instructions that never allocate or call out to code with unbounded time: numeric
//!   arithmetic and comparisons, jumps, reads and writes of existing list elements, list lengths,
//!   calls to strict functions.
//!
//! Each strict function then has a static worst-case bound on the steps (statements and
//! operations) one call executes, computed here and printed by `pata jenga`, and the memory it
//! needs is its frame and those of the strict functions it may call (`kumbukumbu`).

use asili_parser::{BinaryOp, Block, Expr, ExprId, ForMode, Function, Module, Stmt, UnaryOp};
use std::collections::HashMap;

use crate::bytecode::{BytecodeProgram, Opcode, Ty};

/// What the build learned about each strict function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ripoti {
    pub kazi: String,
    /// Upper bound on the steps one call executes (callees included).
    pub hatua: u64,
    /// Upper bound on the frame memory one call needs, in bytes (its own frame and those of
    /// the deepest chain of strict calls it can make).
    pub kumbukumbu: u64,
}

/// A rule a strict function breaks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kosa {
    pub kazi: String,
    pub mstari: usize,
    pub sababu: String,
}

impl std::fmt::Display for Kosa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "kazi salama '{}', mstari {}: {}",
            self.kazi, self.mstari, self.sababu
        )
    }
}

/// Whether `f` is marked `#[salama]`.
pub fn is_strict(f: &Function) -> bool {
    f.attrs.iter().any(|a| a.name == "salama")
}

/// Check every `#[salama]` function of `module` (lowered as `program`) and bound it.
pub fn kagua(module: &Module, program: &BytecodeProgram) -> Result<Vec<Ripoti>, Vec<Kosa>> {
    let strict: HashMap<&str, &Function> = module
        .functions
        .iter()
        .filter(|f| is_strict(f))
        .map(|f| (f.name.as_str(), f))
        .collect();
    // The safe state (`#[hali_salama]`): at most one, taking nothing.
    let safe: Vec<&Function> = module
        .functions
        .iter()
        .filter(|f| f.attrs.iter().any(|a| a.name == "hali_salama"))
        .collect();
    let mut safe_errors = Vec::new();
    for f in &safe {
        if !f.params.is_empty() {
            safe_errors.push(Kosa {
                kazi: f.name.to_string(),
                mstari: f.line,
                sababu: "kazi ya hali salama haichukui hoja".into(),
            });
        }
    }
    if let [_, second, ..] = safe.as_slice() {
        safe_errors.push(Kosa {
            kazi: second.name.to_string(),
            mstari: second.line,
            sababu: "programu ina kazi moja tu ya hali salama".into(),
        });
    }
    if !safe_errors.is_empty() {
        return Err(safe_errors);
    }
    if strict.is_empty() {
        return Ok(Vec::new());
    }
    let mut checker = Checker {
        module,
        strict: &strict,
        errors: Vec::new(),
        current: "",
    };
    let mut names: Vec<&str> = strict.keys().copied().collect();
    names.sort_unstable();
    for &name in &names {
        let f = strict[name];
        checker.current = name;
        checker.signature(f);
        checker.block(&f.body, &mut Vec::new());
        // Calls are checked in the source: a call to a small function may be inlined.
        for callee in callees(module, &f.body) {
            if !strict.contains_key(callee) {
                checker.fail(f.line, format!("inaita '{callee}', ambayo si kazi salama"));
            }
        }
        checker.code(program, name, f.line);
    }
    checker.recursion(&names);
    if !checker.errors.is_empty() {
        return Err(checker.errors);
    }
    let mut bounds = Bounds {
        module,
        strict: &strict,
        program,
        steps: HashMap::new(),
        memory: HashMap::new(),
    };
    Ok(names
        .iter()
        .map(|name| Ripoti {
            kazi: name.to_string(),
            hatua: bounds.steps_of(name),
            kumbukumbu: bounds.memory_of(name),
        })
        .collect())
}

struct Checker<'m> {
    module: &'m Module,
    strict: &'m HashMap<&'m str, &'m Function>,
    errors: Vec<Kosa>,
    current: &'m str,
}

impl<'m> Checker<'m> {
    fn fail(&mut self, line: usize, sababu: impl Into<String>) {
        self.errors.push(Kosa {
            kazi: self.current.to_string(),
            mstari: line,
            sababu: sababu.into(),
        });
    }

    fn signature(&mut self, f: &Function) {
        for p in &f.params {
            let t = p.ty.name.replace(' ', "");
            if !matches!(t.as_str(), "Namba" | "Ukweli" | "Orodha<Namba>") {
                self.fail(
                    p.line,
                    format!(
                        "aina ya '{}' ni '{t}': Namba, Ukweli au Orodha<Namba> tu",
                        p.name
                    ),
                );
            }
        }
        let r = f.return_type.name.replace(' ', "");
        if !matches!(r.as_str(), "Namba" | "Ukweli" | "Tupu") {
            self.fail(
                f.line,
                format!("aina ya kurudisha ni '{r}': Namba, Ukweli au Tupu tu"),
            );
        }
    }

    /// `loop_vars`: the counters of the enclosing loops, which the body may not assign.
    fn block(&mut self, block: &Block, loop_vars: &mut Vec<asili_parser::Name>) {
        for stmt in &block.statements {
            match stmt {
                Stmt::While { line, .. } => {
                    self.fail(*line, "kitanzi cha `wakati` hakina kikomo kinachojulikana")
                }
                Stmt::For {
                    var,
                    mode,
                    body,
                    line,
                    ..
                } => match mode {
                    ForMode::InExpr(_) => self.fail(
                        *line,
                        "kitanzi cha `kwa ... katika` hakina kikomo kinachojulikana",
                    ),
                    ForMode::Range { start, end } => {
                        if self.constant(*start).is_none() || self.constant(*end).is_none() {
                            self.fail(
                                *line,
                                "mipaka ya kitanzi lazima ijulikane wakati wa kujenga \
                                 (namba au thabiti)",
                            );
                        }
                        loop_vars.push(*var);
                        self.block(body, loop_vars);
                        loop_vars.pop();
                    }
                },
                Stmt::Assign { name, line, .. } if loop_vars.contains(name) => self.fail(
                    *line,
                    format!("kigeuzi cha kitanzi '{name}' hakiwezi kubadilishwa ndani yake"),
                ),
                Stmt::If {
                    then_block,
                    else_if,
                    else_block,
                    ..
                } => {
                    self.block(then_block, loop_vars);
                    for (_, b) in else_if {
                        self.block(b, loop_vars);
                    }
                    if let Some(b) = else_block {
                        self.block(b, loop_vars);
                    }
                }
                Stmt::Match { line, .. }
                | Stmt::LetPattern { line, .. }
                | Stmt::Drop { line, .. } => self.fail(
                    *line,
                    "`linganisha`, muundo wa `weka` na `tupa` haviruhusiwi",
                ),
                _ => {}
            }
        }
    }

    /// The value of `e` when it is known at build time: number literals, `thabiti` number
    /// constants and `+ - * //` on them.
    fn constant(&self, e: ExprId) -> Option<f64> {
        constant(self.module, e)
    }

    /// Every instruction of the strict function `name` is one that cannot allocate or run for
    /// an unbounded time.
    fn code(&mut self, program: &BytecodeProgram, name: &str, line: usize) {
        let Some(f) = program.find_function(name) else {
            return;
        };
        for op in &f.code {
            let allowed = match op {
                // Checked in the source (`callees`).
                Opcode::Call(_) => true,
                Opcode::Mov { .. }
                | Opcode::Add { .. }
                | Opcode::Sub { .. }
                | Opcode::Mul { .. }
                | Opcode::Div { .. }
                | Opcode::Rem { .. }
                | Opcode::Pow { .. }
                | Opcode::BitAnd { .. }
                | Opcode::BitOr { .. }
                | Opcode::BitXor { .. }
                | Opcode::Shl { .. }
                | Opcode::Shr { .. }
                | Opcode::Neg { .. }
                | Opcode::BitNot { .. }
                | Opcode::Not { .. }
                | Opcode::Floor { .. }
                | Opcode::Ceil { .. }
                | Opcode::Cmp { .. }
                | Opcode::Jump { .. }
                | Opcode::JumpIfFalse { .. }
                | Opcode::JumpIfTrue { .. }
                | Opcode::JumpIfNot { .. }
                | Opcode::ForStep { .. }
                | Opcode::Trunc { .. }
                | Opcode::ListGet { .. }
                | Opcode::ListSet { .. }
                | Opcode::ListLen { .. }
                | Opcode::ListMov { .. }
                | Opcode::ReturnTupu
                | Opcode::CheckDepth
                | Opcode::Line { .. } => true,
                Opcode::Return { src } => src.ty != Ty::Val,
                _ => false,
            };
            if !allowed {
                let what = op.name();
                self.fail(
                    line,
                    format!("operesheni '{what}' inaweza kutenga kumbukumbu au kuchukua muda usio na kikomo"),
                );
            }
        }
    }

    /// No strict function reaches itself through calls.
    fn recursion(&mut self, names: &[&'m str]) {
        let calls: HashMap<&str, Vec<&str>> = names
            .iter()
            .map(|&n| (n, callees(self.module, &self.strict[n].body)))
            .collect();
        // Depth-first search for a cycle among strict functions.
        let mut state: HashMap<&str, u8> = HashMap::new(); // 1: on the path, 2: done
        fn visit<'a>(
            n: &'a str,
            calls: &HashMap<&'a str, Vec<&'a str>>,
            state: &mut HashMap<&'a str, u8>,
        ) -> Option<&'a str> {
            match state.get(n) {
                Some(1) => return Some(n),
                Some(_) => return None,
                None => {}
            }
            state.insert(n, 1);
            for &c in calls.get(n).into_iter().flatten() {
                if let Some(cycle) = visit(c, calls, state) {
                    return Some(cycle);
                }
            }
            state.insert(n, 2);
            None
        }
        for &n in names {
            if let Some(at) = visit(n, &calls, &mut state) {
                self.current = at;
                let line = self.strict.get(at).map_or(0, |f| f.line);
                self.fail(
                    line,
                    "kujiita (moja kwa moja au kupitia kazi nyingine) hakuruhusiwi",
                );
                return;
            }
        }
    }
}

/// `e`'s value when known at build time.
fn constant(module: &Module, e: ExprId) -> Option<f64> {
    Some(match &module[e] {
        Expr::Number(s) => crate::value::parse_number(s),
        Expr::Group(x) => constant(module, *x)?,
        Expr::Ident { name, .. } => {
            let c = module.constants.iter().find(|c| c.name == name.as_str())?;
            constant(module, c.value)?
        }
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
            ..
        } => -constant(module, *expr)?,
        Expr::Binary {
            left, op, right, ..
        } => {
            let (a, b) = (constant(module, *left)?, constant(module, *right)?);
            match op {
                BinaryOp::Add => a + b,
                BinaryOp::Sub => a - b,
                BinaryOp::Mul => a * b,
                _ => return None,
            }
        }
        Expr::Call { callee, args, .. } => match (&module[*callee], args.as_slice()) {
            // `a // b` is `sakafu(a / b)`.
            (Expr::Ident { name, .. }, [x]) if name.as_str() == "sakafu" => match &module[*x] {
                Expr::Binary {
                    left,
                    op: BinaryOp::Div,
                    right,
                    ..
                } => (constant(module, *left)? / constant(module, *right)?).floor(),
                _ => constant(module, *x)?.floor(),
            },
            _ => return None,
        },
        _ => return None,
    })
}

/// The program functions `block` calls by name.
fn callees<'m>(module: &'m Module, block: &Block) -> Vec<&'m str> {
    let mut out = Vec::new();
    let mut roots = Vec::new();
    collect_roots(block, &mut roots);
    for root in roots {
        for id in module.exprs.descendants(root) {
            if let Expr::Call { callee, .. } = &module[id] {
                if let Expr::Ident { name, .. } = &module[*callee] {
                    if let Some(f) = module.functions.iter().find(|f| f.name == *name) {
                        out.push(f.name.as_str());
                    }
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn collect_roots(block: &Block, out: &mut Vec<ExprId>) {
    let mut b = block.clone();
    b.map_expr_roots(&mut |id| {
        out.push(id);
        id
    });
}

/// Worst-case steps and memory of strict functions (checked first, so no recursion and every
/// loop bound is known).
struct Bounds<'m> {
    module: &'m Module,
    strict: &'m HashMap<&'m str, &'m Function>,
    program: &'m BytecodeProgram,
    steps: HashMap<String, u64>,
    memory: HashMap<String, u64>,
}

impl Bounds<'_> {
    fn steps_of(&mut self, name: &str) -> u64 {
        if let Some(&s) = self.steps.get(name) {
            return s;
        }
        let f = self.strict[name];
        let s = 1 + self.block(&f.body);
        self.steps.insert(name.to_string(), s);
        s
    }

    fn block(&mut self, block: &Block) -> u64 {
        block
            .statements
            .iter()
            .map(|s| self.stmt(s))
            .fold(0, u64::saturating_add)
    }

    fn stmt(&mut self, stmt: &Stmt) -> u64 {
        1u64.saturating_add(match stmt {
            Stmt::Let { value, .. }
            | Stmt::Assign { value, .. }
            | Stmt::Expr { expr: value, .. } => self.expr(*value),
            Stmt::Return { value, .. } => value.map_or(0, |v| self.expr(v)),
            Stmt::If {
                cond,
                then_block,
                else_if,
                else_block,
                ..
            } => {
                // Every condition may be tested; the costliest branch runs.
                let mut conds = self.expr(*cond);
                let mut worst = self.block(then_block);
                for (c, b) in else_if {
                    conds = conds.saturating_add(self.expr(*c));
                    worst = worst.max(self.block(b));
                }
                if let Some(b) = else_block {
                    worst = worst.max(self.block(b));
                }
                conds.saturating_add(worst)
            }
            Stmt::For {
                mode: ForMode::Range { start, end },
                body,
                ..
            } => {
                let a = constant(self.module, *start).unwrap_or(0.0);
                let b = constant(self.module, *end).unwrap_or(0.0);
                let trips = (b - a).ceil().max(0.0) as u64;
                // Each pass: the counter step and test, then the body.
                trips.saturating_mul(1 + self.block(body)).saturating_add(1)
            }
            _ => 0,
        })
    }

    fn expr(&mut self, root: ExprId) -> u64 {
        let mut total: u64 = 0;
        for id in self.module.exprs.descendants(root) {
            total = total.saturating_add(1);
            if let Expr::Call { callee, .. } = &self.module[id] {
                if let Expr::Ident { name, .. } = &self.module[*callee] {
                    if self.strict.contains_key(name.as_str()) {
                        total = total.saturating_add(self.steps_of(name.as_str()));
                    }
                }
            }
        }
        total
    }

    /// Bytes of the register files of `name`'s frame, plus the deepest chain of strict calls.
    fn memory_of(&mut self, name: &str) -> u64 {
        if let Some(&m) = self.memory.get(name) {
            return m;
        }
        let own = self.program.find_function(name).map_or(0, |f| {
            let list = std::mem::size_of::<crate::numlist::NumList>() as u64;
            8 * f.num_regs as u64 + list * f.list_regs as u64
        });
        let deepest = callees(self.module, &self.strict[name].body)
            .into_iter()
            .filter(|c| self.strict.contains_key(c))
            .map(|c| self.memory_of(c))
            .max()
            .unwrap_or(0);
        let m = own + deepest;
        self.memory.insert(name.to_string(), m);
        m
    }
}
