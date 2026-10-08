//! Integer interval analysis over the IR, used to delete comparisons whose outcome is fixed.
//!
//! The bytecode analysis (`native::analyze_numbers`) sees each register as one value over the
//! whole function. After lowering — and especially after full unrolling, where a counter is a
//! different constant in every copy — the IR exposes much tighter facts: a count reset to 0
//! and bumped at most once per copy of a 9-iteration loop is at most 9, so its bound checks
//! can never fail. This forward dataflow tracks `[lo, hi]` per integer
//! register (absent = unknown), narrows on branch edges, and widens at loop headers so it
//! terminates; comparisons it decides become constants for `fold_constants` to turn into jumps.

use super::ir::{Func, ICond, Inst, IntOp, Term, VReg};
use std::collections::HashMap;

type Range = (i64, i64);
type State = HashMap<VReg, Range>;

/// Visits of a block after which changing bounds widen to infinity.
const WIDEN_AFTER: u32 = 3;

fn fit(lo: i128, hi: i128) -> Option<Range> {
    Some((i64::try_from(lo).ok()?, i64::try_from(hi).ok()?))
}

/// Bounds from exact sums, where a side derived from an unbounded operand stays unbounded
/// (instead of the whole range being dropped); any other overflow gives up.
fn sat((lo, lo_inf): (i128, bool), (hi, hi_inf): (i128, bool)) -> Option<Range> {
    let lo = if lo_inf {
        i64::MIN
    } else {
        i64::try_from(lo).ok()?
    };
    let hi = if hi_inf {
        i64::MAX
    } else {
        i64::try_from(hi).ok()?
    };
    Some((lo, hi))
}

fn hull(a: Range, b: Range) -> Range {
    (a.0.min(b.0), a.1.max(b.1))
}

fn arith(op: IntOp, a: Option<Range>, b: Option<Range>) -> Option<Range> {
    let (a, b) = (a?, b?);
    let (al, ah, bl, bh) = (a.0 as i128, a.1 as i128, b.0 as i128, b.1 as i128);
    match op {
        // `i64::MIN`/`MAX` bounds mean "unbounded" and stay so.
        IntOp::Add => sat(
            (al + bl, a.0 == i64::MIN || b.0 == i64::MIN),
            (ah + bh, a.1 == i64::MAX || b.1 == i64::MAX),
        ),
        IntOp::Sub => sat(
            (al - bh, a.0 == i64::MIN || b.1 == i64::MAX),
            (ah - bl, a.1 == i64::MAX || b.0 == i64::MIN),
        ),
        IntOp::Mul => {
            let p = [al * bl, al * bh, ah * bl, ah * bh];
            fit(*p.iter().min()?, *p.iter().max()?)
        }
        IntOp::And if a.0 >= 0 && b.0 >= 0 => Some((0, a.1.min(b.1))),
        IntOp::And if a.0 >= 0 => Some((0, a.1)),
        IntOp::And if b.0 >= 0 => Some((0, b.1)),
        IntOp::Or | IntOp::Xor if a.0 >= 0 && b.0 >= 0 => {
            let bits = 64 - (a.1 | b.1).leading_zeros();
            fit(0, (1i128 << bits) - 1)
        }
        IntOp::Shl if a.0 >= 0 && b.0 >= 0 && b.1 <= 62 => {
            fit(0, ah << b.1).map(|r| (a.0 << b.0, r.1))
        }
        IntOp::Sar if b.0 >= 0 && b.1 <= 63 => {
            Some(((a.0 >> b.0).min(a.0 >> b.1), (a.1 >> b.0).max(a.1 >> b.1)))
        }
        IntOp::UDiv if a.0 >= 0 && b.0 > 0 => Some((a.0 / b.1, a.1 / b.0)),
        IntOp::URem if a.0 >= 0 && b.0 > 0 => Some((0, (b.1 - 1).min(a.1))),
        _ => None,
    }
}

/// Whether `a cond b` is decided by the ranges.
fn decide(cond: ICond, a: Range, b: Range) -> Option<bool> {
    let lt = |x: Range, y: Range| {
        if x.1 < y.0 {
            Some(true)
        } else if x.0 >= y.1 {
            Some(false)
        } else {
            None
        }
    };
    match cond {
        ICond::Lt => lt(a, b),
        ICond::Ge => lt(a, b).map(|t| !t),
        ICond::Gt => lt(b, a),
        ICond::Le => lt(b, a).map(|t| !t),
        ICond::Eq | ICond::Ne => {
            let eq = if a.0 == a.1 && b.0 == b.1 && a.0 == b.0 {
                Some(true)
            } else if a.1 < b.0 || b.1 < a.0 {
                Some(false)
            } else {
                None
            };
            eq.map(|e| e == (cond == ICond::Eq))
        }
        // Unsigned: decided like signed when both sides are non-negative.
        ICond::Ult if a.0 >= 0 && b.0 >= 0 => lt(a, b),
        ICond::Ule if a.0 >= 0 && b.0 >= 0 => lt(b, a).map(|t| !t),
        ICond::Ult | ICond::Ule => None,
    }
}

/// Narrow `x` knowing `x cond y` holds.
fn narrow(cond: ICond, x: Range, y: Range) -> Option<Range> {
    let r = match cond {
        ICond::Lt => (x.0, x.1.min(y.1.saturating_sub(1))),
        ICond::Le => (x.0, x.1.min(y.1)),
        ICond::Gt => (x.0.max(y.0.saturating_add(1)), x.1),
        ICond::Ge => (x.0.max(y.0), x.1),
        ICond::Eq => (x.0.max(y.0), x.1.min(y.1)),
        _ => x,
    };
    (r.0 <= r.1).then_some(r)
}

fn negate(c: ICond) -> Option<ICond> {
    Some(match c {
        ICond::Lt => ICond::Ge,
        ICond::Ge => ICond::Lt,
        ICond::Le => ICond::Gt,
        ICond::Gt => ICond::Le,
        ICond::Eq => ICond::Ne,
        ICond::Ne => ICond::Eq,
        ICond::Ult | ICond::Ule => return None,
    })
}

fn mirror(c: ICond) -> ICond {
    match c {
        ICond::Lt => ICond::Gt,
        ICond::Gt => ICond::Lt,
        ICond::Le => ICond::Ge,
        ICond::Ge => ICond::Le,
        other => other,
    }
}

struct Ctx<'a> {
    consts: &'a HashMap<VReg, i64>,
}

impl Ctx<'_> {
    fn get(&self, s: &State, v: VReg) -> Option<Range> {
        s.get(&v)
            .copied()
            .or_else(|| self.consts.get(&v).map(|&c| (c, c)))
    }

    fn step(&self, s: &mut State, inst: &Inst) {
        let r = match *inst {
            Inst::IConst { value, .. } => Some((value, value)),
            Inst::Mov { src, .. } => self.get(s, src),
            Inst::Int { op, a, b, .. } => arith(op, self.get(s, a), self.get(s, b)),
            Inst::IntImm { op, a, imm, .. } => {
                arith(op, self.get(s, a), Some((imm as i64, imm as i64)))
            }
            Inst::Neg { src, .. } => self
                .get(s, src)
                .and_then(|(l, h)| fit(-(h as i128), -(l as i128))),
            Inst::ICmp { .. } | Inst::ICmpImm { .. } | Inst::TestImm { .. } | Inst::FCmp { .. } => {
                Some((0, 1))
            }
            Inst::Popcnt { .. } => Some((0, 64)),
            Inst::Select { a, b, .. } => match (self.get(s, a), self.get(s, b)) {
                (Some(x), Some(y)) => Some(hull(x, y)),
                _ => None,
            },
            Inst::Call { ret32: true, .. } => Some((0, u32::MAX as i64)),
            _ => None,
        };
        for d in inst.defs() {
            match r {
                Some(r) if inst.defs().len() == 1 => {
                    s.insert(d, r);
                }
                _ => {
                    s.remove(&d);
                }
            }
        }
    }

    /// States on the true and false edges of a branch on `cond`, narrowed by the comparison
    /// that computed it when that is the block's last instruction.
    fn edge_states(&self, s: &State, insts: &[Inst], cond: VReg) -> (State, State) {
        let (mut t, mut f) = (s.clone(), s.clone());
        let Some(last) = insts.last() else {
            return (t, f);
        };
        let (c, a, b) = match *last {
            // (Only when the comparison did not overwrite its own operands.)
            Inst::ICmp { cond: c, dst, a, b } if dst == cond && dst != a && dst != b => {
                (c, a, Some(b))
            }
            Inst::ICmpImm {
                cond: c, dst, a, ..
            } if dst == cond && dst != a => (c, a, None),
            _ => return (t, f),
        };
        let imm = match *last {
            Inst::ICmpImm { imm, .. } => Some((imm as i64, imm as i64)),
            _ => None,
        };
        let rb = b.and_then(|b| self.get(s, b)).or(imm);
        let ra = self.get(s, a);
        let top = (i64::MIN, i64::MAX);
        let apply = |state: &mut State, c: ICond| {
            if let Some(n) = narrow(c, ra.unwrap_or(top), rb.unwrap_or(top)) {
                state.insert(a, n);
            }
            if let Some(b) = b {
                if let Some(n) = narrow(mirror(c), rb.unwrap_or(top), ra.unwrap_or(top)) {
                    state.insert(b, n);
                }
            }
        };
        apply(&mut t, c);
        if let Some(nc) = negate(c) {
            apply(&mut f, nc);
        }
        // An unbounded side narrowed to (MIN, MAX) carries no information.
        t.retain(|_, r| *r != top);
        f.retain(|_, r| *r != top);
        (t, f)
    }
}

/// States flowing out of block `b` along each successor edge, from entry state `s`.
fn successors_out(ctx: &Ctx, func: &Func, b: usize, mut s: State) -> Vec<(usize, State)> {
    for inst in &func.blocks[b].insts {
        ctx.step(&mut s, inst);
    }
    match func.blocks[b].term {
        Term::Jump(t) => vec![(t.0 as usize, s)],
        Term::Branch { cond, then_, else_ } => {
            let (t, f) = ctx.edge_states(&s, &func.blocks[b].insts, cond);
            vec![(then_.0 as usize, t), (else_.0 as usize, f)]
        }
        Term::Return(_) => vec![],
    }
}

/// Registers known in both states, over the hull of their ranges.
fn join(a: &State, b: &State) -> State {
    a.iter()
        .filter_map(|(v, r)| b.get(v).map(|o| (*v, hull(*r, *o))))
        .collect()
}

/// Block entry states of the analysis (`None`: unreachable), or `None` if it did not settle.
///
/// Iterates to a fixpoint widening only at loop heads (targets of retreating edges in reverse
/// postorder), then runs a few narrowing passes — recomputing each entry from its
/// predecessors without widening — which recovers bounds widening overshot (a loop counter
/// widened to `[0, ∞)` narrows back to `[0, 80]` from its exit test).
fn analyze(func: &Func, ctx: &Ctx) -> Option<Vec<Option<State>>> {
    let nb = func.blocks.len();
    // Reverse postorder and loop heads.
    let mut rpo_index = vec![usize::MAX; nb];
    let order = {
        let mut seen = vec![false; nb];
        let mut stack = vec![(0usize, 0usize)];
        seen[0] = true;
        let mut post = Vec::new();
        while let Some(&mut (b, ref mut i)) = stack.last_mut() {
            let succ = func.blocks[b].term.successors();
            if *i < succ.len() {
                let s = succ[*i].0 as usize;
                *i += 1;
                if !seen[s] {
                    seen[s] = true;
                    stack.push((s, 0));
                }
            } else {
                post.push(b);
                stack.pop();
            }
        }
        post.reverse();
        for (i, &b) in post.iter().enumerate() {
            rpo_index[b] = i;
        }
        post
    };
    let mut head = vec![false; nb];
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); nb];
    for &b in &order {
        for s in func.blocks[b].term.successors() {
            let s = s.0 as usize;
            preds[s].push(b);
            if rpo_index[s] <= rpo_index[b] {
                head[s] = true;
            }
        }
    }
    let mut ins: Vec<Option<State>> = vec![None; nb];
    let mut visits = vec![0u32; nb];
    ins[0] = Some(State::new());
    let mut work = std::collections::BTreeSet::from([0usize]);
    let mut steps = 0;
    while let Some(b) = work.pop_first() {
        steps += 1;
        if steps > 50 * nb + 1000 {
            return None; // give up rather than spend unbounded time
        }
        let s = ins[b].clone().expect("queued");
        for (succ, out) in successors_out(ctx, func, b, s) {
            let merged = match &ins[succ] {
                None => out,
                Some(old) => {
                    let mut m = join(old, &out);
                    if head[succ] && visits[succ] >= WIDEN_AFTER {
                        for (v, h) in m.iter_mut() {
                            let r = old[v];
                            if h.0 < r.0 {
                                h.0 = i64::MIN;
                            }
                            if h.1 > r.1 {
                                h.1 = i64::MAX;
                            }
                        }
                    }
                    m.retain(|_, h| *h != (i64::MIN, i64::MAX));
                    m
                }
            };
            if ins[succ].as_ref() != Some(&merged) {
                ins[succ] = Some(merged);
                visits[succ] += 1;
                work.insert(succ);
            }
        }
    }
    // Narrowing: every entry state is re-derived from its predecessors' current states. Each
    // pass keeps the states sound (it applies the transfer to sound inputs) and never wider.
    for _ in 0..NARROW_PASSES {
        for &b in order.iter().skip(1) {
            let mut acc: Option<State> = None;
            for &p in &preds[b] {
                let Some(ps) = ins[p].clone() else { continue };
                for (succ, out) in successors_out(ctx, func, p, ps) {
                    if succ == b {
                        acc = Some(match acc {
                            None => out,
                            Some(a) => join(&a, &out),
                        });
                    }
                }
            }
            if let (Some(mut new), Some(old)) = (acc, ins[b].as_ref()) {
                // Keep only facts at least as tight as before (monotone descent).
                new.retain(|v, r| old.get(v).is_none_or(|o| r.0 >= o.0 && r.1 <= o.1));
                for (v, r) in old {
                    new.entry(*v).or_insert(*r);
                }
                ins[b] = Some(new);
            }
        }
    }
    Some(ins)
}

/// Rounds of narrowing after the widened fixpoint.
const NARROW_PASSES: usize = 2;

/// Replace comparisons the ranges decide with constants; whether anything changed.
pub fn fold_ranges(func: &mut Func, consts: &HashMap<VReg, i64>) -> bool {
    let ctx = Ctx { consts };
    let Some(ins) = analyze(func, &ctx) else {
        return false;
    };
    let mut changed = false;
    for (b, entry) in ins.iter().enumerate() {
        let Some(mut s) = entry.clone() else {
            continue;
        };
        for i in 0..func.blocks[b].insts.len() {
            let decided = match func.blocks[b].insts[i] {
                Inst::ICmp { cond, dst, a, b: y } => ctx
                    .get(&s, a)
                    .zip(ctx.get(&s, y))
                    .and_then(|(ra, rb)| decide(cond, ra, rb))
                    .map(|t| (dst, t)),
                Inst::ICmpImm { cond, dst, a, imm } => ctx
                    .get(&s, a)
                    .and_then(|ra| decide(cond, ra, (imm as i64, imm as i64)))
                    .map(|t| (dst, t)),
                _ => None,
            };
            if let Some((dst, t)) = decided {
                func.blocks[b].insts[i] = Inst::IConst {
                    dst,
                    value: t as i64,
                };
                changed = true;
            }
            ctx.step(&mut s, &func.blocks[b].insts[i]);
        }
    }
    changed
}

/// Largest dividend range checked exhaustively when choosing a small multiplier.
const SMALL_DIVIDEND: i64 = 1 << 16;

/// `(m, s)` with `(a * m) >> s == a / d` for every `a` in `0..=hi`, `m` fitting an `i32`
/// immediate — verified for each `a`, not derived.
fn small_reciprocal(d: i64, hi: i64) -> Option<(i32, u8)> {
    (1u8..=31).find_map(|s| {
        let m = ((1i64 << s) + d - 1) / d;
        let fits = i32::try_from(m).is_ok() && m.checked_mul(hi).is_some();
        (fits && (0..=hi).all(|a| (a * m) >> s == a / d)).then_some((m as i32, s))
    })
}

/// Division and remainder by a constant of a dividend proven small and non-negative become a
/// multiply and shift (`p / 9` for `p` in `0..81` is `(p * 57) >> 9`), like a C compiler's
/// range-aware lowering; whether anything changed.
pub fn narrow_divisions(func: &mut Func, consts: &HashMap<VReg, i64>) -> bool {
    let ctx = Ctx { consts };
    let Some(ins) = analyze(func, &ctx) else {
        return false;
    };
    let mut changed = false;
    for (b, entry) in ins.iter().enumerate() {
        let Some(mut s) = entry.clone() else {
            continue;
        };
        let insts = std::mem::take(&mut func.blocks[b].insts);
        let mut out = Vec::with_capacity(insts.len());
        for inst in insts {
            let small = match inst {
                Inst::IntImm { op, dst, a, imm }
                    if imm > 1
                        && matches!(op, IntOp::UDiv | IntOp::URem | IntOp::SDiv | IntOp::SRem) =>
                {
                    ctx.get(&s, a)
                        .filter(|r| r.0 >= 0 && r.1 <= SMALL_DIVIDEND)
                        .and_then(|r| small_reciprocal(imm as i64, r.1))
                        .map(|(m, sh)| (op, dst, a, imm, m, sh))
                }
                _ => None,
            };
            ctx.step(&mut s, &inst);
            let Some((op, dst, a, d, m, sh)) = small else {
                out.push(inst);
                continue;
            };
            let mut fresh = || {
                func.classes.push(super::ir::Class::Int);
                VReg(func.classes.len() as u32 - 1)
            };
            let t = fresh();
            out.push(Inst::IntImm {
                op: IntOp::Mul,
                dst: t,
                a,
                imm: m,
            });
            if matches!(op, IntOp::UDiv | IntOp::SDiv) {
                out.push(Inst::IntImm {
                    op: IntOp::Sar,
                    dst,
                    a: t,
                    imm: sh as i32,
                });
            } else {
                let (q, qd) = (fresh(), fresh());
                out.extend([
                    Inst::IntImm {
                        op: IntOp::Sar,
                        dst: q,
                        a: t,
                        imm: sh as i32,
                    },
                    Inst::IntImm {
                        op: IntOp::Mul,
                        dst: qd,
                        a: q,
                        imm: d,
                    },
                    Inst::Int {
                        op: IntOp::Sub,
                        dst,
                        a,
                        b: qd,
                    },
                ]);
            }
            changed = true;
        }
        func.blocks[b].insts = out;
    }
    changed
}
