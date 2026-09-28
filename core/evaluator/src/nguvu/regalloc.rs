//! Block layout and register allocation.
//!
//! * **Layout**: reverse postorder from the entry, with cold blocks (deoptimization exits and
//!   blocks that end the call) moved after all hot code, so hot paths fall through.
//! * **Liveness**: per-block dataflow, then precise live *ranges* per virtual register (a
//!   value is only live where some path still needs it; a cold exit that reads every register
//!   no longer makes every register live across the whole function).
//! * **Assignment**: priority-based greedy — registers weighted by use counts scaled by loop
//!   depth get a physical register first, checked against that register's already-assigned
//!   ranges; values live across a runtime call prefer callee-saved registers.
//!
//! Every virtual register also has a home stack slot: spilled registers live there, and
//! register-resident ones are saved there around runtime calls when caller-saved.

use super::ir::{Class, Func, Inst, VReg};
use std::collections::HashMap;

/// Where a virtual register lives: a machine register by its target number, or its stack slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loc {
    Int(u8),
    Float(u8),
    Slot,
}

/// The registers a target lets the allocator hand out (its code generator keeps a few more as
/// scratch), split by whether a runtime call preserves them.
pub struct Target {
    pub int_callee_saved: &'static [u8],
    pub int_caller_saved: &'static [u8],
    pub float_callee_saved: &'static [u8],
    pub float_caller_saved: &'static [u8],
}

impl Target {
    pub fn is_caller_saved(&self, loc: Loc) -> bool {
        match loc {
            Loc::Int(r) => !self.int_callee_saved.contains(&r),
            Loc::Float(r) => !self.float_callee_saved.contains(&r),
            Loc::Slot => false,
        }
    }
}

pub struct Allocation {
    pub loc: Vec<Loc>,
    /// Block emission order.
    pub order: Vec<usize>,
    /// For each call instruction `(block, index)`: registers live after it (excluding its result).
    pub live_across: HashMap<(usize, usize), Vec<VReg>>,
    /// How many times each virtual register is read.
    pub uses: Vec<u32>,
    /// Estimated loop nesting depth of each block.
    pub depth: Vec<u32>,
    /// Each register's home slot index (`u32::MAX`: it never needs one).
    pub slot: Vec<u32>,
    /// Number of home slots.
    pub slots: u32,
}

/// Reverse postorder from block 0, cold blocks (and anything only reachable through them)
/// last.
fn layout(func: &Func) -> Vec<usize> {
    let nb = func.blocks.len();
    let mut seen = vec![false; nb];
    let mut post = Vec::with_capacity(nb);
    // Iterative DFS; the first listed successor is explored first.
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    seen[0] = true;
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
    let (hot, cold): (Vec<usize>, Vec<usize>) = post.into_iter().partition(|&b| !func.cold[b]);
    hot.into_iter().chain(cold).collect()
}

pub fn allocate(func: &Func, target: &Target) -> Allocation {
    let n = func.classes.len();
    let nb = func.blocks.len();
    let order = layout(func);
    let rank: Vec<usize> = {
        let mut r = vec![usize::MAX; nb];
        for (i, &b) in order.iter().enumerate() {
            r[b] = i;
        }
        r
    };
    let mut uses = vec![0u32; n];

    // Loop depth from retreating edges in the layout order.
    let mut depth = vec![0u32; nb];
    for &b in &order {
        for s in func.blocks[b].term.successors() {
            let s = s.0 as usize;
            if rank[s] <= rank[b] {
                for &m in &order[rank[s]..=rank[b]] {
                    depth[m] += 1;
                }
            }
        }
    }

    // Use counts and spill weights (uses and defs scaled by loop depth).
    let mut weight = vec![0f64; n];
    for (b, block) in func.blocks.iter().enumerate() {
        let w = 10f64.powi(depth[b].min(6) as i32);
        let steps = block
            .insts
            .iter()
            .map(|i| (i.uses(), i.defs()))
            .chain(std::iter::once((block.term.uses(), vec![])));
        for (us, ds) in steps {
            for u in us {
                uses[u.0 as usize] += 1;
                weight[u.0 as usize] += w;
            }
            for d in ds {
                weight[d.0 as usize] += w;
            }
        }
    }
    let live_out = func.liveness().live_out;

    // Live ranges over layout positions: instruction i of a block reads at `2i` and writes at
    // `2i + 1`; the terminator reads at the block's last position.
    let mut ranges: Vec<Vec<(u32, u32)>> = vec![Vec::new(); n];
    let mut calls: Vec<u32> = Vec::new();
    let mut live_across = HashMap::new();
    let mut pos = 0u32;
    for &b in &order {
        let block = &func.blocks[b];
        let first = pos;
        let last = first + 2 * block.insts.len() as u32;
        // Registers live (going backwards) and where their current range ends.
        let mut open: HashMap<u32, u32> = live_out[b].iter().map(|v| (v.0, last + 1)).collect();
        for u in block.term.uses() {
            open.entry(u.0).or_insert(last);
        }
        for (i, inst) in block.insts.iter().enumerate().rev() {
            let p = first + 2 * i as u32;
            let defs = inst.defs();
            if matches!(inst, Inst::Call { .. } | Inst::CallDirect { .. }) {
                calls.push(p);
                let across: Vec<VReg> = open
                    .keys()
                    .filter(|v| !defs.iter().any(|d| d.0 == **v))
                    .map(|v| VReg(*v))
                    .collect();
                live_across.insert((b, i), across);
            }
            for d in &defs {
                let end = open.remove(&d.0).unwrap_or(p + 1);
                ranges[d.0 as usize].push((p + 1, end));
            }
            for u in inst.uses() {
                open.entry(u.0).or_insert(p);
            }
        }
        for (v, end) in open {
            ranges[v as usize].push((first, end));
        }
        pos = last + 2;
    }
    for r in ranges.iter_mut() {
        r.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(r.len());
        for &(s, e) in r.iter() {
            match merged.last_mut() {
                Some(last) if s <= last.1 + 1 => last.1 = last.1.max(e),
                _ => merged.push((s, e)),
            }
        }
        *r = merged;
    }
    calls.sort_unstable();
    let spans_call = |rs: &[(u32, u32)]| {
        rs.iter().any(|&(s, e)| {
            let i = calls.partition_point(|&c| c < s);
            // Live across call `c` means live at both its read and write positions.
            calls.get(i).is_some_and(|&c| c < e)
        })
    };
    let overlaps = |a: &[(u32, u32)], b: &[(u32, u32)]| {
        let (mut i, mut j) = (0, 0);
        while i < a.len() && j < b.len() {
            if a[i].0 <= b[j].1 && b[j].0 <= a[i].1 {
                return true;
            }
            if a[i].1 < b[j].1 {
                i += 1;
            } else {
                j += 1;
            }
        }
        false
    };

    // Coalescing: a copy between registers whose live ranges never overlap merges them into
    // one allocation unit, so both share a location and the copy disappears.
    let mut leader: Vec<usize> = (0..n).collect();
    fn find(leader: &mut [usize], v: usize) -> usize {
        let mut r = v;
        while leader[r] != r {
            r = leader[r];
        }
        leader[v] = r;
        r
    }
    for &b in &order {
        for inst in &func.blocks[b].insts {
            let Inst::Mov { dst, src } = inst else {
                continue;
            };
            let (x, y) = (
                find(&mut leader, dst.0 as usize),
                find(&mut leader, src.0 as usize),
            );
            // The incoming arguments keep their own units (the prologue stores them first).
            if x == y || x < 4 || y < 4 || overlaps(&ranges[x], &ranges[y]) {
                continue;
            }
            let (keep, gone) = (x.min(y), x.max(y));
            leader[gone] = keep;
            let moved = std::mem::take(&mut ranges[gone]);
            ranges[keep].extend(moved);
            ranges[keep].sort_unstable();
            weight[keep] += weight[gone];
        }
    }

    // Priority greedy assignment: densest-use units first.
    let mut todo: Vec<usize> = (0..n)
        .filter(|&v| leader[v] == v && !ranges[v].is_empty())
        .collect();
    todo.sort_by(|&a, &b| {
        let len = |v: usize| {
            ranges[v]
                .iter()
                .map(|(s, e)| (e - s + 1) as f64)
                .sum::<f64>()
        };
        let pa = weight[a] / len(a);
        let pb = weight[b] / len(b);
        pb.partial_cmp(&pa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    // Ranges assigned to each physical register (kept sorted and merged).
    let mut taken: HashMap<(bool, u8), Vec<(u32, u32)>> = HashMap::new();
    let mut loc = vec![Loc::Slot; n];
    for v in todo {
        let rs = &ranges[v];
        // Values live across a runtime call prefer registers it preserves.
        let (preserved, clobbered) = match func.classes[v] {
            Class::Int => (target.int_callee_saved, target.int_caller_saved),
            Class::Float => (target.float_callee_saved, target.float_caller_saved),
        };
        let (first, second) = if spans_call(rs) {
            (preserved, clobbered)
        } else {
            (clobbered, preserved)
        };
        let candidates: Vec<Loc> = first
            .iter()
            .chain(second)
            .map(|&r| match func.classes[v] {
                Class::Int => Loc::Int(r),
                Class::Float => Loc::Float(r),
            })
            .collect();
        for c in candidates {
            let key = match c {
                Loc::Int(r) => (false, r),
                Loc::Float(r) => (true, r),
                Loc::Slot => unreachable!(),
            };
            let used = taken.entry(key).or_default();
            if !overlaps(used, rs) {
                used.extend_from_slice(rs);
                used.sort_unstable();
                loc[v] = c;
                break;
            }
        }
    }
    for v in 0..n {
        let l = find(&mut leader, v);
        loc[v] = loc[l];
    }
    // Home slots only for registers that can be read from one: the incoming arguments (parked
    // by the prologue), anything not given a register, and values saved around calls.
    let mut needs_slot: Vec<bool> = (0..n).map(|v| v < 4 || loc[v] == Loc::Slot).collect();
    for v in live_across.values().flatten() {
        needs_slot[v.0 as usize] = true;
    }
    for inst in func.blocks.iter().flat_map(|b| &b.insts) {
        if let Inst::Call { args, .. } | Inst::CallDirect { args, .. } = inst {
            for a in args {
                needs_slot[a.0 as usize] = true;
            }
        }
    }
    let mut slots = 0;
    let slot = needs_slot
        .iter()
        .map(|&needed| {
            slots += needed as u32;
            if needed {
                slots - 1
            } else {
                u32::MAX
            }
        })
        .collect();
    Allocation {
        loc,
        order,
        live_across,
        uses,
        depth,
        slot,
        slots,
    }
}

/// Human-readable IR with locations, hottest blocks marked (`ASILI_NGUVU_IR` debugging aid).
pub fn dump(func: &Func, a: &Allocation) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let name = |v: VReg| match a.loc[v.0 as usize] {
        Loc::Int(r) => format!("v{}:r{r}", v.0),
        Loc::Float(r) => format!("v{}:f{r}", v.0),
        Loc::Slot => format!("v{}:mem", v.0),
    };
    for &b in &a.order {
        let _ = writeln!(out, "B{b} depth={} cold={}", a.depth[b], func.cold[b]);
        for i in &func.blocks[b].insts {
            let d: Vec<String> = i.defs().into_iter().map(name).collect();
            let u: Vec<String> = i.uses().into_iter().map(name).collect();
            let _ = writeln!(out, "  {} = {} {}", d.join(","), brief(i), u.join(", "));
        }
        let _ = writeln!(out, "  term {:?}", func.blocks[b].term);
    }
    out
}

/// Instruction name plus its non-register operands (op, condition, immediate, constant).
fn brief(i: &Inst) -> String {
    match i {
        Inst::IConst { value, .. } => format!("IConst {value}"),
        Inst::FConst { value, .. } => format!("FConst {value}"),
        Inst::Int { op, .. } => format!("{op:?}"),
        Inst::IntImm { op, imm, .. } => format!("{op:?}Imm {imm}"),
        Inst::ICmp { cond, .. } => format!("ICmp.{cond:?}"),
        Inst::ICmpImm { cond, imm, .. } => format!("ICmp.{cond:?}Imm {imm}"),
        Inst::TestImm { zero, imm, .. } => format!("Test{} {imm}", if *zero { "Z" } else { "NZ" }),
        Inst::FCmp { cond, .. } => format!("FCmp.{cond:?}"),
        Inst::Float { op, .. } => format!("F{op:?}"),
        Inst::Call { target, .. } => format!("Call {target:?}"),
        Inst::CallDirect { func, .. } => format!("CallDirect #{func}"),
        Inst::Load { offset, .. } | Inst::Store { offset, .. } => {
            let s = format!("{i:?}");
            format!("{} +{offset}", s.split(' ').next().unwrap_or(""))
        }
        _ => format!("{i:?}")
            .split([' ', '{', '('])
            .next()
            .unwrap_or("")
            .to_string(),
    }
}
