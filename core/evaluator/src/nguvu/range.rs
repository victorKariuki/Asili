//! Integer interval analysis over the IR, used to delete comparisons whose outcome is fixed.
//!
//! The bytecode analysis (`native::analyze_numbers`) sees each register as one value over the
//! whole function. After lowering — and especially after full unrolling, where a counter is a
//! different constant in every copy — the IR exposes much tighter facts: a count reset to 0
//! and bumped at most once per copy of a 9-iteration loop is at most 9, so its ±2^53
//! speculation guard can never fail. This forward dataflow tracks `[lo, hi]` per integer
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

fn hull(a: Range, b: Range) -> Range {
    (a.0.min(b.0), a.1.max(b.1))
}

fn arith(op: IntOp, a: Option<Range>, b: Option<Range>) -> Option<Range> {
    let (a, b) = (a?, b?);
    let (al, ah, bl, bh) = (a.0 as i128, a.1 as i128, b.0 as i128, b.1 as i128);
    match op {
        IntOp::Add => fit(al + bl, ah + bh),
        IntOp::Sub => fit(al - bh, ah - bl),
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
            Inst::ICmp { .. } | Inst::ICmpImm { .. } | Inst::FCmp { .. } => Some((0, 1)),
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

/// Replace comparisons the ranges decide with constants; whether anything changed.
pub fn fold_ranges(func: &mut Func, consts: &HashMap<VReg, i64>) -> bool {
    let ctx = Ctx { consts };
    let nb = func.blocks.len();
    let mut ins: Vec<Option<State>> = vec![None; nb];
    let mut visits = vec![0u32; nb];
    ins[0] = Some(State::new());
    let mut work = std::collections::BTreeSet::from([0usize]);
    let mut steps = 0;
    while let Some(b) = work.pop_first() {
        steps += 1;
        if steps > 50 * nb + 1000 {
            return false; // give up rather than spend unbounded time
        }
        let mut s = ins[b].clone().expect("queued");
        for inst in &func.blocks[b].insts {
            ctx.step(&mut s, inst);
        }
        let outs: Vec<(usize, State)> = match func.blocks[b].term {
            Term::Jump(t) => vec![(t.0 as usize, s)],
            Term::Branch { cond, then_, else_ } => {
                let (t, f) = ctx.edge_states(&s, &func.blocks[b].insts, cond);
                vec![(then_.0 as usize, t), (else_.0 as usize, f)]
            }
            Term::Return(_) => vec![],
        };
        for (succ, out) in outs {
            let merged = match &ins[succ] {
                None => out,
                Some(old) => {
                    let mut m = State::new();
                    for (v, r) in old {
                        if let Some(o) = out.get(v) {
                            let mut h = hull(*r, *o);
                            if visits[succ] >= WIDEN_AFTER {
                                if h.0 < r.0 {
                                    h.0 = i64::MIN;
                                }
                                if h.1 > r.1 {
                                    h.1 = i64::MAX;
                                }
                            }
                            if h != (i64::MIN, i64::MAX) {
                                m.insert(*v, h);
                            }
                        }
                    }
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
