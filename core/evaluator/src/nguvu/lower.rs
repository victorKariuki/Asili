//! Bytecode → IR. The same translation decisions as the LLVM emitter (`aot.rs`), driven by the
//! same analysis (`native::analyze_numbers`): registers proven to hold exact integers become
//! `Int` virtual registers, speculated ones check their ±2^53 bound on every write and
//! deoptimize, proven in-range list accesses skip their bounds check, and every instruction the
//! backend doesn't lower runs in the interpreter through `Runtime::exec`.

use super::ir::{
    Block, Builder, Class, FCond, FloatOp, ICond, Inst, IntOp, RtFn, Term, VReg, FRAME, NUMS, VM,
};
use crate::bytecode::{BytecodeFunc, CmpOp, Opcode, Reg, Ty};
use crate::native::{
    analyze_numbers, leaders, list_writes, num_reads, num_writes, NumFact, STATUS_DEOPT,
    STATUS_FAIL, STATUS_RETURN,
};
use std::collections::HashMap;

const SIGN: i64 = i64::MIN;
const ABS_MASK: i64 = i64::MAX;
const EXACT: f64 = 9_007_199_254_740_992.0; // 2^53

struct Lower<'a> {
    b: Builder,
    function: &'a BytecodeFunc,
    index: usize,
    facts: Vec<NumFact>,
    ints: Vec<bool>,
    guarded: Vec<bool>,
    safe_index: std::collections::HashSet<usize>,
    /// Bytecode numeric register → its virtual register.
    regs: Vec<VReg>,
    /// List register → (data pointer, length) virtual registers.
    lists: Vec<(VReg, VReg)>,
    labels: HashMap<usize, Block>,
    cur_pc: usize,
    /// Bounds known for the integer result being stored (set by `arith` for the next
    /// `set_i`): a side that cannot pass ±2^53 needs no check.
    result_range: (f64, f64),
    /// Shared deoptimization exit and the register carrying the resume pc into it.
    deopt: Option<(Block, VReg)>,
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
            Opcode::JumpIfFalse { target, .. }
            | Opcode::JumpIfTrue { target, .. }
            | Opcode::JumpIfNot { target, .. }
            | Opcode::ForStep { target, .. } => vec![*target as usize, pc + 1],
            Opcode::Return { .. } | Opcode::ReturnTupu => vec![],
            _ => vec![pc + 1],
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

/// Lower one function, or `None` if its jump targets are malformed.
pub fn lower(index: usize, function: &BytecodeFunc) -> Option<super::ir::Func> {
    let code = &function.code;
    let leaders = leaders(code)?;
    let analysis = analyze_numbers(function);
    let facts = analysis.regs;
    let ints: Vec<bool> = facts
        .iter()
        .map(|f| f.exact_int() || f.int_like())
        .collect();
    let guarded = facts
        .iter()
        .map(|f| f.int_like() && !f.exact_int())
        .collect();
    let mut b = Builder::new();
    let regs = ints
        .iter()
        .map(|&i| b.vreg(if i { Class::Int } else { Class::Float }))
        .collect();
    let lists = (0..function.list_regs)
        .map(|_| (b.vreg(Class::Int), b.vreg(Class::Int)))
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
        guarded,
        safe_index: analysis.safe_index,
        regs,
        lists,
        labels: HashMap::new(),
        cur_pc: 0,
        result_range: (f64::NEG_INFINITY, f64::INFINITY),
        deopt: None,
        live_in: liveness(code),
        written,
        consts,
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
    for (pc, op) in code.iter().enumerate() {
        if leaders.contains(&pc) {
            let block = l.label(pc);
            if l.b.is_open() {
                l.b.terminate(Term::Jump(block));
            }
            l.b.switch_to(block);
        } else if !l.b.is_open() {
            continue; // unreachable
        }
        l.cur_pc = pc;
        l.instruction(pc, op);
    }
    if l.b.is_open() {
        let pc = code.len().saturating_sub(1);
        l.ret_status(STATUS_RETURN, pc);
    }
    // Leaders past the end of the code (after a final return) are never jumped to.
    let tail: Vec<(usize, Block)> = l
        .labels
        .iter()
        .filter(|(&pc, _)| pc >= code.len())
        .map(|(&pc, &block)| (pc, block))
        .collect();
    for (pc, block) in tail {
        l.b.switch_to(block);
        l.ret_status(STATUS_RETURN, pc.saturating_sub(1));
    }
    l.deopt_block();
    Some(l.b.finish())
}

impl<'a> Lower<'a> {
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
        let mut num_params = std::collections::HashSet::new();
        let mut list_params = std::collections::HashSet::new();
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
        self.push(Inst::Call {
            target,
            args,
            dst,
            ret32,
        });
        dst
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

    fn fabs(&mut self, v: VReg) -> VReg {
        let bits = self.vreg(Class::Int);
        self.push(Inst::FloatBits { dst: bits, src: v });
        let mask = self.b.iconst(ABS_MASK);
        let abs = self.int_op(IntOp::And, bits, mask);
        let f = self.vreg(Class::Float);
        self.push(Inst::BitsFloat { dst: f, src: abs });
        f
    }

    /// Continue only if `ok != 0`; otherwise deoptimize at the current instruction.
    fn guard(&mut self, ok: VReg) {
        let (deopt, dpc) = match self.deopt {
            Some(d) => d,
            None => {
                let block = self.b.cold_block();
                let dpc = self.vreg(Class::Int);
                self.deopt = Some((block, dpc));
                (block, dpc)
            }
        };
        let site = self.b.cold_block();
        let cont = self.b.block();
        self.b.terminate(Term::Branch {
            cond: ok,
            then_: cont,
            else_: site,
        });
        self.b.switch_to(site);
        // The interpreter resumes at this instruction and only reads registers live there;
        // registers native code never writes already hold their value in the frame.
        let pc = self.cur_pc;
        let need: Vec<Reg> = self.live_in[pc]
            .iter()
            .copied()
            .filter(|r| self.written.contains(r))
            .collect();
        for r in need {
            self.spill(r);
        }
        self.push(Inst::IConst {
            dst: dpc,
            value: pc as i64,
        });
        self.b.terminate(Term::Jump(deopt));
        self.b.switch_to(cont);
    }

    /// `|v| <= 2^53` for a speculated integer register, checking only the sides
    /// `result_range` does not already rule out.
    fn guard_i(&mut self, v: VReg) {
        let (lo, hi) =
            std::mem::replace(&mut self.result_range, (f64::NEG_INFINITY, f64::INFINITY));
        // Strict: the bounds are summed in f64, which rounds; monotonic rounding keeps a
        // result below 2^53 only if the exact sum is.
        let ok = match (lo > -EXACT, hi < EXACT) {
            (true, true) => return,
            (true, false) => {
                let limit = self.b.iconst(1 << 53);
                self.icmp(ICond::Le, v, limit)
            }
            (false, true) => {
                let limit = self.b.iconst(-(1 << 53));
                self.icmp(ICond::Ge, v, limit)
            }
            (false, false) => {
                let bias = self.b.iconst(1 << 53);
                let shifted = self.int_op(IntOp::Add, v, bias);
                let limit = self.b.iconst(1 << 54);
                self.icmp(ICond::Ule, shifted, limit)
            }
        };
        self.guard(ok);
    }

    /// Write every register back and resume in the interpreter at the failing instruction.
    fn deopt_block(&mut self) {
        let Some((block, dpc)) = self.deopt else {
            return;
        };
        self.b.switch_to(block);
        let tag = self.b.iconst((STATUS_DEOPT << 32) as i64);
        let ret = self.int_op(IntOp::Or, dpc, tag);
        self.b.terminate(Term::Return(ret));
    }

    fn set_f(&mut self, r: Reg, v: VReg) {
        let dst = self.regs[r as usize];
        if self.int(r) && self.guarded[r as usize] {
            let a = self.fabs(v);
            let limit = self.b.fconst(EXACT);
            let ok = self.fcmp(FCond::Ole, a, limit);
            self.guard(ok);
        }
        if self.int(r) {
            // Every value this register receives is an exact integer.
            self.push(Inst::FloatToInt { dst, src: v });
        } else if dst != v {
            self.push(Inst::Mov { dst, src: v });
        }
    }

    fn set_i(&mut self, r: Reg, v: VReg) {
        let dst = self.regs[r as usize];
        if self.int(r) && self.guarded[r as usize] {
            self.guard_i(v);
        }
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

    fn refresh_list(&mut self, r: Reg) {
        let (p, l) = self.lists[r as usize];
        let reg = self.b.iconst(r as i64);
        self.push(Inst::Call {
            target: RtFn::ListPtr,
            args: vec![FRAME, reg],
            dst: Some(p),
            ret32: false,
        });
        self.push(Inst::Call {
            target: RtFn::ListLen,
            args: vec![FRAME, reg],
            dst: Some(l),
            ret32: false,
        });
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
        let v = self.b.iconst(((status << 32) | pc as u64) as i64);
        self.b.terminate(Term::Return(v));
    }

    /// Run instruction `pc` in the interpreter; leave the function if it ended the call.
    fn slow(&mut self, pc: usize, op: &Opcode) {
        for r in num_reads(op) {
            self.spill(r);
        }
        let f = self.b.iconst(self.index as i64);
        let p = self.b.iconst(pc as i64);
        let s = self
            .call(RtFn::Exec, vec![VM, FRAME, f, p], Some(Class::Int), true)
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

    /// Bounds-checked element index into `list` (out of range: the interpreter raises the
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

    /// Where to compute an integer result for `r`: straight into its register when no bound
    /// check is needed, else a temporary that `finish_i` checks and copies.
    fn int_dst(&mut self, r: Reg) -> VReg {
        if self.int(r) && !self.guarded[r as usize] {
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
            Opcode::Mul { dst, a, b }
                if self.int(*dst)
                    && self.int(*a)
                    && self.int(*b)
                    && (self.guarded[*dst as usize]
                        || self.guarded[*a as usize]
                        || self.guarded[*b as usize]) =>
            {
                // Operands up to 2^53 can overflow i64: check, then bound the product.
                let x = self.get_i(*a);
                let y = self.get_i(*b);
                let prod = self.vreg(Class::Int);
                let ovf = self.vreg(Class::Int);
                self.push(Inst::MulOverflow {
                    dst: prod,
                    ovf,
                    a: x,
                    b: y,
                });
                let z = self.b.iconst(0);
                let ok = self.icmp(ICond::Eq, ovf, z);
                self.guard(ok);
                self.set_i(*dst, prod);
            }
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
                let v = self.float_dst(*dst);
                self.push(Inst::LoadIndex {
                    dst: v,
                    base,
                    index: i,
                });
                self.finish_f(*dst, v);
            }
            Opcode::ListSet { list, idx, src } => {
                let i = self.element(*list, *idx, pc, op);
                let v = self.get_f(*src);
                let (base, _) = self.lists[*list as usize];
                self.push(Inst::StoreIndex {
                    src: v,
                    base,
                    index: i,
                });
            }
            Opcode::ListPush { list, src } => {
                let v = self.get_f(*src);
                let (p, l) = self.lists[*list as usize];
                let reg = self.b.iconst(*list as i64);
                self.push(Inst::Call {
                    target: RtFn::ListPush,
                    args: vec![FRAME, reg, v],
                    dst: Some(p),
                    ret32: false,
                });
                let one = self.b.iconst(1);
                self.push(Inst::Int {
                    op: IntOp::Add,
                    dst: l,
                    a: l,
                    b: one,
                });
            }
            Opcode::ListRemove { list, idx } => {
                // Out-of-range removal is a no-op, like the interpreter.
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
                self.push(Inst::Call {
                    target: RtFn::ListRemove,
                    args: vec![FRAME, reg, i],
                    dst: Some(len),
                    ret32: false,
                });
                self.b.terminate(Term::Jump(done));
                self.b.switch_to(done);
            }
            Opcode::ListLen { dst, list } => {
                let (_, len) = self.lists[*list as usize];
                self.set_i(*dst, len);
            }
            Opcode::Return { src } => {
                if matches!(src.ty, Ty::Num | Ty::Bool) {
                    self.spill(src.reg);
                }
                self.ret_status(STATUS_RETURN, pc);
            }
            Opcode::ReturnTupu => self.ret_status(STATUS_RETURN, pc),
            other => self.slow(pc, other),
        }
    }
}
