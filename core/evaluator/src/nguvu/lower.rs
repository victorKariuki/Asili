//! Bytecode → IR, driven by the range analysis (`native::analyze_numbers`): registers proven to
//! hold exact integers become `Int` virtual registers, proven in-range list accesses skip their
//! bounds check, and every instruction the backend doesn't lower runs through `Runtime::exec`.
//! Nothing is speculated: native code never hands a call back to anything else part-way, so
//! every decision here is a proof.

use super::ir::{
    Block, Builder, Class, FCond, FloatOp, ICond, Inst, IntOp, RtFn, Term, VReg, FRAME, HOST, NUMS,
};
use crate::bytecode::{BytecodeFunc, BytecodeProgram, CallOp, CmpOp, Opcode, Reg, Ty};
use crate::native::{
    analyze_numbers, jump_target, leaders, list_writes, num_reads, num_writes, NumFact,
    STATUS_FAIL, STATUS_RETURN,
};
use crate::numlist::Kind;
use asili_parser::{FxHashMap as HashMap, FxHashSet as HashSet};

/// What the function being lowered may assume about the rest of the program.
pub struct Ctx<'a> {
    pub program: &'a BytecodeProgram,
    /// Functions with a direct native entry: calls to them skip the host.
    pub direct: &'a HashSet<usize>,
    /// Lower the direct entry — arguments `(host, registers)`, the result returned in a float
    /// register with status 0 — instead of the ordinary one.
    pub entry_direct: bool,
}

/// Whether a function's shape allows a direct entry: numbers in, a number out, and no list or
/// generic registers (so the only frame it needs is its numeric register file).
pub fn direct_signature(f: &BytecodeFunc) -> bool {
    f.list_regs == 0
        && f.val_regs == 0
        && matches!(f.ret, Ty::Num | Ty::Bool)
        && f.params.iter().all(|p| matches!(p.ty, Ty::Num | Ty::Bool))
}

/// Which instructions a path from the entry reaches (following jumps and fall-through).
fn reachable(code: &[Opcode]) -> Vec<bool> {
    let mut seen = vec![false; code.len()];
    let mut work = vec![0usize];
    while let Some(pc) = work.pop() {
        if pc >= code.len() || seen[pc] {
            continue;
        }
        seen[pc] = true;
        if let Some(t) = jump_target(&code[pc]) {
            work.push(t);
        }
        if !matches!(
            code[pc],
            Opcode::Jump { .. } | Opcode::Return { .. } | Opcode::ReturnTupu
        ) {
            work.push(pc + 1);
        }
    }
    seen
}

const SIGN: i64 = i64::MIN;
const ABS_MASK: i64 = i64::MAX;

struct Lower<'a> {
    b: Builder,
    function: &'a BytecodeFunc,
    index: usize,
    facts: Vec<NumFact>,
    ints: Vec<bool>,
    safe_index: asili_parser::FxHashSet<usize>,
    /// Bytecode numeric register → its virtual register.
    regs: Vec<VReg>,
    /// List register → (data pointer, length) virtual registers.
    lists: Vec<(VReg, VReg)>,
    /// For lists this function appends to: (the list's address, bytes of storage) — pushes
    /// that fit are made in place.
    list_room: Vec<Option<(VReg, VReg)>>,
    labels: HashMap<usize, Block>,
    leaders: std::collections::BTreeSet<usize>,
    cur_pc: usize,
    /// Bounds known for the integer result being stored (set by `arith` for the next
    /// `set_i`): a side that cannot pass ±2^53 needs no check.
    result_range: (f64, f64),
    /// How each list register's elements are stored (`NumAnalysis::list_kinds`).
    list_kinds: Vec<Kind>,
    ctx: &'a Ctx<'a>,
    /// A direct entry reached something only the host can do: no direct entry.
    failed: bool,
    /// Instructions some path from the entry reaches.
    reachable: Vec<bool>,
    /// Largest register buffer a direct call site needs, in bytes.
    call_buffer: u32,
    /// Numeric registers live on entry to each instruction (bytecode liveness).
    live_in: Vec<std::collections::BTreeSet<Reg>>,
    /// Registers some instruction writes; the others keep their frame value all call long.
    written: std::collections::BTreeSet<Reg>,
    /// Never-written registers holding a constant: used as immediates, never kept in registers.
    consts: HashMap<Reg, f64>,
}

/// Numeric registers an instruction reads, including control flow and returns.
fn reads(op: &Opcode) -> Vec<Reg> {
    let mut r = num_reads(op);
    match op {
        Opcode::JumpIfFalse { cond, .. } | Opcode::JumpIfTrue { cond, .. } => r.push(*cond),
        Opcode::JumpIfNot { a, b, .. } => r.extend([*a, *b]),
        Opcode::ForStep { ctr, end, .. } => r.extend([*ctr, *end]),
        Opcode::Return { src } if matches!(src.ty, Ty::Num | Ty::Bool) => r.push(src.reg),
        _ => {}
    }
    r
}

/// Numeric registers an instruction writes.
fn writes(op: &Opcode) -> Vec<Reg> {
    let mut w = num_writes(op);
    if let Opcode::ForStep { ctr, .. } = op {
        w.push(*ctr);
    }
    w
}

/// Live-in sets per instruction, by backward dataflow over the bytecode.
fn liveness(code: &[Opcode]) -> Vec<std::collections::BTreeSet<Reg>> {
    use std::collections::BTreeSet;
    let succ = |pc: usize| -> Vec<usize> {
        match &code[pc] {
            Opcode::Jump { target } => vec![*target as usize],
            Opcode::Return { .. } | Opcode::ReturnTupu => vec![],
            op => jump_target(op).into_iter().chain([pc + 1]).collect(),
        }
    };
    let mut live: Vec<BTreeSet<Reg>> = vec![BTreeSet::new(); code.len() + 1];
    let mut changed = true;
    while changed {
        changed = false;
        for pc in (0..code.len()).rev() {
            let mut set: BTreeSet<Reg> = BTreeSet::new();
            for s in succ(pc) {
                if let Some(l) = live.get(s) {
                    set.extend(l.iter().copied());
                }
            }
            for w in writes(&code[pc]) {
                set.remove(&w);
            }
            set.extend(reads(&code[pc]));
            if set != live[pc] {
                live[pc] = set;
                changed = true;
            }
        }
    }
    live
}

/// Lower one function, or `None` if its jump targets are malformed (or, for a direct entry,
/// if some instruction needs the host).
pub fn lower(index: usize, function: &BytecodeFunc, ctx: &Ctx) -> Option<super::ir::Func> {
    if ctx.entry_direct && !direct_signature(function) {
        return None;
    }
    let code = &function.code;
    let leaders = leaders(code)?;
    let analysis = analyze_numbers(function);
    let facts = analysis.regs;
    let ints: Vec<bool> = facts.iter().map(|f| f.exact_int()).collect();
    let mut b = Builder::new();
    let regs = ints
        .iter()
        .map(|&i| b.vreg(if i { Class::Int } else { Class::Float }))
        .collect();
    let lists = (0..function.list_regs)
        .map(|_| (b.vreg(Class::Int), b.vreg(Class::Int)))
        .collect();
    let pushed: std::collections::BTreeSet<Reg> = code
        .iter()
        .filter_map(|op| match op {
            Opcode::ListPush { list, .. } => Some(*list),
            _ => None,
        })
        .collect();
    let list_room = (0..function.list_regs)
        .map(|r| {
            pushed
                .contains(&r)
                .then(|| (b.vreg(Class::Int), b.vreg(Class::Int)))
        })
        .collect();
    let written: std::collections::BTreeSet<Reg> = code.iter().flat_map(writes).collect();
    let consts: HashMap<Reg, f64> = function
        .num_consts
        .iter()
        .copied()
        .filter(|(r, _)| !written.contains(r))
        .collect();
    let mut l = Lower {
        b,
        function,
        index,
        facts,
        ints,
        safe_index: analysis.safe_index,
        regs,
        lists,
        list_room,
        labels: HashMap::default(),
        leaders: leaders.clone(),
        cur_pc: 0,
        result_range: (f64::NEG_INFINITY, f64::INFINITY),
        list_kinds: analysis.list_kinds,
        live_in: liveness(code),
        written,
        consts,
        ctx,
        failed: false,
        reachable: reachable(code),
        call_buffer: 0,
    };
    let entry = l.b.block();
    for &pc in &leaders {
        let block = l.b.block();
        l.labels.insert(pc, block);
    }
    l.b.switch_to(entry);
    l.prologue();
    let first = l.label(0);
    l.b.terminate(Term::Jump(first));
    l.lower_range(0, code.len());
    if l.b.is_open() {
        let pc = code.len().saturating_sub(1);
        if ctx.entry_direct {
            // A direct entry returns only through `Return` with a value.
            if l.reachable.get(pc).copied().unwrap_or(false) {
                l.failed = true;
            }
            let v = l.b.iconst(((STATUS_FAIL << 32) | pc as u64) as i64);
            l.b.terminate(Term::Return(v));
        } else {
            l.ret_status(STATUS_RETURN, pc);
        }
    }
    // Leaders past the end of the code (after a final return) are never jumped to.
    let tail: Vec<(usize, Block)> = l
        .labels
        .iter()
        .filter(|(&pc, _)| pc >= code.len())
        .map(|(&pc, &block)| (pc, block))
        .collect();
    for (pc, block) in tail {
        // Never jumped to: any status will do.
        l.b.switch_to(block);
        let v =
            l.b.iconst(((STATUS_RETURN << 32) | pc.saturating_sub(1) as u64) as i64);
        l.b.terminate(Term::Return(v));
    }
    if l.failed {
        return None;
    }
    let call_buffer = l.call_buffer;
    let mut func = l.b.finish();
    func.call_buffer = call_buffer;
    func.args = if ctx.entry_direct {
        super::ir::DIRECT_ARGS
    } else {
        super::ir::ENTRY_ARGS
    };
    Some(func)
}

/// Most iterations a counted loop is fully unrolled for, and most instructions it may grow to.
const UNROLL_TRIPS: i64 = 16;
const UNROLL_BUDGET: usize = 256;

impl<'a> Lower<'a> {
    /// Lower instructions `from..to`, starting a new block at every jump target.
    fn lower_range(&mut self, from: usize, to: usize) {
        let code = &self.function.code;
        let mut pc = from;
        while pc < to {
            if self.leaders.contains(&pc) {
                let block = self.label(pc);
                if self.b.is_open() {
                    self.b.terminate(Term::Jump(block));
                }
                self.b.switch_to(block);
            } else if !self.b.is_open() {
                pc += 1;
                continue; // unreachable
            }
            if let Some(next) = self.unroll(pc) {
                pc = next;
                continue;
            }
            self.cur_pc = pc;
            self.instruction(pc, &code[pc]);
            pc += 1;
        }
    }

    /// Fully unroll the counted loop whose entry test is at `pc` when its bounds are constants
    /// and it is small: each copy of the body sees the counter as a constant, so everything
    /// derived from it folds (`1 << (v - 1)` becomes an immediate mask). Returns the pc to
    /// continue at, or `None` to lower the loop normally.
    fn unroll(&mut self, pc: usize) -> Option<usize> {
        let code = &self.function.code;
        let Opcode::JumpIfNot {
            op: CmpOp::Lt,
            a: ctr,
            b: end,
            target,
        } = code[pc]
        else {
            return None;
        };
        let exit = target as usize;
        let step = exit.checked_sub(1)?;
        match code.get(step)? {
            Opcode::ForStep {
                ctr: c,
                end: e,
                target: t,
            } if *c == ctr && *e == end && *t as usize == pc + 1 => {}
            _ => return None,
        }
        // Constant bounds, set right before the entry test.
        let bound = |at: usize, reg: Reg| match code.get(at)? {
            Opcode::Trunc { dst, src } if *dst == reg => self.consts.get(src).map(|v| v.trunc()),
            _ => None,
        };
        let (lo, hi) = (bound(pc.checked_sub(2)?, ctr)?, bound(pc - 1, end)?);
        if !(lo.abs() < 1e15 && hi.abs() < 1e15) {
            return None;
        }
        let trips = (hi - lo).max(0.0) as i64;
        let body = pc + 1..step;
        if trips > UNROLL_TRIPS || trips as usize * (body.len() + 1) > UNROLL_BUDGET {
            return None;
        }
        // A `vunja` leaves the copies without the counter's register being updated, so the
        // counter must be dead after the loop (it is a fresh temporary in practice).
        if self.live_in.get(exit).is_some_and(|l| l.contains(&ctr)) {
            return None;
        }
        // Single entry (only the loop itself jumps into the body) and the bounds untouched.
        let inside = |t: usize| t > pc && t <= step;
        for (at, op) in code.iter().enumerate() {
            let from_inside = at > pc && at <= step;
            if !from_inside && jump_target(op).is_some_and(inside) {
                return None;
            }
            if body.contains(&at) && (writes(op).contains(&ctr) || writes(op).contains(&end)) {
                return None;
            }
        }
        let mut region: Vec<usize> = self.leaders.range(pc + 1..step).copied().collect();
        region.push(step); // where each copy ends (and `endelea` jumps)
        for i in 0..trips {
            // Fresh blocks for this copy's jump targets; `continue` (the step) ends the copy.
            for &l in &region {
                let block = self.b.block();
                self.labels.insert(l, block);
            }
            self.consts.insert(ctr, lo + i as f64);
            self.lower_range(pc + 1, step);
            let next = self.label(step);
            if self.b.is_open() {
                self.b.terminate(Term::Jump(next));
            }
            self.b.switch_to(next);
            self.consts.remove(&ctr);
        }
        let after = self.label(exit);
        self.b.terminate(Term::Jump(after));
        Some(exit)
    }

    /// The callee of `call` if it can be entered directly from native code.
    fn direct_callee(&self, call: &CallOp) -> Option<&'a BytecodeFunc> {
        let index = call.function as usize;
        let callee = self.ctx.program.functions.get(index)?;
        let numeric = |ty: Ty| matches!(ty, Ty::Num | Ty::Bool);
        (self.ctx.direct.contains(&index)
            && numeric(call.dst.ty)
            && call.args.len() == callee.params.len()
            && call.args.iter().all(|a| numeric(a.ty)))
        .then_some(callee)
    }

    /// A call to a function with a direct entry: its registers go in a buffer in this frame,
    /// and native code calls native code, which returns the number in a register. When the
    /// call depth reaches the limit or the stack is low, the call goes through the host's call
    /// path instead (`RtFn::CallHost`: it grows the stack, or reports the depth error, still
    /// runs the callee's native code, and stores the number after the callee's registers).
    fn direct_call(&mut self, pc: usize, _op: &Opcode, call: &CallOp) {
        let callee = self.direct_callee(call).expect("direct callee");
        let result_offset = 8 * callee.num_regs as i32;
        self.call_buffer = self.call_buffer.max(8 * (callee.num_regs + 1));
        let buf = self.vreg(Class::Int);
        self.push(Inst::CallBuffer { dst: buf });
        for (arg, param) in call.args.iter().zip(&callee.params) {
            let v = self.get_f(arg.reg);
            self.push(Inst::Store {
                src: v,
                base: buf,
                offset: 8 * param.reg as i32,
            });
        }
        let depth = self.vreg(Class::Int);
        self.push(Inst::Load {
            dst: depth,
            base: HOST,
            offset: crate::native::DEPTH_OFFSET,
        });
        let max = self.b.iconst(crate::host::MAX_CALL_DEPTH as i64);
        let via_vm = self.b.cold_block();
        let result = self.vreg(Class::Float);
        let got = self.b.block();
        let fail = self.b.cold_block();
        // Two compare-and-branch pairs (each fuses), not one branch on their conjunction.
        let shallow = self.icmp(ICond::Lt, depth, max);
        let next = self.b.block();
        self.b.terminate(Term::Branch {
            cond: shallow,
            then_: next,
            else_: via_vm,
        });
        self.b.switch_to(next);
        let limit = self.vreg(Class::Int);
        self.push(Inst::Load {
            dst: limit,
            base: HOST,
            offset: crate::native::LIMIT_OFFSET,
        });
        let sp = self.vreg(Class::Int);
        self.push(Inst::StackPointer { dst: sp });
        let roomy = self.icmp(ICond::Ult, limit, sp);
        let fast = self.b.block();
        self.b.terminate(Term::Branch {
            cond: roomy,
            then_: fast,
            else_: via_vm,
        });

        // Through the host: anything but a plain return is a failure to pass on.
        self.b.switch_to(via_vm);
        let f = self.b.iconst(call.function as i64);
        let s = self
            .call(RtFn::CallHost, vec![HOST, f, buf], Some(Class::Int), false)
            .expect("status");
        self.push(Inst::Load {
            dst: result,
            base: buf,
            offset: result_offset,
        });
        let kind = self.vreg(Class::Int);
        self.push(Inst::IntImm {
            op: IntOp::Sar,
            dst: kind,
            a: s,
            imm: 32,
        });
        let ret = self.b.iconst(crate::native::STATUS_RETURN as i64);
        let ok = self.icmp(ICond::Eq, kind, ret);
        self.b.terminate(Term::Branch {
            cond: ok,
            then_: got,
            else_: fail,
        });

        self.b.switch_to(fast);
        let one = self.b.iconst(1);
        let deeper = self.int_op(IntOp::Add, depth, one);
        let set_depth = |l: &mut Self, v: VReg| {
            l.push(Inst::Store {
                src: v,
                base: HOST,
                offset: crate::native::DEPTH_OFFSET,
            })
        };
        set_depth(self, deeper);
        let status = self.vreg(Class::Int);
        self.push(Inst::CallDirect {
            func: call.function,
            args: vec![HOST, buf],
            dst: status,
            result,
        });
        set_depth(self, depth);
        let z = self.b.iconst(0);
        let ok = self.icmp(ICond::Eq, status, z);
        self.b.terminate(Term::Branch {
            cond: ok,
            then_: got,
            else_: fail,
        });

        self.b.switch_to(fail);
        let status = self.b.iconst(((STATUS_FAIL << 32) | pc as u64) as i64);
        self.b.terminate(Term::Return(status));

        self.b.switch_to(got);
        self.set_f(call.dst.reg, result);
    }

    fn label(&self, pc: usize) -> Block {
        self.labels[&pc]
    }

    fn int(&self, r: Reg) -> bool {
        self.ints[r as usize]
    }

    fn vreg(&mut self, class: Class) -> VReg {
        self.b.vreg(class)
    }

    fn push(&mut self, inst: Inst) {
        self.b.push(inst);
    }

    fn prologue(&mut self) {
        let f = self.function;
        let consts: HashMap<Reg, f64> = f.num_consts.iter().copied().collect();
        let mut num_params = asili_parser::FxHashSet::default();
        let mut list_params = asili_parser::FxHashSet::default();
        for p in &f.params {
            match p.ty {
                Ty::Num | Ty::Bool => {
                    num_params.insert(p.reg);
                }
                Ty::List => {
                    list_params.insert(p.reg);
                }
                Ty::Val => {}
            }
        }
        for r in 0..f.num_regs {
            let dst = self.regs[r as usize];
            if self.consts.contains_key(&r) {
                continue; // materialized at each use
            }
            if !self.live_in.first().is_some_and(|l| l.contains(&r)) {
                continue; // written before any read: no entry value needed
            }
            if let Some(n) = consts.get(&r) {
                if self.int(r) {
                    self.push(Inst::IConst {
                        dst,
                        value: *n as i64,
                    });
                } else {
                    self.push(Inst::FConst { dst, value: *n });
                }
            } else if num_params.contains(&r) {
                self.reload(r);
            } else if self.int(r) {
                self.push(Inst::IConst { dst, value: 0 });
            } else {
                self.push(Inst::FConst { dst, value: 0.0 });
            }
        }
        for r in 0..f.list_regs {
            if list_params.contains(&r) {
                self.refresh_list(r);
            } else {
                let (p, l) = self.lists[r as usize];
                self.push(Inst::IConst { dst: p, value: 0 });
                self.push(Inst::IConst { dst: l, value: 0 });
                if let Some((head, room)) = self.list_room[r as usize] {
                    // No room yet: the first push goes through the runtime, which sets both.
                    self.push(Inst::IConst {
                        dst: head,
                        value: 0,
                    });
                    self.push(Inst::IConst {
                        dst: room,
                        value: 0,
                    });
                }
            }
        }
    }

    // ----- register access -------------------------------------------------------------

    fn get_f(&mut self, r: Reg) -> VReg {
        if let Some(&n) = self.consts.get(&r) {
            return self.b.fconst(n);
        }
        let v = self.regs[r as usize];
        if self.int(r) {
            let t = self.vreg(Class::Float);
            self.push(Inst::IntToFloat { dst: t, src: v });
            t
        } else {
            v
        }
    }

    /// Integer value; Rust's saturating `as i64` for float registers.
    fn get_i(&mut self, r: Reg) -> VReg {
        if let Some(&n) = self.consts.get(&r) {
            // Saturating `as i64` — what the runtime helper computes for float registers.
            return self.b.iconst(n as i64);
        }
        let v = self.regs[r as usize];
        if self.int(r) {
            v
        } else {
            self.call(RtFn::FloatToIntSat, vec![v], Some(Class::Int), false)
                .expect("result")
        }
    }

    fn call(
        &mut self,
        target: RtFn,
        args: Vec<VReg>,
        ret: Option<Class>,
        ret32: bool,
    ) -> Option<VReg> {
        let dst = ret.map(|c| self.vreg(c));
        self.call_into(target, args, dst, ret32);
        dst
    }

    /// Call runtime function `target`, its result (if any) into `dst`.
    fn call_into(&mut self, target: RtFn, args: Vec<VReg>, dst: Option<VReg>, ret32: bool) {
        let table = self.runtime_table();
        self.push(Inst::Call {
            table,
            target,
            args,
            dst,
            ret32,
        });
    }

    /// The runtime table: an ordinary entry's first argument; a direct entry reads it from the
    /// host (only on the paths that call the runtime).
    fn runtime_table(&mut self) -> VReg {
        if !self.ctx.entry_direct {
            return super::ir::RT;
        }
        let rt = self.vreg(Class::Int);
        self.push(Inst::Load {
            dst: rt,
            base: HOST,
            offset: crate::native::RT_OFFSET,
        });
        rt
    }

    fn int_op(&mut self, op: IntOp, a: VReg, b: VReg) -> VReg {
        let dst = self.vreg(Class::Int);
        self.push(Inst::Int { op, dst, a, b });
        dst
    }

    fn icmp(&mut self, cond: ICond, a: VReg, b: VReg) -> VReg {
        let dst = self.vreg(Class::Int);
        self.push(Inst::ICmp { cond, dst, a, b });
        dst
    }

    fn fcmp(&mut self, cond: FCond, a: VReg, b: VReg) -> VReg {
        let dst = self.vreg(Class::Int);
        self.push(Inst::FCmp { cond, dst, a, b });
        dst
    }

    fn set_f(&mut self, r: Reg, v: VReg) {
        let dst = self.regs[r as usize];
        if self.int(r) {
            // Every value this register receives is an exact integer.
            self.push(Inst::FloatToInt { dst, src: v });
        } else if dst != v {
            self.push(Inst::Mov { dst, src: v });
        }
    }

    fn set_i(&mut self, r: Reg, v: VReg) {
        let dst = self.regs[r as usize];
        if self.int(r) {
            if dst != v {
                self.push(Inst::Mov { dst, src: v });
            }
        } else {
            self.push(Inst::IntToFloat { dst, src: v });
        }
    }

    fn spill(&mut self, r: Reg) {
        if !self.written.contains(&r) {
            return; // the frame already holds this register's only value
        }
        let v = self.get_f(r);
        self.push(Inst::Store {
            src: v,
            base: NUMS,
            offset: 8 * r as i32,
        });
    }

    fn reload(&mut self, r: Reg) {
        let t = self.vreg(Class::Float);
        self.push(Inst::Load {
            dst: t,
            base: NUMS,
            offset: 8 * r as i32,
        });
        self.set_f(r, t);
    }

    /// Register `src` as stored in a list of `kind`: the float, or (the analysis having proved
    /// it an exact integer of that kind) the integer.
    fn list_value(&mut self, kind: Kind, src: Reg) -> VReg {
        if kind == Kind::F64 {
            self.get_f(src)
        } else if self.int(src) {
            self.get_i(src)
        } else {
            // The analysis proved the value an exact integer: truncation is exact.
            let f = self.get_f(src);
            let t = self.vreg(Class::Int);
            self.push(Inst::FloatToInt { dst: t, src: f });
            t
        }
    }

    fn refresh_list(&mut self, r: Reg) {
        let (p, l) = self.lists[r as usize];
        let reg = self.b.iconst(r as i64);
        let kind = self.list_kinds[r as usize];
        let want = self.b.iconst(kind.code() as i64);
        self.call_into(RtFn::ListPtr, vec![FRAME, reg, want], Some(p), false);
        if kind != Kind::F64 {
            // The analysis saw every value stored into this list, so each fits `kind` and the
            // runtime always hands back a pointer. Null would mean that proof was wrong: stop
            // with an internal error rather than read the list at the wrong width.
            let z = self.b.iconst(0);
            let ok = self.icmp(ICond::Ne, p, z);
            let fine = self.b.block();
            let broken = self.b.cold_block();
            self.b.terminate(Term::Branch {
                cond: ok,
                then_: fine,
                else_: broken,
            });
            self.b.switch_to(broken);
            let status = self
                .b
                .iconst(((STATUS_FAIL << 32) | self.cur_pc as u64) as i64);
            self.b.terminate(Term::Return(status));
            self.b.switch_to(fine);
        }
        self.call_into(RtFn::ListLen, vec![FRAME, reg], Some(l), false);
        if let Some((head, room)) = self.list_room[r as usize] {
            self.call_into(RtFn::ListHead, vec![FRAME, reg], Some(head), false);
            self.call_into(RtFn::ListRoom, vec![FRAME, reg], Some(room), false);
        }
    }

    /// `dst = cond` (0/1).
    fn set_flag(&mut self, dst: Reg, cond: VReg) {
        let d = self.regs[dst as usize];
        if self.int(dst) {
            self.push(Inst::Mov { dst: d, src: cond });
        } else {
            self.push(Inst::IntToFloat { dst: d, src: cond });
        }
    }

    fn compare(&mut self, op: CmpOp, a: Reg, b: Reg) -> VReg {
        if self.int(a) && self.int(b) {
            let x = self.get_i(a);
            let y = self.get_i(b);
            let cond = match op {
                CmpOp::Lt => ICond::Lt,
                CmpOp::Le => ICond::Le,
                CmpOp::Gt => ICond::Gt,
                CmpOp::Ge => ICond::Ge,
                CmpOp::Eq => ICond::Eq,
                CmpOp::Ne => ICond::Ne,
            };
            self.icmp(cond, x, y)
        } else {
            let x = self.get_f(a);
            let y = self.get_f(b);
            let cond = match op {
                CmpOp::Lt => FCond::Olt,
                CmpOp::Le => FCond::Ole,
                CmpOp::Gt => FCond::Ogt,
                CmpOp::Ge => FCond::Oge,
                CmpOp::Eq => FCond::Oeq,
                // Rust's `!=` is true when either side is NaN.
                CmpOp::Ne => FCond::Une,
            };
            self.fcmp(cond, x, y)
        }
    }

    /// `r != 0`.
    fn truthy(&mut self, r: Reg) -> VReg {
        if self.int(r) {
            let v = self.get_i(r);
            let z = self.b.iconst(0);
            self.icmp(ICond::Ne, v, z)
        } else {
            let v = self.get_f(r);
            let z = self.b.fconst(0.0);
            self.fcmp(FCond::Une, v, z)
        }
    }

    fn ret_status(&mut self, status: u64, pc: usize) {
        if self.ctx.entry_direct && status == STATUS_RETURN {
            self.failed = true; // a direct entry returns only through `Return` with a value
        }
        let v = self.b.iconst(((status << 32) | pc as u64) as i64);
        self.b.terminate(Term::Return(v));
    }

    /// Run instruction `pc` in the host; leave the function if it ended the call.
    fn slow(&mut self, pc: usize, op: &Opcode) {
        if self.ctx.entry_direct {
            self.failed = true; // direct entries have no host frame to run it in
        }
        for r in num_reads(op) {
            self.spill(r);
        }
        let f = self.b.iconst(self.index as i64);
        let p = self.b.iconst(pc as i64);
        let s = self
            .call(RtFn::Exec, vec![HOST, FRAME, f, p], Some(Class::Int), true)
            .expect("status");
        let z = self.b.iconst(0);
        let c = self.icmp(ICond::Ne, s, z);
        let done = self.b.cold_block();
        let cont = self.b.block();
        self.b.terminate(Term::Branch {
            cond: c,
            then_: done,
            else_: cont,
        });
        self.b.switch_to(done);
        let sh = self.b.iconst(32);
        let hi = self.int_op(IntOp::Shl, s, sh);
        let pcv = self.b.iconst(pc as i64);
        let ret = self.int_op(IntOp::Or, hi, pcv);
        self.b.terminate(Term::Return(ret));
        self.b.switch_to(cont);
        for r in num_writes(op) {
            self.reload(r);
        }
        for r in list_writes(op) {
            self.refresh_list(r);
        }
    }

    /// Bounds-checked element index into `list` (out of range: the host raises the
    /// exact error and the function returns).
    fn element(&mut self, list: Reg, idx: Reg, pc: usize, op: &Opcode) -> VReg {
        let i0 = self.get_i(idx);
        let i = if self.int(idx) && self.facts[idx as usize].lo >= 0.0 {
            i0
        } else {
            let z = self.b.iconst(0);
            let neg = self.icmp(ICond::Lt, i0, z);
            let i = self.vreg(Class::Int);
            self.push(Inst::Select {
                dst: i,
                cond: neg,
                a: z,
                b: i0,
            });
            i
        };
        if self.safe_index.contains(&pc) {
            return i;
        }
        let (_, len) = self.lists[list as usize];
        let ok = self.icmp(ICond::Ult, i, len);
        let fast = self.b.block();
        let oob = self.b.cold_block();
        self.b.terminate(Term::Branch {
            cond: ok,
            then_: fast,
            else_: oob,
        });
        self.b.switch_to(oob);
        self.slow(pc, op);
        self.ret_status(STATUS_FAIL, pc);
        self.b.switch_to(fast);
        i
    }

    fn copysign(&mut self, mag: VReg, sign: VReg) -> VReg {
        let mb = self.vreg(Class::Int);
        self.push(Inst::FloatBits { dst: mb, src: mag });
        let am = self.b.iconst(ABS_MASK);
        let m = self.int_op(IntOp::And, mb, am);
        let sb = self.vreg(Class::Int);
        self.push(Inst::FloatBits { dst: sb, src: sign });
        let sm = self.b.iconst(SIGN);
        let s = self.int_op(IntOp::And, sb, sm);
        let bits = self.int_op(IntOp::Or, m, s);
        let f = self.vreg(Class::Float);
        self.push(Inst::BitsFloat { dst: f, src: bits });
        f
    }

    /// `a % b` with `fmod` semantics and an integer fast path.
    fn remainder_f(&mut self, a: VReg, b: VReg) -> VReg {
        let ai = self
            .call(RtFn::FloatToIntSat, vec![a], Some(Class::Int), false)
            .expect("int");
        let bi = self
            .call(RtFn::FloatToIntSat, vec![b], Some(Class::Int), false)
            .expect("int");
        let af = self.vreg(Class::Float);
        self.push(Inst::IntToFloat { dst: af, src: ai });
        let bf = self.vreg(Class::Float);
        self.push(Inst::IntToFloat { dst: bf, src: bi });
        let ea = self.fcmp(FCond::Oeq, af, a);
        let eb = self.fcmp(FCond::Oeq, bf, b);
        let zero = self.b.iconst(0);
        let nz = self.icmp(ICond::Ne, bi, zero);
        let m1 = self.b.iconst(-1);
        let nm = self.icmp(ICond::Ne, bi, m1);
        let ok1 = self.int_op(IntOp::And, ea, eb);
        let ok2 = self.int_op(IntOp::And, ok1, nz);
        let ok = self.int_op(IntOp::And, ok2, nm);
        let result = self.vreg(Class::Float);
        let fast = self.b.block();
        let slow = self.b.block();
        let merge = self.b.block();
        self.b.terminate(Term::Branch {
            cond: ok,
            then_: fast,
            else_: slow,
        });
        self.b.switch_to(fast);
        let r = self.int_op(IntOp::SRem, ai, bi);
        let rf = self.vreg(Class::Float);
        self.push(Inst::IntToFloat { dst: rf, src: r });
        // fmod's result carries the dividend's sign, including -0.0.
        let rc = self.copysign(rf, a);
        self.push(Inst::Mov {
            dst: result,
            src: rc,
        });
        self.b.terminate(Term::Jump(merge));
        self.b.switch_to(slow);
        let rs = self
            .call(RtFn::Fmod, vec![a, b], Some(Class::Float), false)
            .expect("fmod");
        self.push(Inst::Mov {
            dst: result,
            src: rs,
        });
        self.b.terminate(Term::Jump(merge));
        self.b.switch_to(merge);
        result
    }

    /// `sakafu(a / b)` right after its division, on exact non-negative integers with a positive
    /// divisor: an integer division gives the same result.
    fn floor_div(&self, pc: usize, src: Reg) -> Option<(Reg, Reg)> {
        let prev = self.function.code.get(pc.checked_sub(1)?)?;
        match prev {
            Opcode::Div { dst, a, b }
                if *dst == src
                    && self.int(*a)
                    && self.int(*b)
                    && self.facts[*a as usize].lo >= 0.0
                    && self.facts[*b as usize].lo > 0.0 =>
            {
                Some((*a, *b))
            }
            _ => None,
        }
    }

    /// Where to compute an integer result for `r`: straight into its register when it is an
    /// integer register, else a temporary that `finish_i` converts.
    fn int_dst(&mut self, r: Reg) -> VReg {
        if self.int(r) {
            self.regs[r as usize]
        } else {
            self.vreg(Class::Int)
        }
    }

    fn finish_i(&mut self, r: Reg, t: VReg) {
        if t != self.regs[r as usize] {
            self.set_i(r, t);
        }
    }

    fn float_dst(&mut self, r: Reg) -> VReg {
        if self.int(r) {
            self.vreg(Class::Float)
        } else {
            self.regs[r as usize]
        }
    }

    fn finish_f(&mut self, r: Reg, t: VReg) {
        if t != self.regs[r as usize] {
            self.set_f(r, t);
        }
    }

    fn arith(&mut self, dst: Reg, a: Reg, b: Reg, iop: IntOp, fop: FloatOp) {
        if self.int(dst) && self.int(a) && self.int(b) {
            let x = self.get_i(a);
            let y = self.get_i(b);
            let t = self.int_dst(dst);
            self.push(Inst::Int {
                op: iop,
                dst: t,
                a: x,
                b: y,
            });
            // Operands are integers within ±2^53, so the sum/difference is bounded by theirs.
            let (fa, fb) = (self.facts[a as usize], self.facts[b as usize]);
            self.result_range = match iop {
                IntOp::Add => (fa.lo + fb.lo, fa.hi + fb.hi),
                IntOp::Sub => (fa.lo - fb.hi, fa.hi - fb.lo),
                _ => (f64::NEG_INFINITY, f64::INFINITY),
            };
            self.finish_i(dst, t);
            self.result_range = (f64::NEG_INFINITY, f64::INFINITY);
        } else {
            let x = self.get_f(a);
            let y = self.get_f(b);
            let t = self.float_dst(dst);
            self.push(Inst::Float {
                op: fop,
                dst: t,
                a: x,
                b: y,
            });
            self.finish_f(dst, t);
        }
    }

    fn ibin(&mut self, dst: Reg, a: Reg, b: Reg, op: IntOp) {
        let natural = self.facts[a as usize].lo >= 0.0 && self.facts[b as usize].lo > 0.0;
        let op = match op {
            IntOp::SDiv if natural => IntOp::UDiv,
            IntOp::SRem if natural => IntOp::URem,
            op => op,
        };
        let x = self.get_i(a);
        let y = self.get_i(b);
        let t = self.int_dst(dst);
        self.push(Inst::Int {
            op,
            dst: t,
            a: x,
            b: y,
        });
        self.finish_i(dst, t);
    }

    fn instruction(&mut self, pc: usize, op: &Opcode) {
        match op {
            Opcode::Mov { dst, src } => {
                if self.int(*dst) && self.int(*src) {
                    // The source already satisfies the destination's bound.
                    let v = self.get_i(*src);
                    let d = self.regs[*dst as usize];
                    if d != v {
                        self.push(Inst::Mov { dst: d, src: v });
                    }
                } else {
                    let v = self.get_f(*src);
                    self.set_f(*dst, v);
                }
            }
            Opcode::Add { dst, a, b } => self.arith(*dst, *a, *b, IntOp::Add, FloatOp::Add),
            Opcode::Sub { dst, a, b } => self.arith(*dst, *a, *b, IntOp::Sub, FloatOp::Sub),
            Opcode::Mul { dst, a, b } => self.arith(*dst, *a, *b, IntOp::Mul, FloatOp::Mul),
            Opcode::Div { dst, a, b } => {
                let x = self.get_f(*a);
                let y = self.get_f(*b);
                let t = self.vreg(Class::Float);
                self.push(Inst::Float {
                    op: FloatOp::Div,
                    dst: t,
                    a: x,
                    b: y,
                });
                self.set_f(*dst, t);
            }
            Opcode::Rem { dst, a, b } => {
                if self.int(*dst) && self.int(*a) && self.int(*b) {
                    self.ibin(*dst, *a, *b, IntOp::SRem);
                } else {
                    let x = self.get_f(*a);
                    let y = self.get_f(*b);
                    let r = self.remainder_f(x, y);
                    self.set_f(*dst, r);
                }
            }
            Opcode::Pow { dst, a, b } => {
                let x = self.get_f(*a);
                let y = self.get_f(*b);
                let t = self
                    .call(RtFn::Pow, vec![x, y], Some(Class::Float), false)
                    .expect("pow");
                self.set_f(*dst, t);
            }
            Opcode::BitAnd { dst, a, b } => self.ibin(*dst, *a, *b, IntOp::And),
            Opcode::BitOr { dst, a, b } => self.ibin(*dst, *a, *b, IntOp::Or),
            Opcode::BitXor { dst, a, b } => self.ibin(*dst, *a, *b, IntOp::Xor),
            Opcode::Shl { dst, a, b } | Opcode::Shr { dst, a, b } => {
                let x = self.get_i(*a);
                let fb = self.facts[*b as usize];
                // `shift_amount`: saturating i32; anything outside 0..=63 shifts by 0.
                let s = if self.int(*b) && fb.lo >= 0.0 && fb.hi <= 63.0 {
                    self.get_i(*b)
                } else {
                    let y = self.get_f(*b);
                    self.call(RtFn::ShiftAmount, vec![y], Some(Class::Int), false)
                        .expect("shift")
                };
                let iop = if matches!(op, Opcode::Shl { .. }) {
                    IntOp::Shl
                } else {
                    IntOp::Sar
                };
                let t = self.int_op(iop, x, s);
                self.set_i(*dst, t);
            }
            Opcode::Neg { dst, src } => {
                if self.int(*dst) && self.int(*src) {
                    let v = self.get_i(*src);
                    let t = self.vreg(Class::Int);
                    self.push(Inst::Neg { dst: t, src: v });
                    self.set_i(*dst, t);
                } else {
                    let v = self.get_f(*src);
                    let bits = self.vreg(Class::Int);
                    self.push(Inst::FloatBits { dst: bits, src: v });
                    let sign = self.b.iconst(SIGN);
                    let flipped = self.int_op(IntOp::Xor, bits, sign);
                    let t = self.vreg(Class::Float);
                    self.push(Inst::BitsFloat {
                        dst: t,
                        src: flipped,
                    });
                    self.set_f(*dst, t);
                }
            }
            Opcode::BitNot { dst, src } => {
                let i = self.get_i(*src);
                let t = self.vreg(Class::Int);
                self.push(Inst::Not { dst: t, src: i });
                self.set_i(*dst, t);
            }
            Opcode::Not { dst, src } => {
                let c = self.truthy(*src);
                let one = self.b.iconst(1);
                let n = self.int_op(IntOp::Xor, c, one);
                self.set_flag(*dst, n);
            }
            Opcode::Floor { dst, src } | Opcode::Ceil { dst, src } => {
                let floor = matches!(op, Opcode::Floor { .. });
                if let (true, Some((a, b))) = (floor && self.int(*dst), self.floor_div(pc, *src)) {
                    self.ibin(*dst, a, b, IntOp::SDiv);
                } else if self.int(*src) {
                    let v = self.get_i(*src);
                    self.set_i(*dst, v);
                } else {
                    let v = self.get_f(*src);
                    let target = if floor { RtFn::Floor } else { RtFn::Ceil };
                    let t = self
                        .call(target, vec![v], Some(Class::Float), false)
                        .expect("round");
                    self.set_f(*dst, t);
                }
            }
            Opcode::Trunc { dst, src } => {
                let i = self.get_i(*src);
                self.set_i(*dst, i);
            }
            Opcode::Cmp { op: cmp, dst, a, b } => {
                let c = self.compare(*cmp, *a, *b);
                self.set_flag(*dst, c);
            }
            Opcode::Jump { target } => {
                let t = self.label(*target as usize);
                self.b.terminate(Term::Jump(t));
            }
            Opcode::JumpIfFalse { cond, target } | Opcode::JumpIfTrue { cond, target } => {
                let c = self.truthy(*cond);
                let target = self.label(*target as usize);
                let next = self.label(pc + 1);
                let (then_, else_) = if matches!(op, Opcode::JumpIfTrue { .. }) {
                    (target, next)
                } else {
                    (next, target)
                };
                self.b.terminate(Term::Branch {
                    cond: c,
                    then_,
                    else_,
                });
            }
            Opcode::JumpIfNot {
                op: cmp,
                a,
                b,
                target,
            } => {
                let c = self.compare(*cmp, *a, *b);
                let then_ = self.label(pc + 1);
                let else_ = self.label(*target as usize);
                self.b.terminate(Term::Branch {
                    cond: c,
                    then_,
                    else_,
                });
            }
            Opcode::ForStep { ctr, end, target } => {
                if self.int(*ctr) {
                    let x = self.get_i(*ctr);
                    let one = self.b.iconst(1);
                    let n = self.int_op(IntOp::Add, x, one);
                    self.set_i(*ctr, n);
                } else {
                    let x = self.get_f(*ctr);
                    let one = self.b.fconst(1.0);
                    let n = self.vreg(Class::Float);
                    self.push(Inst::Float {
                        op: FloatOp::Add,
                        dst: n,
                        a: x,
                        b: one,
                    });
                    self.set_f(*ctr, n);
                }
                let c = self.compare(CmpOp::Lt, *ctr, *end);
                let then_ = self.label(*target as usize);
                let else_ = self.label(pc + 1);
                self.b.terminate(Term::Branch {
                    cond: c,
                    then_,
                    else_,
                });
            }
            Opcode::ListGet { dst, list, idx, .. } => {
                let i = self.element(*list, *idx, pc, op);
                let (base, _) = self.lists[*list as usize];
                let kind = self.list_kinds[*list as usize];
                if kind != Kind::F64 {
                    // Integers (exact within ±2^53), read without conversion.
                    let v = if self.int(*dst) {
                        self.int_dst(*dst)
                    } else {
                        self.vreg(Class::Int)
                    };
                    self.push(Inst::LoadIndex {
                        dst: v,
                        base,
                        index: i,
                        kind,
                    });
                    self.finish_i(*dst, v);
                } else {
                    let v = self.float_dst(*dst);
                    self.push(Inst::LoadIndex {
                        dst: v,
                        base,
                        index: i,
                        kind,
                    });
                    self.finish_f(*dst, v);
                }
            }
            Opcode::ListSet { list, idx, src } => {
                let i = self.element(*list, *idx, pc, op);
                let kind = self.list_kinds[*list as usize];
                let v = self.list_value(kind, *src);
                let (base, _) = self.lists[*list as usize];
                self.push(Inst::StoreIndex {
                    src: v,
                    base,
                    index: i,
                    kind,
                });
            }
            Opcode::ListPush { list, src } => {
                let (p, l) = self.lists[*list as usize];
                let (head, room) = self.list_room[*list as usize].expect("pushed list");
                let kind = self.list_kinds[*list as usize];
                let one = self.b.iconst(1);
                let n = self.int_op(IntOp::Add, l, one);
                let width = self.b.iconst(kind.width() as i64);
                let need = self.int_op(IntOp::Mul, n, width);
                let fits = self.icmp(ICond::Ule, need, room);
                let fast = self.b.block();
                let grow = self.b.cold_block();
                let done = self.b.block();
                self.b.terminate(Term::Branch {
                    cond: fits,
                    then_: fast,
                    else_: grow,
                });
                // Room left: store after the last element and record the new length in the
                // list itself (the value fits `kind`, as for `ListSet`).
                self.b.switch_to(fast);
                let v = self.list_value(kind, *src);
                self.push(Inst::StoreIndex {
                    src: v,
                    base: p,
                    index: l,
                    kind,
                });
                self.push(Inst::Mov { dst: l, src: n });
                self.push(Inst::Store {
                    src: l,
                    base: head,
                    offset: 0,
                });
                self.b.terminate(Term::Jump(done));
                // Full: the runtime grows the storage and appends.
                self.b.switch_to(grow);
                let v = self.get_f(*src);
                let reg = self.b.iconst(*list as i64);
                self.call_into(RtFn::ListPush, vec![FRAME, reg, v], Some(p), false);
                self.push(Inst::Mov { dst: l, src: n });
                self.call_into(RtFn::ListHead, vec![FRAME, reg], Some(head), false);
                self.call_into(RtFn::ListRoom, vec![FRAME, reg], Some(room), false);
                self.b.terminate(Term::Jump(done));
                self.b.switch_to(done);
            }
            Opcode::ListRemove { list, idx } => {
                // Out-of-range removal is a no-op, as in the language.
                let i0 = self.get_i(*idx);
                let z = self.b.iconst(0);
                let neg = self.icmp(ICond::Lt, i0, z);
                let i = self.vreg(Class::Int);
                self.push(Inst::Select {
                    dst: i,
                    cond: neg,
                    a: z,
                    b: i0,
                });
                let (_, len) = self.lists[*list as usize];
                let ok = self.icmp(ICond::Ult, i, len);
                let yes = self.b.block();
                let done = self.b.block();
                self.b.terminate(Term::Branch {
                    cond: ok,
                    then_: yes,
                    else_: done,
                });
                self.b.switch_to(yes);
                let reg = self.b.iconst(*list as i64);
                self.call_into(RtFn::ListRemove, vec![FRAME, reg, i], Some(len), false);
                self.b.terminate(Term::Jump(done));
                self.b.switch_to(done);
            }
            // Value-register instructions that cannot fail: direct runtime calls.
            Opcode::BoxNum { dst, src } | Opcode::BoxBool { dst, src } => {
                let target = if matches!(op, Opcode::BoxNum { .. }) {
                    RtFn::BoxNum
                } else {
                    RtFn::BoxBool
                };
                let v = self.get_f(*src);
                let d = self.b.iconst(*dst as i64);
                self.call(target, vec![FRAME, d, v], None, false);
            }
            Opcode::ValMov { dst, src } => {
                let d = self.b.iconst(*dst as i64);
                let s = self.b.iconst(*src as i64);
                self.call(RtFn::ValMov, vec![FRAME, d, s], None, false);
            }
            Opcode::ConstVal { dst, k } => {
                let d = self.b.iconst(*dst as i64);
                let k = self.b.iconst(*k as i64);
                self.call(RtFn::ConstVal, vec![HOST, FRAME, d, k], None, false);
            }
            Opcode::CheckDepth => {
                let depth = self.vreg(Class::Int);
                self.push(Inst::Load {
                    dst: depth,
                    base: HOST,
                    offset: crate::native::DEPTH_OFFSET,
                });
                let max = self.b.iconst(crate::host::MAX_CALL_DEPTH as i64);
                let ok = self.icmp(ICond::Lt, depth, max);
                let cont = self.b.block();
                let fail = self.b.cold_block();
                self.b.terminate(Term::Branch {
                    cond: ok,
                    then_: cont,
                    else_: fail,
                });
                self.b.switch_to(fail);
                let status = self
                    .call(RtFn::DepthError, vec![HOST], Some(Class::Int), false)
                    .expect("status");
                self.b.terminate(Term::Return(status));
                self.b.switch_to(cont);
            }
            Opcode::ListLen { dst, list } => {
                let (_, len) = self.lists[*list as usize];
                self.set_i(*dst, len);
            }
            Opcode::Return { src } if self.ctx.entry_direct => {
                if matches!(src.ty, Ty::Num | Ty::Bool) {
                    let v = self.get_f(src.reg);
                    self.b.terminate(Term::ReturnNum(v));
                } else {
                    self.failed = true;
                    self.ret_status(STATUS_FAIL, pc);
                }
            }
            Opcode::Return { src } => {
                if matches!(src.ty, Ty::Num | Ty::Bool) {
                    self.spill(src.reg);
                }
                self.ret_status(STATUS_RETURN, pc);
            }
            Opcode::Call(call) if self.direct_callee(call).is_some() => {
                self.direct_call(pc, op, call)
            }
            // A direct entry returns only a number: one that can return nothing has none (the
            // `ReturnTupu` every function ends with is usually unreachable).
            Opcode::ReturnTupu if self.ctx.entry_direct => {
                if self.reachable[pc] {
                    self.failed = true;
                }
                let v = self.b.iconst(((STATUS_FAIL << 32) | pc as u64) as i64);
                self.b.terminate(Term::Return(v));
            }
            Opcode::ReturnTupu => self.ret_status(STATUS_RETURN, pc),
            other => self.slow(pc, other),
        }
    }
}
