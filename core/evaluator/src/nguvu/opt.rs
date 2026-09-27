//! IR optimizations run between lowering and code generation.
//!
//! * **Immediate folding**: an operand defined only by an `IConst` that fits the instruction's
//!   immediate form becomes part of the instruction (`add r, 9`, `cmp r, 81`, `shl r, 3`).
//! * **Local value reuse**: within a block, an immediate operation already computed on the same
//!   operand is copied instead of recomputed, and `a % d` next to `a / d` becomes `a - q * d`.
//! * **Constant hoisting**: integer constants too wide for an immediate, and float constants,
//!   are materialized once on entry instead of at every use (a loop then compares against a
//!   register rather than rebuilding a 64-bit constant each iteration).
//! * **Dead code elimination**: pure instructions whose results are never read are dropped.

use super::ir::{Class, Func, ICond, Inst, IntOp, VReg};
use std::collections::HashMap;

pub fn optimize(func: &mut Func) {
    fold_immediates(func);
    reuse_values(func);
    hoist_wide_constants(func);
    eliminate_dead_code(func);
}

/// Registers whose every definition is an `IConst` of the same value, or a copy of such a
/// register (a loop bound re-initialized on each entry to the loop is as constant as one
/// written once).
fn constants(func: &Func) -> HashMap<VReg, i64> {
    // Optimistic fixpoint: `None` = not constant; copies resolve once their source does.
    let mut value: HashMap<VReg, Option<i64>> = HashMap::new();
    for round in 0.. {
        if round == 16 {
            return HashMap::new(); // not settled: assume nothing
        }
        let mut next: HashMap<VReg, Option<i64>> = HashMap::new();
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
    let k = constants(func);
    let imm = |v: &VReg| k.get(v).and_then(|n| i32::try_from(*n).ok());
    for block in &mut func.blocks {
        for inst in &mut block.insts {
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
        }
    }
}

fn reuse_values(func: &mut Func) {
    for bi in 0..func.blocks.len() {
        // (op, operand, immediate) → register currently holding that value.
        let mut avail: HashMap<(IntOp, VReg, i32), VReg> = HashMap::new();
        let insts = std::mem::take(&mut func.blocks[bi].insts);
        let mut out = Vec::with_capacity(insts.len());
        for inst in insts {
            let mut emitted = vec![inst.clone()];
            if let Inst::IntImm { op, dst, a, imm } = inst {
                let divide = match op {
                    IntOp::SRem => Some(IntOp::SDiv),
                    IntOp::URem => Some(IntOp::UDiv),
                    _ => None,
                };
                if let Some(&p) = avail.get(&(op, a, imm)) {
                    emitted = vec![Inst::Mov { dst, src: p }];
                } else if let Some(&q) = divide.and_then(|d| avail.get(&(d, a, imm))) {
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
            for e in &emitted {
                for d in e.defs() {
                    avail.retain(|(_, a, _), v| *a != d && *v != d);
                }
            }
            if let Inst::IntImm { op, dst, a, imm } = inst {
                if dst != a {
                    avail.insert((op, a, imm), dst);
                }
            }
            out.extend(emitted);
        }
        func.blocks[bi].insts = out;
    }
}

fn hoist_wide_constants(func: &mut Func) {
    use std::collections::HashSet;
    // Single-definition constants only: the value is the same wherever the register is read.
    let mut defs: HashMap<VReg, u32> = HashMap::new();
    for block in &func.blocks {
        for inst in &block.insts {
            for d in inst.defs() {
                *defs.entry(d).or_default() += 1;
            }
        }
    }
    // Only constants read on hot paths: cold exits keep building theirs locally.
    let mut hot: HashSet<VReg> = HashSet::new();
    for (b, block) in func.blocks.iter().enumerate() {
        if !func.cold[b] {
            hot.extend(block.insts.iter().flat_map(|i| i.uses()));
            hot.extend(block.term.uses());
        }
    }
    let mut value: HashMap<VReg, (Class, u64)> = HashMap::new();
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
    let mut shared: HashMap<(Class, u64), VReg> = HashMap::new();
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

fn eliminate_dead_code(func: &mut Func) {
    loop {
        let mut used = vec![false; func.classes.len()];
        for block in &func.blocks {
            for inst in &block.insts {
                for u in inst.uses() {
                    used[u.0 as usize] = true;
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
