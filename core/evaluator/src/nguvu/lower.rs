//! Bytecode → IR. The same translation decisions as the LLVM emitter (`aot.rs`), driven by the
//! same analysis (`native::analyze_numbers`): registers proven to hold exact integers become
//! `Int` virtual registers, speculated ones check their ±2^53 bound on every write and
//! deoptimize, proven in-range list accesses skip their bounds check, and every instruction the
//! backend doesn't lower runs in the interpreter through `Runtime::exec`.

use super::ir::{
    Block, Builder, Class, FCond, FloatOp, ICond, Inst, IntOp, RtFn, Term, VReg, FRAME, NUMS, VM,
};
use crate::bytecode::{BytecodeFunc, BytecodeProgram, CallOp, CmpOp, Opcode, Reg, Ty};
use crate::native::{
    analyze_numbers, jump_target, leaders, list_writes, num_reads, num_writes, NumFact,
    STATUS_DEOPT, STATUS_FAIL, STATUS_RETURN,
};
use crate::numlist::Kind;
use std::collections::{HashMap, HashSet};

/// What the function being lowered may assume about the rest of the program.
pub struct Ctx<'a> {
    pub program: &'a BytecodeProgram,
    /// Functions with a direct native entry: calls to them skip the interpreter.
    pub direct: &'a HashSet<usize>,
    /// Lower the direct entry — arguments `(rt, vm, stack limit, registers)`, the result stored
    /// after the registers — instead of the ordinary one.
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

/// The registers whose values a direct entry's caller buffer carries: parameters and every
/// register the function writes (the others keep their entry values).
pub(crate) fn frame_registers(f: &BytecodeFunc) -> Vec<Reg> {
    let mut regs: std::collections::BTreeSet<Reg> = f.code.iter().flat_map(writes).collect();
    regs.extend(
        f.params
            .iter()
            .filter(|p| matches!(p.ty, Ty::Num | Ty::Bool))
            .map(|p| p.reg),
    );
    regs.into_iter().collect()
}

/// A direct entry's third argument: the stack limit (the ordinary entry's is the frame).
const LIMIT_ARG: VReg = VReg(2);

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
    leaders: std::collections::BTreeSet<usize>,
    cur_pc: usize,
    /// Bounds known for the integer result being stored (set by `arith` for the next
    /// `set_i`): a side that cannot pass ±2^53 needs no check.
    result_range: (f64, f64),
    /// How each list register's elements are stored (`NumAnalysis::list_kinds`).
    list_kinds: Vec<Kind>,
    /// While reloading after an instruction the interpreter ran: where a failing guard resumes
    /// (the next instruction) and the registers already current in the frame.
    resume: Option<(usize, Vec<Reg>)>,
    ctx: &'a Ctx<'a>,
    /// A direct entry reached something only the interpreter can do: no direct entry.
    failed: bool,
    /// Stack limit for direct calls (an argument of direct entries, fetched on entry otherwise).
    limit: Option<VReg>,
    /// Largest register buffer a direct call site needs, in bytes.
    call_buffer: u32,
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
/// if some instruction needs the interpreter).
pub fn lower(index: usize, function: &BytecodeFunc, ctx: &Ctx) -> Option<super::ir::Func> {
    if ctx.entry_direct && !direct_signature(function) {
        return None;
    }
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
        leaders: leaders.clone(),
        cur_pc: 0,
        result_range: (f64::NEG_INFINITY, f64::INFINITY),
        list_kinds: analysis.list_kinds,
        resume: None,
        deopt: None,
        live_in: liveness(code),
        written,
        consts,
        ctx,
        failed: false,
        limit: None,
        call_buffer: 0,
    };
    let entry = l.b.block();
    for &pc in &leaders {
        let block = l.b.block();
        l.labels.insert(pc, block);
    }
    l.b.switch_to(entry);
    l.prologue();
    if ctx.entry_direct {
        l.limit = Some(LIMIT_ARG);
    } else if code
        .iter()
        .any(|op| matches!(op, Opcode::Call(c) if l.direct_callee(c).is_some()))
    {
        l.limit = l.call(RtFn::StackLimit, vec![], Some(Class::Int), false);
    }
    let first = l.label(0);
    l.b.terminate(Term::Jump(first));
    l.lower_range(0, code.len());
    if l.b.is_open() {
        let pc = code.len().saturating_sub(1);
        if ctx.entry_direct {
            l.cur_pc = pc;
            l.deopt_here();
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
    l.deopt_block();
    if l.failed {
        return None;
    }
    let call_buffer = l.call_buffer;
    let mut func = l.b.finish();
    func.call_buffer = call_buffer;
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
    ///
    /// A copy that deoptimizes spills the counter as that iteration's constant, so the
    /// interpreter resumes with exactly its frame state.
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
    /// and native code calls native code. The interpreter's call path still runs it when the
    /// call depth reaches the VM's limit or the stack is low (it can grow the stack), and it
    /// finishes the call when the callee deoptimizes.
    fn direct_call(&mut self, pc: usize, op: &Opcode, call: &CallOp) {
        let callee = self.direct_callee(call).expect("direct callee");
        let limit = self.limit.expect("stack limit fetched on entry");
        let result_offset = 8 * callee.num_regs as i32;
        self.call_buffer = self.call_buffer.max(8 * (callee.num_regs + 1));
        let depth = self.vreg(Class::Int);
        self.push(Inst::Load {
            dst: depth,
            base: VM,
            offset: crate::native::VM_DEPTH_OFFSET,
        });
        let max = self.b.iconst(crate::bytecode::MAX_CALL_DEPTH as i64);
        let done = self.b.block();
        // Two compare-and-branch pairs (each fuses), not one branch on their conjunction.
        let interp = (!self.ctx.entry_direct).then(|| self.b.cold_block());
        let check = |l: &mut Self, ok: VReg| match interp {
            // No interpreter frame here: deoptimize, and the caller finishes this function in
            // the interpreter — which makes the call through its own path.
            None => l.guard(ok),
            Some(interp) => {
                let next = l.b.block();
                l.b.terminate(Term::Branch {
                    cond: ok,
                    then_: next,
                    else_: interp,
                });
                l.b.switch_to(next);
            }
        };
        let shallow = self.icmp(ICond::Lt, depth, max);
        check(self, shallow);
        let sp = self.vreg(Class::Int);
        self.push(Inst::StackPointer { dst: sp });
        let roomy = self.icmp(ICond::Ult, limit, sp);
        check(self, roomy);
        if let Some(interp) = interp {
            let fast = self.b.block();
            self.b.terminate(Term::Jump(fast));
            self.b.switch_to(interp);
            self.slow(pc, op);
            self.b.terminate(Term::Jump(done));
            self.b.switch_to(fast);
        }
        let one = self.b.iconst(1);
        let deeper = self.int_op(IntOp::Add, depth, one);
        let set_depth = |l: &mut Self, v: VReg| {
            l.push(Inst::Store {
                src: v,
                base: VM,
                offset: crate::native::VM_DEPTH_OFFSET,
            })
        };
        set_depth(self, deeper);
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
        let status = self.vreg(Class::Int);
        self.push(Inst::CallDirect {
            func: call.function,
            args: vec![super::ir::RT, VM, limit, buf],
            dst: status,
        });
        set_depth(self, depth);
        let kind = self.vreg(Class::Int);
        self.push(Inst::IntImm {
            op: IntOp::Sar,
            dst: kind,
            a: status,
            imm: 32,
        });
        let ret = self.b.iconst(crate::native::STATUS_RETURN as i64);
        let returned = self.icmp(ICond::Eq, kind, ret);
        let got = self.b.block();
        let other = self.b.cold_block();
        self.b.terminate(Term::Branch {
            cond: returned,
            then_: got,
            else_: other,
        });

        // Not a plain return: a deoptimized callee finishes in the interpreter (counted at
        // its depth, as the VM would); anything else is a failure to pass on.
        self.b.switch_to(other);
        let deopt = self.b.iconst(STATUS_DEOPT as i64);
        let is_deopt = self.icmp(ICond::Eq, kind, deopt);
        let resume = self.b.cold_block();
        let fail = self.b.cold_block();
        self.b.terminate(Term::Branch {
            cond: is_deopt,
            then_: resume,
            else_: fail,
        });
        self.b.switch_to(resume);
        set_depth(self, deeper);
        let low = self.vreg(Class::Int);
        self.push(Inst::IntImm {
            op: IntOp::And,
            dst: low,
            a: status,
            imm: i32::MAX,
        });
        let f = self.b.iconst(call.function as i64);
        let resumed = self
            .call(RtFn::Resume, vec![VM, f, buf, low], Some(Class::Int), false)
            .expect("status");
        set_depth(self, depth);
        let kind2 = self.vreg(Class::Int);
        self.push(Inst::IntImm {
            op: IntOp::Sar,
            dst: kind2,
            a: resumed,
            imm: 32,
        });
        let ret2 = self.b.iconst(crate::native::STATUS_RETURN as i64);
        let ok2 = self.icmp(ICond::Eq, kind2, ret2);
        self.b.terminate(Term::Branch {
            cond: ok2,
            then_: got,
            else_: fail,
        });
        self.b.switch_to(fail);
        let status = self.b.iconst(((STATUS_FAIL << 32) | pc as u64) as i64);
        self.b.terminate(Term::Return(status));

        self.b.switch_to(got);
        let v = self.vreg(Class::Float);
        self.push(Inst::Load {
            dst: v,
            base: buf,
            offset: result_offset,
        });
        // The call has happened: a bound check on the result deoptimizes to the next
        // instruction, with the result already in the frame.
        let dst = call.dst.reg;
        if self.int(dst) && self.guarded[dst as usize] {
            self.push(Inst::Store {
                src: v,
                base: NUMS,
                offset: 8 * dst as i32,
            });
            self.resume = Some((pc + 1, vec![dst]));
        }
        self.set_f(dst, v);
        self.resume = None;
        self.b.terminate(Term::Jump(done));
        self.b.switch_to(done);
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
        // The interpreter resumes at this instruction (or after one it just ran) and only reads
        // registers live there; registers native code never writes, and those the interpreter
        // just wrote, already hold their value in the frame.
        let (pc, fresh) = match &self.resume {
            Some((pc, fresh)) => (*pc, fresh.clone()),
            None => (self.cur_pc, Vec::new()),
        };
        let need: Vec<Reg> = self.live_in[pc]
            .iter()
            .copied()
            .filter(|r| self.written.contains(r) && !fresh.contains(r))
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

    /// Leave a direct entry for the interpreter at the current instruction (something only it
    /// does exactly, such as a `kazi` declared to return a number returning nothing).
    fn deopt_here(&mut self) {
        let never = self.b.iconst(0);
        self.guard(never);
        let unreachable = self
            .b
            .iconst(((STATUS_FAIL << 32) | self.cur_pc as u64) as i64);
        self.b.terminate(Term::Return(unreachable));
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
        let kind = self.list_kinds[r as usize];
        let want = self.b.iconst(kind.code() as i64);
        self.push(Inst::Call {
            target: RtFn::ListPtr,
            args: vec![FRAME, reg, want],
            dst: Some(p),
            ret32: false,
        });
        if kind != Kind::F64 {
            // Null: some element does not fit after all — continue in the interpreter.
            let z = self.b.iconst(0);
            let ok = self.icmp(ICond::Ne, p, z);
            self.guard(ok);
        }
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
        if self.ctx.entry_direct && status == STATUS_RETURN {
            self.failed = true; // a direct entry returns only through `Return` with a value
        }
        let v = self.b.iconst(((status << 32) | pc as u64) as i64);
        self.b.terminate(Term::Return(v));
    }

    /// Run instruction `pc` in the interpreter; leave the function if it ended the call.
    fn slow(&mut self, pc: usize, op: &Opcode) {
        if self.ctx.entry_direct {
            self.failed = true; // direct entries have no interpreter frame to run it in
        }
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
        // The instruction has run: a guard failing while reading its results back resumes
        // after it, without overwriting what it wrote.
        self.resume = Some((pc + 1, num_writes(op)));
        for r in num_writes(op) {
            self.reload(r);
        }
        for r in list_writes(op) {
            self.refresh_list(r);
        }
        self.resume = None;
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
                let v = if kind == Kind::F64 {
                    self.get_f(*src)
                } else if self.int(*src) {
                    self.get_i(*src)
                } else {
                    // The analysis proved the value an exact integer: truncation is exact.
                    let f = self.get_f(*src);
                    let t = self.vreg(Class::Int);
                    self.push(Inst::FloatToInt { dst: t, src: f });
                    t
                };
                let (base, _) = self.lists[*list as usize];
                self.push(Inst::StoreIndex {
                    src: v,
                    base,
                    index: i,
                    kind,
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
            Opcode::Return { src } if self.ctx.entry_direct => {
                if matches!(src.ty, Ty::Num | Ty::Bool) {
                    // The caller reads the result after the callee's registers.
                    let v = self.get_f(src.reg);
                    self.push(Inst::Store {
                        src: v,
                        base: NUMS,
                        offset: 8 * self.function.num_regs as i32,
                    });
                    let status = self.b.iconst(((STATUS_RETURN << 32) | pc as u64) as i64);
                    self.b.terminate(Term::Return(status));
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
            Opcode::ReturnTupu if self.ctx.entry_direct => self.deopt_here(),
            Opcode::ReturnTupu => self.ret_status(STATUS_RETURN, pc),
            other => self.slow(pc, other),
        }
    }
}
