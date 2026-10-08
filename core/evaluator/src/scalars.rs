//! Scalar replacement of `umbo` values: a struct held in a register that never leaves the
//! function — only built (`MakeStruct`), copied between such registers (`ValMov`) and read
//! field by field (`Field`, `FieldNum`) — is kept as one register per field instead, so building
//! it allocates nothing and reading a field is a register move. A field built from a number
//! (`BoxNum`) lives in a numeric register, so a loop over such structs is plain numeric code.
//!
//! The result is exactly the original's: every read of a replaced struct is preceded, on every
//! path, by a write of it (so the original never reads a missing struct), every struct in a
//! group of registers has the same type and fields, and a field read as a number (`FieldNum`)
//! is only replaced where that field always holds a number.

use crate::bytecode::{BytecodeFunc, Opcode, Reg, StoredConstant, Ty};
use std::collections::{BTreeMap, BTreeSet};

/// How an instruction touches a generic (value) register.
#[derive(Clone, Copy, PartialEq)]
enum Access {
    Read,
    Write,
}

/// Every value register `op` reads or writes.
fn val_access(op: &Opcode, f: &mut impl FnMut(Reg, Access)) {
    use Access::{Read, Write};
    let operand = |o: &crate::bytecode::Operand, a: Access, f: &mut dyn FnMut(Reg, Access)| {
        if o.ty == Ty::Val {
            f(o.reg, a);
        }
    };
    match op {
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
        | Opcode::MakeNumList { .. }
        | Opcode::ListRepeat { .. }
        | Opcode::ListGet { .. }
        | Opcode::ListSet { .. }
        | Opcode::ListPush { .. }
        | Opcode::ListRemove { .. }
        | Opcode::ListLen { .. }
        | Opcode::ListMov { .. }
        | Opcode::ReturnTupu
        | Opcode::CheckDepth
        | Opcode::Line { .. } => {}
        Opcode::ListGetTokeo { dst, .. }
        | Opcode::ListRemoveVal { dst, .. }
        | Opcode::ListToVal { dst, .. }
        | Opcode::ConstVal { dst, .. }
        | Opcode::BoxNum { dst, .. }
        | Opcode::BoxBool { dst, .. } => f(*dst, Write),
        Opcode::ListFromVal { src, .. }
        | Opcode::UnboxNum { src, .. }
        | Opcode::UnboxBool { src, .. }
        | Opcode::ValLen { src, .. }
        | Opcode::FieldNum { src, .. } => f(*src, Read),
        Opcode::ValMov { dst, src }
        | Opcode::ValUnary { dst, src, .. }
        | Opcode::Unwrap { dst, src }
        | Opcode::IterItems { dst, src }
        | Opcode::Jaribu { dst, src }
        | Opcode::Cast { dst, src, .. }
        | Opcode::Field { dst, src, .. } => {
            f(*src, Read);
            f(*dst, Write);
        }
        Opcode::ValBinary { dst, a, b, .. } => {
            f(*a, Read);
            f(*b, Read);
            f(*dst, Write);
        }
        Opcode::ValIndex { dst, base, idx, .. } => {
            f(*base, Read);
            f(*idx, Read);
            f(*dst, Write);
        }
        Opcode::ListMutate(call) => {
            call.args.iter().for_each(|r| f(*r, Read));
            f(call.dst, Write);
        }
        Opcode::MakeList { dst, items } => {
            items.iter().for_each(|r| f(*r, Read));
            f(*dst, Write);
        }
        Opcode::Call(call) => {
            call.args.iter().for_each(|a| operand(a, Read, f));
            operand(&call.dst, Write, f);
        }
        Opcode::CallBuiltin(call) => {
            call.args.iter().for_each(|r| f(*r, Read));
            f(call.dst, Write);
        }
        Opcode::CallMethod(call) => {
            f(call.recv, Read);
            call.args.iter().for_each(|r| f(*r, Read));
            f(call.dst, Write);
        }
        Opcode::MutMethod(call) => {
            f(call.recv, Read);
            f(call.recv, Write);
            call.args.iter().for_each(|r| f(*r, Read));
            f(call.dst, Write);
        }
        Opcode::Return { src } => operand(src, Read, f),
        Opcode::MakeStruct { dst, fields, .. } => {
            fields.iter().for_each(|(_, r)| f(*r, Read));
            f(*dst, Write);
        }
        Opcode::IterItem { dst, items, .. } => {
            f(*items, Read);
            f(*dst, Write);
        }
        Opcode::MakeEnum { dst, data, .. } => {
            data.iter().for_each(|r| f(*r, Read));
            f(*dst, Write);
        }
        Opcode::MatchPattern { src, binds, .. } => {
            f(*src, Read);
            binds.iter().for_each(|(_, r)| f(*r, Write));
        }
        Opcode::MakeMap { dst, entries } => {
            entries.iter().for_each(|(k, v)| {
                f(*k, Read);
                f(*v, Read);
            });
            f(*dst, Write);
        }
    }
}

/// Instructions that can follow `pc`.
fn successors(code: &[Opcode], pc: usize) -> Vec<usize> {
    match &code[pc] {
        Opcode::Jump { target } => vec![*target as usize],
        Opcode::Return { .. } | Opcode::ReturnTupu => vec![],
        op => {
            let mut next = vec![pc + 1];
            next.extend(crate::native::jump_target(op));
            next
        }
    }
}

/// The struct a group of registers holds: its type constant and its field-name constants.
type Shape = (u32, Vec<u32>);

/// Replace the structs of `f` that never leave it by their fields (see the module docs).
pub(crate) fn split_structs(f: &mut BytecodeFunc, constants: &[StoredConstant]) {
    let code = &f.code.clone();
    // Candidates: written only by `MakeStruct`/`ValMov`, read only by fields and `ValMov`.
    let mut ok: BTreeMap<Reg, bool> = BTreeMap::new();
    let params: BTreeSet<Reg> = f
        .params
        .iter()
        .filter(|p| p.ty == Ty::Val)
        .map(|p| p.reg)
        .collect();
    for op in code {
        val_access(op, &mut |r, a| {
            let allowed = match (op, a) {
                (Opcode::MakeStruct { dst, .. }, Access::Write) => *dst == r,
                (Opcode::ValMov { .. }, _) => true,
                (Opcode::Field { src, .. } | Opcode::FieldNum { src, .. }, Access::Read) => {
                    *src == r
                }
                _ => false,
            };
            let entry = ok.entry(r).or_insert(true);
            *entry &= allowed && !params.contains(&r);
        });
    }
    // A `ValMov` ties its two registers together: both stay only if both qualify.
    loop {
        let mut changed = false;
        for op in code {
            if let Opcode::ValMov { dst, src } = op {
                let both =
                    ok.get(dst).copied().unwrap_or(false) && ok.get(src).copied().unwrap_or(false);
                for r in [dst, src] {
                    if ok.get(r).copied().unwrap_or(false) && !both {
                        ok.insert(*r, false);
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    let candidates: BTreeSet<Reg> = ok.iter().filter(|(_, v)| **v).map(|(r, _)| *r).collect();
    if candidates.is_empty() {
        return;
    }
    // Group registers joined by `ValMov` (union-find) and give each group one shape.
    let mut parent: BTreeMap<Reg, Reg> = candidates.iter().map(|r| (*r, *r)).collect();
    fn root(parent: &mut BTreeMap<Reg, Reg>, r: Reg) -> Reg {
        let p = parent[&r];
        if p == r {
            return r;
        }
        let top = root(parent, p);
        parent.insert(r, top);
        top
    }
    for op in code {
        if let Opcode::ValMov { dst, src } = op {
            if candidates.contains(dst) {
                let (a, b) = (root(&mut parent, *dst), root(&mut parent, *src));
                parent.insert(a, b);
            }
        }
    }
    let mut shape: BTreeMap<Reg, Option<Shape>> = BTreeMap::new();
    let mut bad: BTreeSet<Reg> = BTreeSet::new();
    for op in code {
        if let Opcode::MakeStruct { dst, name, fields } = op {
            if !candidates.contains(dst) {
                continue;
            }
            let g = root(&mut parent, *dst);
            let this: Shape = (*name, fields.iter().map(|(n, _)| *n).collect());
            match shape.get(&g) {
                Some(Some(s)) if *s != this => {
                    bad.insert(g);
                }
                _ => {
                    shape.insert(g, Some(this));
                }
            }
        }
    }
    for r in &candidates {
        let g = root(&mut parent, *r);
        if !shape.contains_key(&g) {
            bad.insert(g); // never built here: nothing to replace
        }
    }
    // Every read must follow a write on every path (forward "definitely written").
    let all: BTreeSet<Reg> = candidates.clone();
    let mut state: Vec<Option<BTreeSet<Reg>>> = vec![None; code.len() + 1];
    state[0] = Some(BTreeSet::new());
    let mut work = vec![0usize];
    while let Some(pc) = work.pop() {
        if pc >= code.len() {
            continue;
        }
        let mut s = state[pc].clone().expect("queued with a state");
        let mut unsafe_read = Vec::new();
        val_access(&code[pc], &mut |r, a| {
            if !all.contains(&r) {
                return;
            }
            match a {
                Access::Read if !s.contains(&r) => unsafe_read.push(r),
                Access::Read => {}
                Access::Write => {}
            }
        });
        for r in unsafe_read {
            let g = root(&mut parent, r);
            bad.insert(g);
        }
        val_access(&code[pc], &mut |r, a| {
            if a == Access::Write && all.contains(&r) {
                s.insert(r);
            }
        });
        for next in successors(code, pc) {
            let merged = match &state[next] {
                None => s.clone(),
                Some(old) => old.intersection(&s).copied().collect(),
            };
            if state[next].as_ref() != Some(&merged) {
                state[next] = Some(merged);
                work.push(next);
            }
        }
    }
    let groups: BTreeMap<Reg, Shape> = shape
        .into_iter()
        .filter(|(g, _)| !bad.contains(g))
        .filter_map(|(g, s)| s.map(|s| (g, s)))
        .collect();
    if groups.is_empty() {
        return;
    }
    // Per group and field: numeric when every build of it boxes a number just before (no write
    // to that number in between, within the block), and every `FieldNum` of it then reads a
    // number. `numeric_src[pc][slot]` is that number's register.
    let leaders = crate::native::leaders(code).unwrap_or_default();
    let boxed_num = |pc: usize, v: Reg| -> Option<Reg> {
        let mut at = pc;
        while at > 0 && !leaders.contains(&at) {
            at -= 1;
            if let Opcode::BoxNum { dst, src } = &code[at] {
                if *dst == v {
                    let clobbered = code[at + 1..pc]
                        .iter()
                        .any(|op| crate::native::num_writes(op).contains(src));
                    return (!clobbered).then_some(*src);
                }
            }
            let mut writes_v = false;
            val_access(&code[at], &mut |r, a| {
                writes_v |= r == v && a == Access::Write
            });
            if writes_v {
                return None;
            }
        }
        None
    };
    let mut numeric: BTreeMap<(Reg, usize), bool> = BTreeMap::new();
    for (pc, op) in code.iter().enumerate() {
        if let Opcode::MakeStruct { dst, fields, .. } = op {
            if !candidates.contains(dst) {
                continue;
            }
            let g = root(&mut parent, *dst);
            if !groups.contains_key(&g) {
                continue;
            }
            for (slot, (_, v)) in fields.iter().enumerate() {
                let e = numeric.entry((g, slot)).or_insert(true);
                *e &= boxed_num(pc, *v).is_some();
            }
        }
    }
    // A read of a field the struct does not have, or a `FieldNum` of a field that is not always
    // a number, keeps the struct (its error is raised as before).
    let mut drop_groups = BTreeSet::new();
    let named = |k: u32, field: &str| matches!(constants.get(k as usize), Some(StoredConstant::Neno(n)) if n == field);
    for op in code {
        let (src, field, slot, number) = match op {
            Opcode::FieldNum {
                src, field, slot, ..
            } => (src, field, slot, true),
            Opcode::Field {
                src, field, slot, ..
            } => (src, field, slot, false),
            _ => continue,
        };
        if !candidates.contains(src) {
            continue;
        }
        let g = root(&mut parent, *src);
        let Some((_, names)) = groups.get(&g) else {
            continue;
        };
        let slot = *slot as usize;
        let declared = names.get(slot).is_some_and(|k| named(*k, field));
        let numeric_ok = !number || numeric.get(&(g, slot)).copied().unwrap_or(false);
        if !declared || !numeric_ok {
            drop_groups.insert(g);
        }
    }
    let groups: BTreeMap<Reg, Shape> = groups
        .into_iter()
        .filter(|(g, _)| !drop_groups.contains(g))
        .collect();
    if groups.is_empty() {
        return;
    }
    // Field registers, one set per struct register (the shape is its group's).
    let mut field_regs: BTreeMap<(Reg, usize), (bool, Reg)> = BTreeMap::new();
    for r in &candidates {
        let g = root(&mut parent, *r);
        let Some((_, names)) = groups.get(&g) else {
            continue;
        };
        for slot in 0..names.len() {
            let num = numeric.get(&(g, slot)).copied().unwrap_or(false);
            let reg = if num {
                f.num_regs += 1;
                f.num_regs - 1
            } else {
                f.val_regs += 1;
                f.val_regs - 1
            };
            field_regs.insert((*r, slot), (num, reg));
        }
    }
    // Rewrite.
    let old = std::mem::take(&mut f.code);
    // Structs with no fields stay as they are (nothing to gain, and no field register marks
    // them as replaced).
    let replaced = |r: Reg| field_regs.contains_key(&(r, 0));
    let mut out: Vec<Opcode> = Vec::with_capacity(old.len());
    let mut new_pc = vec![0u32; old.len() + 1];
    let slots = |r: Reg| -> usize {
        (0..)
            .take_while(|k| field_regs.contains_key(&(r, *k)))
            .count()
    };
    for (pc, op) in old.iter().enumerate() {
        new_pc[pc] = out.len() as u32;
        match op {
            Opcode::MakeStruct { dst, fields, .. } if replaced(*dst) => {
                for (slot, (_, v)) in fields.iter().enumerate() {
                    let (num, reg) = field_regs[&(*dst, slot)];
                    out.push(if num {
                        Opcode::Mov {
                            dst: reg,
                            src: boxed_num(pc, *v).expect("numeric field"),
                        }
                    } else {
                        Opcode::ValMov { dst: reg, src: *v }
                    });
                }
            }
            Opcode::ValMov { dst, src } if replaced(*dst) => {
                for slot in 0..slots(*dst) {
                    let (num, to) = field_regs[&(*dst, slot)];
                    let (_, from) = field_regs[&(*src, slot)];
                    out.push(if num {
                        Opcode::Mov { dst: to, src: from }
                    } else {
                        Opcode::ValMov { dst: to, src: from }
                    });
                }
            }
            Opcode::FieldNum { dst, src, slot, .. } if replaced(*src) => {
                let (_, reg) = field_regs[&(*src, *slot as usize)];
                out.push(Opcode::Mov {
                    dst: *dst,
                    src: reg,
                });
            }
            Opcode::Field { dst, src, slot, .. } if replaced(*src) => {
                let (num, reg) = field_regs[&(*src, *slot as usize)];
                out.push(if num {
                    Opcode::BoxNum {
                        dst: *dst,
                        src: reg,
                    }
                } else {
                    Opcode::ValMov {
                        dst: *dst,
                        src: reg,
                    }
                });
            }
            other => out.push(other.clone()),
        }
    }
    new_pc[old.len()] = out.len() as u32;
    for op in &mut out {
        match op {
            Opcode::Jump { target }
            | Opcode::JumpIfFalse { target, .. }
            | Opcode::JumpIfTrue { target, .. }
            | Opcode::JumpIfNot { target, .. }
            | Opcode::ForStep { target, .. } => *target = new_pc[*target as usize],
            _ => {}
        }
    }
    // Boxes nothing reads any more: drop them (each was a call into the host).
    let mut read: BTreeSet<Reg> = BTreeSet::new();
    for op in &out {
        val_access(op, &mut |r, a| {
            if a == Access::Read {
                read.insert(r);
            }
        });
    }
    let keep: Vec<bool> = out
        .iter()
        .map(|op| !matches!(op, Opcode::BoxNum { dst, .. } if !read.contains(dst)))
        .collect();
    if keep.iter().all(|k| *k) {
        f.code = out;
        return;
    }
    let mut renum = vec![0u32; out.len() + 1];
    let mut n = 0u32;
    for (i, k) in keep.iter().enumerate() {
        renum[i] = n;
        if *k {
            n += 1;
        }
    }
    renum[out.len()] = n;
    f.code = out
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(mut op, _)| {
            match &mut op {
                Opcode::Jump { target }
                | Opcode::JumpIfFalse { target, .. }
                | Opcode::JumpIfTrue { target, .. }
                | Opcode::JumpIfNot { target, .. }
                | Opcode::ForStep { target, .. } => *target = renum[*target as usize],
                _ => {}
            }
            op
        })
        .collect();
}
