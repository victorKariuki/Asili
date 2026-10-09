//! IR optimizations run between lowering and code generation.
//!
//! * **Constant folding**: integer operations and comparisons whose operands are all constants
//!   (typically a fully unrolled loop's counter) become constants, and branches on a constant
//!   condition become jumps.
//! * **If-conversion**: `if c { n += 1 }` — a branch around a few integer instructions — runs
//!   the instructions unconditionally into fresh registers and commits them with `cmov`, so a
//!   data-dependent condition costs no misprediction.
//! * **Immediate folding**: an operand defined only by an `IConst` that fits the instruction's
//!   immediate form becomes part of the instruction (`add r, 9`, `cmp r, 81`, `shl r, 3`).
//! * **Local value reuse**: within a block, an immediate operation already computed on the same
//!   operand is copied instead of recomputed, and `a % d` next to `a / d` becomes `a - q * d`.
//! * **Constant hoisting**: integer constants too wide for an immediate, and float constants,
//!   are materialized once on entry instead of at every use (a loop then compares against a
//!   register rather than rebuilding a 64-bit constant each iteration).
//! * **Dead code elimination**: pure instructions whose results are never read are dropped.

use super::ir::{Class, Func, ICond, Inst, IntOp, Term, VReg};
use asili_parser::FxHashMap as HashMap;

/// Run every pass over `func`. In debug builds (and so in every test) the IR is verified after
/// each pass, naming the pass that broke it; the caller verifies the final result always.
pub fn optimize(func: &mut Func) -> Result<(), String> {
    let check = |func: &Func, pass: &str| -> Result<(), String> {
        if cfg!(debug_assertions) {
            super::verify::verify(func).map_err(|e| format!("after {pass}: {e}"))?;
        }
        Ok(())
    };
    fold_all_constants(func);
    check(func, "fold_all_constants")?;
    let consts = constants(func);
    if super::range::fold_ranges(func, &consts) {
        check(func, "fold_ranges")?;
        fold_all_constants(func);
        check(func, "fold_all_constants")?;
    }
    merge_blocks(func);
    check(func, "merge_blocks")?;
    eliminate_dead_code(func); // folded guards leave their constants behind
    check(func, "eliminate_dead_code")?;
    if_convert(func);
    check(func, "if_convert")?;
    merge_blocks(func); // converted triangles leave straight chains behind
    check(func, "merge_blocks")?;
    fold_immediates(func);
    check(func, "fold_immediates")?;
    super::range::narrow_divisions(func, &constants(func));
    check(func, "narrow_divisions")?;
    select_to_arith(func);
    check(func, "select_to_arith")?;
    eliminate_dead_code(func); // so an `and` sits right before the test reading it
    fuse_bit_tests(func);
    check(func, "fuse_bit_tests")?;
    eliminate_dead_code(func); // drops the `and`s the tests absorbed
    if super::features::popcnt() {
        recognize_popcount(func);
        check(func, "recognize_popcount")?;
    }
    fold_all_constants(func); // `n = 0; n += popcnt(…)` is a copy
    check(func, "fold_all_constants")?;
    reuse_values(func);
    check(func, "reuse_values")?;
    hoist_wide_constants(func);
    check(func, "hoist_wide_constants")?;
    eliminate_dead_code(func);
    Ok(())
}

/// Registers whose every definition is an `IConst` of the same value, or a copy of such a
/// register (a loop bound re-initialized on each entry to the loop is as constant as one
/// written once).
fn constants(func: &Func) -> HashMap<VReg, i64> {
    // Optimistic fixpoint: `None` = not constant; copies resolve once their source does.
    let mut value: HashMap<VReg, Option<i64>> = HashMap::default();
    for round in 0.. {
        if round == 16 {
            return HashMap::default(); // not settled: assume nothing
        }
        let mut next: HashMap<VReg, Option<i64>> = HashMap::default();
        let mut merge = |d: VReg, v: Option<i64>| {
            let e = next.entry(d).or_insert(v);
            if *e != v {
                *e = None;
            }
        };
        for block in &func.blocks {
            for inst in &block.insts {
                match inst {
                    Inst::IConst { dst, value: v } => merge(*dst, Some(*v)),
                    // A copy of a register not (yet) known merges as unknown only when that
                    // register is known non-constant.
                    Inst::Mov { dst, src } => match value.get(src) {
                        Some(v) => merge(*dst, *v),
                        None if value.is_empty() => {}
                        None => merge(*dst, None),
                    },
                    _ => {
                        for d in inst.defs() {
                            merge(d, None);
                        }
                    }
                }
            }
        }
        if next == value {
            break;
        }
        value = next;
    }
    value
        .into_iter()
        .filter_map(|(v, k)| Some((v, k?)))
        .collect()
}

/// `a op b` exactly as the generated code computes it, or `None` where the IR leaves the result
/// undefined (shift out of range, division by zero or overflowing).
fn eval_int(op: IntOp, a: i64, b: i64) -> Option<i64> {
    Some(match op {
        IntOp::Add => a.wrapping_add(b),
        IntOp::Sub => a.wrapping_sub(b),
        IntOp::Mul => a.wrapping_mul(b),
        IntOp::And => a & b,
        IntOp::Or => a | b,
        IntOp::Xor => a ^ b,
        IntOp::Shl if (0..=63).contains(&b) => a << b,
        IntOp::Sar if (0..=63).contains(&b) => a >> b,
        IntOp::SDiv | IntOp::UDiv => a.checked_div(b)?,
        IntOp::SRem | IntOp::URem => a.checked_rem(b)?,
        IntOp::Shl | IntOp::Sar => return None,
    })
}

fn eval_cmp(cond: ICond, a: i64, b: i64) -> bool {
    match cond {
        ICond::Eq => a == b,
        ICond::Ne => a != b,
        ICond::Lt => a < b,
        ICond::Le => a <= b,
        ICond::Gt => a > b,
        ICond::Ge => a >= b,
        ICond::Ult => (a as u64) < (b as u64),
        ICond::Ule => (a as u64) <= (b as u64),
    }
}

/// One round of constant folding; whether anything changed.
fn fold_constants(func: &mut Func) -> bool {
    let global = constants(func);
    let mut changed = false;
    for block in &mut func.blocks {
        let mut k = Known::new(&global);
        for inst in &mut block.insts {
            let folded = match *inst {
                Inst::Mov { dst, src } => k.get(&src).map(|v| (dst, v)),
                // x + 0, 0 + x: a copy.
                Inst::Int {
                    op: IntOp::Add | IntOp::Or | IntOp::Xor,
                    dst,
                    a,
                    b,
                } if (k.get(&a) == Some(0)) != (k.get(&b) == Some(0)) => {
                    let src = if k.get(&a) == Some(0) { b } else { a };
                    *inst = Inst::Mov { dst, src };
                    changed = true;
                    None
                }
                Inst::Int { op, dst, a, b } => match (k.get(&a), k.get(&b)) {
                    (Some(x), Some(y)) => eval_int(op, x, y).map(|v| (dst, v)),
                    _ => None,
                },
                Inst::IntImm { op, dst, a, imm } => k
                    .get(&a)
                    .and_then(|x| eval_int(op, x, imm as i64))
                    .map(|v| (dst, v)),
                Inst::ICmp { cond, dst, a, b } => match (k.get(&a), k.get(&b)) {
                    (Some(x), Some(y)) => Some((dst, eval_cmp(cond, x, y) as i64)),
                    _ => None,
                },
                Inst::ICmpImm { cond, dst, a, imm } => k
                    .get(&a)
                    .map(|x| (dst, eval_cmp(cond, x, imm as i64) as i64)),
                Inst::Popcnt { dst, src } => k.get(&src).map(|x| (dst, x.count_ones() as i64)),
                _ => None,
            };
            if let Some((dst, value)) = folded {
                if func.classes[dst.0 as usize] == Class::Int {
                    *inst = Inst::IConst { dst, value };
                    changed = true;
                }
            }
            k.track(inst);
        }
        if let Term::Branch { cond, then_, else_ } = block.term {
            if let Some(c) = k.get(&cond) {
                block.term = Term::Jump(if c != 0 { then_ } else { else_ });
                changed = true;
            }
        }
    }
    changed
}

/// Constant registers at a point in a block: the function-wide ones plus values the block
/// itself has just set (an unrolled copy's counter is written once per copy, each time with a
/// different constant, so only local knowledge sees it).
struct Known<'a> {
    global: &'a HashMap<VReg, i64>,
    local: HashMap<VReg, Option<i64>>,
}

impl<'a> Known<'a> {
    fn new(global: &'a HashMap<VReg, i64>) -> Self {
        Known {
            global,
            local: HashMap::default(),
        }
    }

    fn get(&self, v: &VReg) -> Option<i64> {
        match self.local.get(v) {
            Some(known) => *known,
            None => self.global.get(v).copied(),
        }
    }

    /// Account for `inst` having executed.
    fn track(&mut self, inst: &Inst) {
        match inst {
            Inst::IConst { dst, value } => {
                self.local.insert(*dst, Some(*value));
            }
            _ => {
                for d in inst.defs() {
                    self.local.insert(d, None);
                }
            }
        }
    }
}

fn swapped(c: ICond) -> Option<ICond> {
    Some(match c {
        ICond::Eq => ICond::Eq,
        ICond::Ne => ICond::Ne,
        ICond::Lt => ICond::Gt,
        ICond::Le => ICond::Ge,
        ICond::Gt => ICond::Lt,
        ICond::Ge => ICond::Le,
        // No unsigned > / >= in the IR.
        ICond::Ult | ICond::Ule => return None,
    })
}

fn fold_immediates(func: &mut Func) {
    let global = constants(func);
    for block in &mut func.blocks {
        let mut k = Known::new(&global);
        for inst in &mut block.insts {
            let imm = |v: &VReg| k.get(v).and_then(|n| i32::try_from(n).ok());
            let replacement = match inst {
                Inst::Int { op, dst, a, b } => {
                    let foldable = |op: IntOp, n: i32| match op {
                        IntOp::Shl | IntOp::Sar => (0..=63).contains(&n),
                        // Division by a constant is its own lowering (see codegen).
                        IntOp::SDiv | IntOp::SRem | IntOp::UDiv | IntOp::URem => n > 0,
                        _ => true,
                    };
                    let commutative = matches!(
                        op,
                        IntOp::Add | IntOp::Mul | IntOp::And | IntOp::Or | IntOp::Xor
                    );
                    match (imm(b), imm(a)) {
                        (Some(n), _) if foldable(*op, n) => Some(Inst::IntImm {
                            op: *op,
                            dst: *dst,
                            a: *a,
                            imm: n,
                        }),
                        (_, Some(n)) if commutative => Some(Inst::IntImm {
                            op: *op,
                            dst: *dst,
                            a: *b,
                            imm: n,
                        }),
                        _ => None,
                    }
                }
                Inst::ICmp { cond, dst, a, b } => match (imm(b), imm(a)) {
                    (Some(n), _) => Some(Inst::ICmpImm {
                        cond: *cond,
                        dst: *dst,
                        a: *a,
                        imm: n,
                    }),
                    (_, Some(n)) => swapped(*cond).map(|c| Inst::ICmpImm {
                        cond: c,
                        dst: *dst,
                        a: *b,
                        imm: n,
                    }),
                    _ => None,
                },
                _ => None,
            };
            if let Some(r) = replacement {
                *inst = r;
            }
            k.track(inst);
        }
    }
}

/// Values already computed on the way into a block: `IntImm` results and list elements.
#[derive(Clone, Default)]
struct Available {
    /// (op, operand, immediate) → register currently holding that value.
    arith: HashMap<(IntOp, VReg, i32), VReg>,
    /// (list data, index, class) → register holding that element.
    elements: HashMap<(VReg, VReg, Class), VReg>,
    /// Register → the register it is a copy of (keys above use the original).
    copies: HashMap<VReg, VReg>,
}

impl Available {
    /// `d` was redefined: forget everything computed from it or held in it.
    fn kill(&mut self, d: VReg) {
        self.arith.retain(|(_, a, _), v| *a != d && *v != d);
        self.elements
            .retain(|(b, i, _), v| *b != d && *i != d && *v != d);
        self.copies.retain(|c, o| *c != d && *o != d);
    }

    fn original(&self, v: VReg) -> VReg {
        self.copies.get(&v).copied().unwrap_or(v)
    }
}

/// Reuse a value instead of recomputing it: an `IntImm` whose result a register still holds (or,
/// for a remainder, the matching quotient), and a list element loaded or stored earlier with no
/// store or call in between. Blocks with a single predecessor start from what it ends with, so
/// the arms of a branch reuse what the test computed.
fn reuse_values(func: &mut Func) {
    let nb = func.blocks.len();
    let mut preds = vec![0u32; nb];
    let mut pred_of = vec![usize::MAX; nb];
    for (b, block) in func.blocks.iter().enumerate() {
        for s in block.term.successors() {
            preds[s.0 as usize] += 1;
            pred_of[s.0 as usize] = b;
        }
    }
    // Reverse postorder, so a single predecessor is processed before its successor (unless
    // the edge is a back edge, whose target then starts empty).
    let mut order = Vec::with_capacity(nb);
    let mut seen = vec![false; nb];
    let mut stack = vec![(0usize, 0usize)];
    seen[0] = true;
    while let Some(&mut (b, ref mut next)) = stack.last_mut() {
        let succ = func.blocks[b].term.successors();
        if let Some(s) = succ.get(*next) {
            *next += 1;
            let s = s.0 as usize;
            if !seen[s] {
                seen[s] = true;
                stack.push((s, 0));
            }
        } else {
            order.push(b);
            stack.pop();
        }
    }
    order.reverse();
    let mut at_end: Vec<Option<Available>> = vec![None; nb];
    for bi in order {
        let mut avail = if preds[bi] == 1 && pred_of[bi] != bi {
            at_end[pred_of[bi]].clone().unwrap_or_default()
        } else {
            Available::default()
        };
        let insts = std::mem::take(&mut func.blocks[bi].insts);
        let mut out = Vec::with_capacity(insts.len());
        for inst in insts {
            let mut emitted = vec![inst.clone()];
            match inst {
                Inst::IntImm { op, dst, a, imm } => {
                    let divide = match op {
                        IntOp::SRem => Some(IntOp::SDiv),
                        IntOp::URem => Some(IntOp::UDiv),
                        _ => None,
                    };
                    let a = avail.original(a);
                    if let Some(&p) = avail.arith.get(&(op, a, imm)) {
                        emitted = vec![Inst::Mov { dst, src: p }];
                    } else if let Some(&q) = divide.and_then(|d| avail.arith.get(&(d, a, imm))) {
                        func.classes.push(Class::Int);
                        let t = VReg(func.classes.len() as u32 - 1);
                        emitted = vec![
                            Inst::IntImm {
                                op: IntOp::Mul,
                                dst: t,
                                a: q,
                                imm,
                            },
                            Inst::Int {
                                op: IntOp::Sub,
                                dst,
                                a,
                                b: t,
                            },
                        ];
                    }
                }
                Inst::LoadIndex {
                    dst, base, index, ..
                } => {
                    let key = (avail.original(base), avail.original(index), func.class(dst));
                    if let Some(&p) = avail.elements.get(&key) {
                        emitted = vec![Inst::Mov { dst, src: p }];
                    }
                }
                // Memory may have changed (any list may share storage with another).
                Inst::Store { .. }
                | Inst::StoreIndex { .. }
                | Inst::Call { .. }
                | Inst::CallDirect { .. } => avail.elements.clear(),
                _ => {}
            }
            for e in &emitted {
                for d in e.defs() {
                    avail.kill(d);
                }
            }
            match inst {
                Inst::IntImm { op, dst, a, imm } if dst != a => {
                    let a = avail.original(a);
                    avail.arith.insert((op, a, imm), dst);
                }
                Inst::LoadIndex {
                    dst, base, index, ..
                } if dst != base && dst != index => {
                    let key = (avail.original(base), avail.original(index), func.class(dst));
                    avail.elements.insert(key, dst);
                }
                Inst::StoreIndex {
                    src, base, index, ..
                } => {
                    let key = (avail.original(base), avail.original(index), func.class(src));
                    avail.elements.insert(key, src);
                }
                _ => {}
            }
            // Copies made here or found in the code: later keys look through them.
            for e in &emitted {
                if let Inst::Mov { dst, src } = e {
                    let src = avail.original(*src);
                    if src != *dst {
                        avail.copies.insert(*dst, src);
                    }
                }
            }
            out.extend(emitted);
        }
        func.blocks[bi].insts = out;
        at_end[bi] = Some(avail);
    }
}

fn hoist_wide_constants(func: &mut Func) {
    use asili_parser::FxHashSet as HashSet;
    // Single-definition constants only: the value is the same wherever the register is read.
    let mut defs: HashMap<VReg, u32> = HashMap::default();
    for block in &func.blocks {
        for inst in &block.insts {
            for d in inst.defs() {
                *defs.entry(d).or_default() += 1;
            }
        }
    }
    // Only constants read on hot paths: cold exits keep building theirs locally.
    let mut hot: HashSet<VReg> = HashSet::default();
    for (b, block) in func.blocks.iter().enumerate() {
        if !func.cold[b] {
            hot.extend(block.insts.iter().flat_map(|i| i.uses()));
            hot.extend(block.term.uses());
        }
    }
    let mut value: HashMap<VReg, (Class, u64)> = HashMap::default();
    for block in &func.blocks {
        for inst in &block.insts {
            let (dst, key) = match inst {
                Inst::IConst { dst, value } if i32::try_from(*value).is_err() => {
                    (*dst, (Class::Int, *value as u64))
                }
                Inst::FConst { dst, value } if value.to_bits() != 0 => {
                    (*dst, (Class::Float, value.to_bits()))
                }
                _ => continue,
            };
            if defs.get(&dst) == Some(&1) && hot.contains(&dst) {
                value.insert(dst, key);
            }
        }
    }
    if value.is_empty() {
        return;
    }
    let mut shared: HashMap<(Class, u64), VReg> = HashMap::default();
    let mut entry = Vec::new();
    let mut keys: Vec<(Class, u64)> = value
        .values()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    keys.sort_by_key(|(c, bits)| (*c == Class::Float, *bits));
    for (class, bits) in keys {
        func.classes.push(class);
        let v = VReg(func.classes.len() as u32 - 1);
        entry.push(match class {
            Class::Int => Inst::IConst {
                dst: v,
                value: bits as i64,
            },
            Class::Float => Inst::FConst {
                dst: v,
                value: f64::from_bits(bits),
            },
        });
        shared.insert((class, bits), v);
    }
    for block in &mut func.blocks {
        for inst in &mut block.insts {
            for u in inst.uses_mut() {
                if let Some(key) = value.get(u) {
                    *u = shared[key];
                }
            }
        }
        for u in block.term.uses_mut() {
            if let Some(key) = value.get(u) {
                *u = shared[key];
            }
        }
    }
    func.blocks[0].insts.splice(0..0, entry);
}

fn fold_all_constants(func: &mut Func) {
    for _ in 0..8 {
        if !fold_constants(func) {
            break;
        }
    }
}

/// Append a block to its only predecessor when that predecessor jumps straight to it (what a
/// folded guard leaves behind), so the pair is one block for if-conversion and layout.
fn merge_blocks(func: &mut Func) {
    let nb = func.blocks.len();
    // Predecessors among reachable blocks only (folding and if-conversion orphan blocks).
    let mut reachable = vec![false; nb];
    let mut stack = vec![0usize];
    reachable[0] = true;
    while let Some(b) = stack.pop() {
        for s in func.blocks[b].term.successors() {
            if !reachable[s.0 as usize] {
                reachable[s.0 as usize] = true;
                stack.push(s.0 as usize);
            }
        }
    }
    let mut preds = vec![0u32; nb];
    for (b, block) in func.blocks.iter().enumerate() {
        if reachable[b] {
            for s in block.term.successors() {
                preds[s.0 as usize] += 1;
            }
        }
    }
    for b in (0..nb).filter(|&b| reachable[b]) {
        while let Term::Jump(next) = func.blocks[b].term {
            let n = next.0 as usize;
            if n == b || n == 0 || preds[n] != 1 || func.cold[n] != func.cold[b] {
                break;
            }
            let moved = std::mem::take(&mut func.blocks[n].insts);
            let term = std::mem::replace(&mut func.blocks[n].term, Term::Return(super::ir::RT));
            func.blocks[b].insts.extend(moved);
            func.blocks[b].term = term;
            preds[n] = 0;
        }
    }
}

/// Longest conditional block turned into straight-line code.
const IF_CONVERT_MAX: usize = 4;

fn if_convert(func: &mut Func) {
    let nb = func.blocks.len();
    let mut preds = vec![0u32; nb];
    for block in &func.blocks {
        for s in block.term.successors() {
            preds[s.0 as usize] += 1;
        }
    }
    // Only registers live into the join need committing: an arm's temporaries are not, and a
    // live one is written on the path around the arm too, so the select reads a defined value.
    // (Liveness from before any conversion stays sound: a select only reads a register that
    // was already live there.)
    let live = func.liveness();
    for a in 0..nb {
        let Term::Branch { cond, then_, else_ } = func.blocks[a].term else {
            continue;
        };
        // Triangle: one arm is a small block that only falls into the other arm.
        let (side, join, taken_on_true) = match (
            &func.blocks[then_.0 as usize].term,
            &func.blocks[else_.0 as usize].term,
        ) {
            (Term::Jump(j), _) if *j == else_ => (then_.0 as usize, else_, true),
            (_, Term::Jump(j)) if *j == then_ => (else_.0 as usize, then_, false),
            _ => continue,
        };
        let insts = &func.blocks[side].insts;
        let convertible = side != a
            && preds[side] == 1
            && func.cold[side] == func.cold[a]
            && !insts.is_empty()
            && insts.len() <= IF_CONVERT_MAX
            && insts.iter().all(|i| match i {
                Inst::IConst { .. }
                | Inst::Mov { .. }
                | Inst::Neg { .. }
                | Inst::Not { .. }
                | Inst::ICmp { .. }
                | Inst::ICmpImm { .. }
                | Inst::Select { .. } => true,
                Inst::Int { op, .. } | Inst::IntImm { op, .. } => {
                    !matches!(op, IntOp::SDiv | IntOp::SRem | IntOp::UDiv | IntOp::URem)
                }
                _ => false,
            })
            && insts
                .iter()
                .flat_map(|i| i.defs())
                .all(|d| func.classes[d.0 as usize] == Class::Int && d != cond);
        if !convertible {
            continue;
        }
        // Compute into fresh registers, then commit each written register with a select.
        let mut renamed: HashMap<VReg, VReg> = HashMap::default();
        let mut moved = Vec::new();
        for mut inst in std::mem::take(&mut func.blocks[side].insts) {
            for u in inst.uses_mut() {
                if let Some(r) = renamed.get(u) {
                    *u = *r;
                }
            }
            let defs = inst.defs();
            for d in defs {
                func.classes.push(Class::Int);
                let fresh = VReg(func.classes.len() as u32 - 1);
                renamed.insert(d, fresh);
                rename_def(&mut inst, d, fresh);
            }
            moved.push(inst);
        }
        let mut commits: Vec<(VReg, VReg)> = renamed
            .into_iter()
            .filter(|(orig, _)| live.live_in[join.0 as usize].contains(*orig))
            .collect();
        commits.sort();
        for (orig, fresh) in commits {
            let (x, y) = if taken_on_true {
                (fresh, orig)
            } else {
                (orig, fresh)
            };
            moved.push(Inst::Select {
                dst: orig,
                cond,
                a: x,
                b: y,
            });
        }
        func.blocks[side].term = Term::Jump(join);
        func.blocks[a].insts.extend(moved);
        func.blocks[a].term = Term::Jump(join);
        preds[side] = 0;
    }
}

/// `v = select(c, v + 1, v)` with a 0/1 condition is `v = v + c` (a counted `if`): no
/// conditional move at all.
fn select_to_arith(func: &mut Func) {
    for block in &mut func.blocks {
        // Registers holding 0/1 flags / `v + 1` results, as last defined in this block.
        let mut flag: asili_parser::FxHashSet<VReg> = Default::default();
        let mut plus_one: HashMap<VReg, VReg> = HashMap::default();
        for inst in &mut block.insts {
            if let Inst::Select { dst, cond, a, b } = *inst {
                if b == dst && flag.contains(&cond) && plus_one.get(&a) == Some(&dst) {
                    *inst = Inst::Int {
                        op: IntOp::Add,
                        dst,
                        a: dst,
                        b: cond,
                    };
                }
            }
            for d in inst.defs() {
                flag.remove(&d);
                plus_one.remove(&d);
                plus_one.retain(|_, src| *src != d);
            }
            match *inst {
                Inst::ICmp { dst, .. }
                | Inst::ICmpImm { dst, .. }
                | Inst::TestImm { dst, .. }
                | Inst::FCmp { dst, .. } => {
                    flag.insert(dst);
                }
                Inst::Mov { dst, src } if dst != src => {
                    if let Some(&base) = plus_one.get(&src) {
                        if base != dst {
                            plus_one.insert(dst, base);
                        }
                    }
                }
                Inst::IntImm {
                    op: IntOp::Add,
                    dst,
                    a,
                    imm: 1,
                } if dst != a => {
                    plus_one.insert(dst, a);
                }
                _ => {}
            }
        }
    }
}

/// `t = a & k; c = t == 0` becomes one `test a, k` when `t` is dead afterwards.
fn fuse_bit_tests(func: &mut Func) {
    let live = func.liveness();
    let dead_after = |insts: &[Inst], term: &Term, out: &super::ir::RegSet, t: VReg| {
        for inst in insts {
            if inst.uses().contains(&t) {
                return false;
            }
            if inst.defs().contains(&t) {
                return true;
            }
        }
        !term.uses().contains(&t) && !out.contains(t)
    };
    for (block, out) in func.blocks.iter_mut().zip(&live.live_out) {
        for i in 1..block.insts.len() {
            let Inst::ICmpImm {
                cond: cond @ (ICond::Eq | ICond::Ne),
                dst,
                a: t,
                imm: 0,
            } = block.insts[i]
            else {
                continue;
            };
            let Inst::IntImm {
                op: IntOp::And,
                dst: and_dst,
                a,
                imm,
            } = block.insts[i - 1]
            else {
                continue;
            };
            if and_dst == t && a != t && dead_after(&block.insts[i + 1..], &block.term, out, t) {
                block.insts[i] = Inst::TestImm {
                    zero: cond == ICond::Eq,
                    dst,
                    a,
                    imm,
                };
            }
        }
    }
}

/// `n += (a & 1) == 0; n += (a & 2) == 0; …` — single-bit tests of one register summed into
/// one counter (an unrolled "count the free digits" loop) — is `n += popcnt(~a & mask)`.
fn recognize_popcount(func: &mut Func) {
    let mut reads = vec![0u32; func.classes.len()];
    for block in &func.blocks {
        for u in block
            .insts
            .iter()
            .flat_map(|i| i.uses())
            .chain(block.term.uses())
        {
            reads[u.0 as usize] += 1;
        }
    }
    for b in 0..func.blocks.len() {
        let mut i = 0;
        while i + 1 < func.blocks[b].insts.len() {
            let insts = &func.blocks[b].insts;
            // (tested register, counter, zero-test?) of the pair at `j`, and its bit.
            let pair = |j: usize| -> Option<(VReg, VReg, bool, i32)> {
                let Inst::TestImm {
                    zero,
                    dst: f,
                    a,
                    imm,
                } = *insts.get(j)?
                else {
                    return None;
                };
                let Inst::Int {
                    op: IntOp::Add,
                    dst: n,
                    a: x,
                    b: y,
                } = *insts.get(j + 1)?
                else {
                    return None;
                };
                let single = imm > 0 && (imm as u32).is_power_of_two();
                let adds_flag = (x == n && y == f) || (y == n && x == f);
                (single && adds_flag && n != f && n != a && a != f && reads[f.0 as usize] == 1)
                    .then_some((a, n, zero, imm))
            };
            let Some((a, n, zero, first)) = pair(i) else {
                i += 1;
                continue;
            };
            let mut mask = first;
            let mut len = 1;
            while let Some((a2, n2, z2, bit)) = pair(i + 2 * len) {
                if a2 != a || n2 != n || z2 != zero || mask & bit != 0 {
                    break;
                }
                mask |= bit;
                len += 1;
            }
            if len < 3 {
                i += 1;
                continue;
            }
            let fresh = |func: &mut Func| {
                func.classes.push(Class::Int);
                VReg(func.classes.len() as u32 - 1)
            };
            let mut replacement = Vec::new();
            let source = if zero {
                let t = fresh(func);
                replacement.push(Inst::Not { dst: t, src: a });
                t
            } else {
                a
            };
            let masked = fresh(func);
            let count = fresh(func);
            replacement.extend([
                Inst::IntImm {
                    op: IntOp::And,
                    dst: masked,
                    a: source,
                    imm: mask,
                },
                Inst::Popcnt {
                    dst: count,
                    src: masked,
                },
                Inst::Int {
                    op: IntOp::Add,
                    dst: n,
                    a: n,
                    b: count,
                },
            ]);
            let added = replacement.len();
            func.blocks[b].insts.splice(i..i + 2 * len, replacement);
            i += added;
        }
    }
}

/// Point `inst`'s definition of `from` at `to`.
fn rename_def(inst: &mut Inst, from: VReg, to: VReg) {
    let slot = match inst {
        Inst::IConst { dst, .. }
        | Inst::FConst { dst, .. }
        | Inst::Mov { dst, .. }
        | Inst::Int { dst, .. }
        | Inst::IntImm { dst, .. }
        | Inst::ICmpImm { dst, .. }
        | Inst::Neg { dst, .. }
        | Inst::Not { dst, .. }
        | Inst::Popcnt { dst, .. }
        | Inst::Float { dst, .. }
        | Inst::ICmp { dst, .. }
        | Inst::FCmp { dst, .. }
        | Inst::Select { dst, .. }
        | Inst::IntToFloat { dst, .. }
        | Inst::FloatToInt { dst, .. }
        | Inst::FloatBits { dst, .. }
        | Inst::BitsFloat { dst, .. }
        | Inst::Load { dst, .. }
        | Inst::LoadIndex { dst, .. } => dst,
        _ => return,
    };
    if *slot == from {
        *slot = to;
    }
}

fn eliminate_dead_code(func: &mut Func) {
    drop_unobserved(func);
    drop_dead_at_definition(func);
}

/// Remove pure instructions whose registers nothing observes: never read, or read only by
/// pure instructions writing that same register (`v = select(c, t, v)` or `v = v + 1` with
/// `v` otherwise unused — a cycle liveness alone cannot break).
fn drop_unobserved(func: &mut Func) {
    loop {
        let mut used = vec![false; func.classes.len()];
        for block in &func.blocks {
            for inst in &block.insts {
                let own = inst.defs();
                for u in inst.uses() {
                    if !(inst.is_pure() && own == [u]) {
                        used[u.0 as usize] = true;
                    }
                }
            }
            for u in block.term.uses() {
                used[u.0 as usize] = true;
            }
        }
        let mut removed = false;
        for block in &mut func.blocks {
            let before = block.insts.len();
            block
                .insts
                .retain(|inst| !inst.is_pure() || inst.defs().iter().any(|d| used[d.0 as usize]));
            removed |= block.insts.len() != before;
        }
        if !removed {
            break;
        }
    }
}

/// Remove pure instructions whose results are dead where they are computed (a register
/// reused by an unrolled copy is overwritten before anything reads this value).
fn drop_dead_at_definition(func: &mut Func) {
    // Backward sweep from each block's live-out set, dropping pure instructions whose results
    // are dead at that point. Repeat: removals free operands.
    loop {
        let live = func.liveness();
        let mut removed = false;
        for (b, out) in live.live_out.into_iter().enumerate() {
            let mut live = out;
            for u in func.blocks[b].term.uses() {
                live.insert(u);
            }
            let insts = std::mem::take(&mut func.blocks[b].insts);
            let mut kept = Vec::with_capacity(insts.len());
            for inst in insts.into_iter().rev() {
                let defs = inst.defs();
                if inst.is_pure() && !defs.iter().any(|d| live.contains(*d)) {
                    removed = true;
                    continue;
                }
                for d in &defs {
                    live.remove(*d);
                }
                for u in inst.uses() {
                    live.insert(u);
                }
                kept.push(inst);
            }
            kept.reverse();
            func.blocks[b].insts = kept;
        }
        if !removed {
            break;
        }
    }
}
