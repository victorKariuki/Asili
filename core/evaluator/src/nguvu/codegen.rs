//! IR → x86-64 (System V calling convention).
//!
//! Frame layout: `rbp` frame pointer, callee-saved `rbx`, `r12`–`r15` pushed, then one 8-byte
//! home slot per virtual register. Virtual registers live where [`regalloc`] put them; operands
//! are used in place when they sit in a register, and `rax`/`rcx`/`rdx`, `xmm0`/`xmm1` are
//! scratch. A comparison feeding the block's branch is fused into `cmp` + `jcc`.

use super::ir::{Class, FCond, FloatOp, Func, ICond, Inst, IntOp, Term, VReg};
use super::regalloc::{allocate, is_caller_saved, Allocation, Loc};
use super::x64::{Alu, Asm, Cond, Gpr, Label, Mem, MemIdx, Sse, Xmm};

const INT_ARGS: [Gpr; 6] = [Gpr::Rdi, Gpr::Rsi, Gpr::Rdx, Gpr::Rcx, Gpr::R8, Gpr::R9];
/// Bytes pushed below `rbp`: rbx, r12, r13, r14, r15.
const SAVED: i32 = 40;
const X0: Xmm = Xmm(0);
const X1: Xmm = Xmm(1);

struct Gen<'f> {
    asm: Asm,
    func: &'f Func,
    alloc: Allocation,
}

fn slot(v: VReg) -> Mem {
    Mem {
        base: Gpr::Rbp,
        disp: -SAVED - 8 * (v.0 as i32 + 1),
    }
}

fn icond(c: ICond) -> Cond {
    match c {
        ICond::Eq => Cond::E,
        ICond::Ne => Cond::Ne,
        ICond::Lt => Cond::L,
        ICond::Le => Cond::Le,
        ICond::Gt => Cond::G,
        ICond::Ge => Cond::Ge,
        ICond::Ult => Cond::B,
        ICond::Ule => Cond::Be,
    }
}

fn negate(c: Cond) -> Cond {
    match c {
        Cond::O => Cond::O, // never negated
        Cond::B => Cond::Ae,
        Cond::Ae => Cond::B,
        Cond::E => Cond::Ne,
        Cond::Ne => Cond::E,
        Cond::Be => Cond::A,
        Cond::A => Cond::Be,
        Cond::P => Cond::Np,
        Cond::Np => Cond::P,
        Cond::L => Cond::Ge,
        Cond::Ge => Cond::L,
        Cond::Le => Cond::G,
        Cond::G => Cond::Le,
    }
}

/// Machine code for `func`, position independent (runtime calls go through the `rt` table).
pub fn generate(func: &Func) -> Vec<u8> {
    let alloc = allocate(func);
    if let Ok(path) = std::env::var("ASILI_NGUVU_IR") {
        use std::io::Write as _;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = f.write_all(super::regalloc::dump(func, &alloc).as_bytes());
        }
    }
    let mut g = Gen {
        asm: Asm::new(),
        func,
        alloc,
    };
    let slots = func.classes.len() as i32;
    // After `push rbp` the stack is 16-aligned; five more pushes leave it at 8 mod 16, so the
    // slot area must be 8 mod 16 for calls to see an aligned stack.
    let mut frame = 8 * slots;
    if frame % 16 != 8 {
        frame += 8;
    }
    g.asm.push(Gpr::Rbp);
    g.asm.mov_rr(Gpr::Rbp, Gpr::Rsp);
    for r in [Gpr::Rbx, Gpr::R12, Gpr::R13, Gpr::R14, Gpr::R15] {
        g.asm.push(r);
    }
    g.asm.alu_ri(Alu::Sub, Gpr::Rsp, frame);
    // Incoming arguments: park them in their home slots first (their allocated registers may be
    // other argument registers), then load each where it lives.
    for (i, r) in INT_ARGS.iter().take(4).enumerate() {
        g.asm.store(slot(VReg(i as u32)), *r);
    }
    for i in 0..4 {
        let v = VReg(i);
        if let Loc::Gpr(r) = g.loc(v) {
            g.asm.load(r, slot(v));
        }
    }
    let labels: Vec<_> = (0..func.blocks.len()).map(|_| g.asm.new_label()).collect();
    let order = g.alloc.order.clone();
    for (k, &bi) in order.iter().enumerate() {
        let block = &func.blocks[bi];
        g.asm.bind(labels[bi]);
        let next = order.get(k + 1).map(|&b| labels[b]);
        // A comparison whose only reader is this block's branch compiles to cmp + jcc.
        let fused = matches!(
            (&block.term, block.insts.last()),
            (
                Term::Branch { cond, .. },
                Some(
                    Inst::ICmp { dst, .. }
                    | Inst::ICmpImm { dst, .. }
                    | Inst::TestImm { dst, .. }
                    | Inst::FCmp { dst, .. },
                ),
            ) if dst == cond && g.alloc.uses[cond.0 as usize] == 1
        );
        let body = if fused {
            &block.insts[..block.insts.len() - 1]
        } else {
            &block.insts[..]
        };
        let mut ii = 0;
        while ii < body.len() {
            // A comparison read only by the selects right after it: cmp once, then cmovcc.
            // Conditional moves and plain copies (mov/movapd/loads/stores) leave the flags
            // intact, so copies may sit in between.
            let run = match body[ii] {
                Inst::ICmp { dst, .. } | Inst::ICmpImm { dst, .. } | Inst::TestImm { dst, .. } => {
                    let run = body[ii + 1..]
                        .iter()
                        .take_while(|i| match i {
                            Inst::Select { cond, dst: d, .. } => *cond == dst && *d != dst,
                            Inst::Mov { dst: d, src } => *d != dst && *src != dst,
                            _ => false,
                        })
                        .count();
                    let selects = body[ii + 1..ii + 1 + run]
                        .iter()
                        .filter(|i| matches!(i, Inst::Select { .. }))
                        .count();
                    (selects > 0 && g.alloc.uses[dst.0 as usize] as usize == selects).then_some(run)
                }
                _ => None,
            };
            match run {
                Some(n) => {
                    let cc = g.flags_for(&body[ii]);
                    for (k, inst) in body[ii + 1..ii + 1 + n].iter().enumerate() {
                        match inst {
                            Inst::Select { dst, a, b, .. } => g.select_on(cc, *dst, *a, *b),
                            other => g.inst(bi, ii + 1 + k, other),
                        }
                    }
                    ii += 1 + n;
                }
                None => {
                    g.inst(bi, ii, &body[ii]);
                    ii += 1;
                }
            }
        }
        match &block.term {
            Term::Jump(t) => {
                if Some(labels[t.0 as usize]) != next {
                    g.asm.jmp(labels[t.0 as usize]);
                }
            }
            Term::Branch { cond, then_, else_ } => {
                let (t, e) = (labels[then_.0 as usize], labels[else_.0 as usize]);
                if fused {
                    g.fused_branch(block.insts.last().expect("compare"), t, e, next);
                } else {
                    let r = g.int_in(*cond, Gpr::Rax);
                    g.asm.alu_rr(Alu::Test, r, r);
                    g.branch_on(Cond::Ne, t, e, next);
                }
            }
            Term::Return(v) => {
                let r = g.int_in(*v, Gpr::Rax);
                g.asm.mov_rr(Gpr::Rax, r);
                g.epilogue();
            }
        }
    }
    g.asm.finish()
}

impl<'f> Gen<'f> {
    fn loc(&self, v: VReg) -> Loc {
        self.alloc.loc[v.0 as usize]
    }

    fn epilogue(&mut self) {
        let a = &mut self.asm;
        a.lea(
            Gpr::Rsp,
            Mem {
                base: Gpr::Rbp,
                disp: -SAVED,
            },
        );
        for r in [Gpr::R15, Gpr::R14, Gpr::R13, Gpr::R12, Gpr::Rbx] {
            a.pop(r);
        }
        a.pop(Gpr::Rbp);
        a.ret();
    }

    /// Jump to `t` when `cc` holds, else to `e`, falling through where possible.
    fn branch_on(&mut self, cc: Cond, t: Label, e: Label, next: Option<Label>) {
        if Some(t) == next {
            self.asm.jcc(negate(cc), e);
        } else {
            self.asm.jcc(cc, t);
            if Some(e) != next {
                self.asm.jmp(e);
            }
        }
    }

    fn fused_branch(&mut self, cmp: &Inst, t: Label, e: Label, next: Option<Label>) {
        match cmp {
            Inst::ICmp { cond, a, b, .. } => {
                let ra = self.int_in(*a, Gpr::Rax);
                self.alu_with(Alu::Cmp, ra, *b);
                self.branch_on(icond(*cond), t, e, next);
            }
            Inst::ICmpImm { cond, a, imm, .. } => {
                let ra = self.int_in(*a, Gpr::Rax);
                self.asm.cmp_ri(ra, *imm);
                self.branch_on(icond(*cond), t, e, next);
            }
            Inst::TestImm { zero, a, imm, .. } => {
                let ra = self.int_in(*a, Gpr::Rax);
                self.asm.alu_ri(Alu::Test, ra, *imm);
                self.branch_on(if *zero { Cond::E } else { Cond::Ne }, t, e, next);
            }
            Inst::FCmp { cond, a, b, .. } => {
                let xa = self.float_in(*a, X0);
                let xb = self.float_in(*b, X1);
                match cond {
                    FCond::Olt => {
                        self.asm.ucomisd(xb, xa);
                        self.branch_on(Cond::A, t, e, next);
                    }
                    FCond::Ole => {
                        self.asm.ucomisd(xb, xa);
                        self.branch_on(Cond::Ae, t, e, next);
                    }
                    FCond::Ogt => {
                        self.asm.ucomisd(xa, xb);
                        self.branch_on(Cond::A, t, e, next);
                    }
                    FCond::Oge => {
                        self.asm.ucomisd(xa, xb);
                        self.branch_on(Cond::Ae, t, e, next);
                    }
                    FCond::Oeq => {
                        // Equal and ordered: unordered (PF) goes to `e`.
                        self.asm.ucomisd(xa, xb);
                        self.asm.jcc(Cond::P, e);
                        self.branch_on(Cond::E, t, e, next);
                    }
                    FCond::Une => {
                        self.asm.ucomisd(xa, xb);
                        self.asm.jcc(Cond::P, t);
                        self.branch_on(Cond::Ne, t, e, next);
                    }
                }
            }
            _ => unreachable!("fused branch on a non-compare"),
        }
    }

    /// The register holding integer-class `v`, loading it into `scratch` if it is spilled.
    /// Set the flags for an integer comparison and return the condition meaning "true".
    fn flags_for(&mut self, cmp: &Inst) -> Cond {
        match cmp {
            Inst::ICmp { cond, a, b, .. } => {
                let ra = self.int_in(*a, Gpr::Rax);
                self.alu_with(Alu::Cmp, ra, *b);
                icond(*cond)
            }
            Inst::ICmpImm { cond, a, imm, .. } => {
                let ra = self.int_in(*a, Gpr::Rax);
                self.asm.cmp_ri(ra, *imm);
                icond(*cond)
            }
            Inst::TestImm { zero, a, imm, .. } => {
                let ra = self.int_in(*a, Gpr::Rax);
                self.asm.alu_ri(Alu::Test, ra, *imm);
                if *zero {
                    Cond::E
                } else {
                    Cond::Ne
                }
            }
            _ => unreachable!("flags for a non-comparison"),
        }
    }

    /// `dst = cc ? a : b` with the flags already set (only moves in between).
    fn select_on(&mut self, cc: Cond, dst: VReg, a: VReg, b: VReg) {
        use Gpr::*;
        let ra = self.int_in(a, Rcx);
        match self.loc(dst) {
            Loc::Gpr(d) if self.loc(b) == Loc::Gpr(d) => self.asm.cmov(cc, d, ra),
            _ => {
                let rb = self.int_in(b, Rax);
                self.asm.mov_rr(Rax, rb);
                self.asm.cmov(cc, Rax, ra);
                self.put_int(dst, Rax);
            }
        }
    }

    /// `dst op= v`, reading `v` straight from its slot when spilled.
    fn alu_with(&mut self, op: Alu, dst: Gpr, v: VReg) {
        match self.loc(v) {
            Loc::Slot => self.asm.alu_rm(op, dst, slot(v)),
            _ => {
                let r = self.int_in(v, Gpr::Rcx);
                self.asm.alu_rr(op, dst, r);
            }
        }
    }

    fn int_in(&mut self, v: VReg, scratch: Gpr) -> Gpr {
        match self.loc(v) {
            Loc::Gpr(r) => r,
            Loc::Slot => {
                self.asm.load(scratch, slot(v));
                scratch
            }
            Loc::Xmm(_) => unreachable!("integer value in an xmm register"),
        }
    }

    fn float_in(&mut self, v: VReg, scratch: Xmm) -> Xmm {
        match self.loc(v) {
            Loc::Xmm(x) => x,
            Loc::Slot => {
                self.asm.movsd_load(scratch, slot(v));
                scratch
            }
            Loc::Gpr(_) => unreachable!("float value in a general register"),
        }
    }

    /// Write `src` (a general register holding `v`'s value or bits) to `v`'s location.
    fn put_int(&mut self, v: VReg, src: Gpr) {
        match self.loc(v) {
            Loc::Gpr(r) if r == src => {}
            Loc::Gpr(r) => self.asm.mov_rr(r, src),
            Loc::Xmm(x) => self.asm.movq_xr(x, src),
            Loc::Slot => self.asm.store(slot(v), src),
        }
    }

    fn put_float(&mut self, v: VReg, src: Xmm) {
        match self.loc(v) {
            Loc::Xmm(x) if x == src => {}
            Loc::Xmm(x) => self.asm.movapd(x, src),
            Loc::Gpr(r) => self.asm.movq_rx(r, src),
            Loc::Slot => self.asm.movsd_store(slot(v), src),
        }
    }

    /// The raw 64 bits of `v` (either class) in a general register.
    fn bits_in(&mut self, v: VReg, scratch: Gpr) -> Gpr {
        match self.loc(v) {
            Loc::Gpr(r) => r,
            Loc::Xmm(x) => {
                self.asm.movq_rx(scratch, x);
                scratch
            }
            Loc::Slot => {
                self.asm.load(scratch, slot(v));
                scratch
            }
        }
    }

    /// Integer destination register to compute into: `v`'s own register, or `scratch`.
    fn int_target(&self, v: VReg, scratch: Gpr) -> Gpr {
        match self.loc(v) {
            Loc::Gpr(r) => r,
            _ => scratch,
        }
    }

    fn float_target(&self, v: VReg, scratch: Xmm) -> Xmm {
        match self.loc(v) {
            Loc::Xmm(x) => x,
            _ => scratch,
        }
    }

    /// Store `v` to its home slot if it lives in a register.
    fn save(&mut self, v: VReg) {
        match self.loc(v) {
            Loc::Gpr(r) => self.asm.store(slot(v), r),
            Loc::Xmm(x) => self.asm.movsd_store(slot(v), x),
            Loc::Slot => {}
        }
    }

    fn restore(&mut self, v: VReg) {
        match self.loc(v) {
            Loc::Gpr(r) => self.asm.load(r, slot(v)),
            Loc::Xmm(x) => self.asm.movsd_load(x, slot(v)),
            Loc::Slot => {}
        }
    }

    /// `dst = a op imm`.
    fn int_imm(&mut self, op: IntOp, dst: VReg, a: VReg, imm: i32) {
        use Gpr::*;
        match op {
            IntOp::SDiv | IntOp::SRem | IntOp::UDiv | IntOp::URem => {
                return self.div_const(op, dst, a, imm)
            }
            _ => {}
        }
        let d = self.int_target(dst, Rax);
        let ra = self.int_in(a, d);
        match op {
            IntOp::Mul => self.asm.imul_rri(d, ra, imm),
            _ => {
                self.asm.mov_rr(d, ra);
                match op {
                    IntOp::Add => self.asm.alu_ri(Alu::Add, d, imm),
                    IntOp::Sub => self.asm.alu_ri(Alu::Sub, d, imm),
                    IntOp::And => self.asm.alu_ri(Alu::And, d, imm),
                    IntOp::Or => self.asm.alu_ri(Alu::Or, d, imm),
                    IntOp::Xor => self.asm.alu_ri(Alu::Xor, d, imm),
                    IntOp::Shl => self.asm.shift_ri(true, d, imm as u8),
                    IntOp::Sar => self.asm.shift_ri(false, d, imm as u8),
                    _ => unreachable!(),
                }
            }
        }
        self.put_int(dst, d);
    }

    /// Truncating `a / d` or `a % d` for a constant `d > 0`, as a multiplication by a
    /// fixed-point reciprocal: `q = mulhi(|a|, floor(2^64 / d) + 1)` is exact whenever
    /// `|a| * d < 2^64` — always true here, since integer registers hold `|a| <= 2^53` and the
    /// reciprocal is only used for `d < 2^11`. Larger divisors use `idiv`. Unsigned forms
    /// (non-negative dividend) skip the sign handling, and powers of two become shifts/masks.
    fn div_const(&mut self, op: IntOp, dst: VReg, a: VReg, d: i32) {
        use Gpr::*;
        let quotient = matches!(op, IntOp::SDiv | IntOp::UDiv);
        let unsigned = matches!(op, IntOp::UDiv | IntOp::URem);
        if unsigned && (d as u32).is_power_of_two() {
            let t = self.int_target(dst, Rax);
            let ra = self.int_in(a, t);
            self.asm.mov_rr(t, ra);
            if quotient {
                self.asm.shift_ri(false, t, d.trailing_zeros() as u8);
            } else {
                self.asm.alu_ri(Alu::And, t, d - 1);
            }
            self.put_int(dst, t);
            return;
        }
        let ra = self.int_in(a, Rax);
        self.asm.mov_rr(Rax, ra);
        if d == 1 {
            if !quotient {
                self.asm.mov_ri(Rax, 0);
            }
            self.put_int(dst, Rax);
            return;
        }
        if d >= 2048 {
            self.asm.mov_ri(Rcx, d as i64);
            if unsigned {
                self.asm.zero_div(Rcx);
            } else {
                self.asm.cqo_idiv(Rcx);
            }
            self.put_int(dst, if quotient { Rax } else { Rdx });
            return;
        }
        let magic = (u64::MAX / d as u64).wrapping_add(1); // floor(2^64/d) + 1 for d > 1
        if unsigned {
            self.asm.mov_ri(Rdx, magic as i64);
            self.asm.mul(Rdx); // rdx = a / d
        } else {
            // m = a >> 63 (all ones if negative); |a| = (a ^ m) - m
            self.asm.mov_rr(Rcx, Rax);
            self.asm.shift_ri(false, Rcx, 63);
            self.asm.alu_rr(Alu::Xor, Rax, Rcx);
            self.asm.alu_rr(Alu::Sub, Rax, Rcx);
            self.asm.mov_ri(Rdx, magic as i64);
            self.asm.mul(Rdx); // rdx = |a| / d
                               // Restore the sign: q = (q ^ m) - m
            self.asm.alu_rr(Alu::Xor, Rdx, Rcx);
            self.asm.alu_rr(Alu::Sub, Rdx, Rcx);
        }
        if quotient {
            self.put_int(dst, Rdx);
        } else {
            // r = a - q * d (same sign as a, like `srem`)
            self.asm.imul_rri(Rdx, Rdx, d);
            let ra = self.int_in(a, Rax);
            self.asm.mov_rr(Rax, ra);
            self.asm.alu_rr(Alu::Sub, Rax, Rdx);
            self.put_int(dst, Rax);
        }
    }

    fn inst(&mut self, bi: usize, ii: usize, inst: &Inst) {
        use Gpr::*;
        match inst {
            Inst::IConst { dst, value } => {
                let d = self.int_target(*dst, Rax);
                self.asm.mov_ri(d, *value);
                self.put_int(*dst, d);
            }
            Inst::FConst { dst, value } => {
                self.asm.mov_ri(Rax, value.to_bits() as i64);
                self.put_int(*dst, Rax);
            }
            Inst::Mov { dst, src } => match self.func.class(*dst) {
                Class::Int => {
                    let r = self.int_in(*src, Rax);
                    self.put_int(*dst, r);
                }
                Class::Float => {
                    let x = self.float_in(*src, X0);
                    self.put_float(*dst, x);
                }
            },
            Inst::FloatBits { dst, src } | Inst::BitsFloat { dst, src } => {
                let r = self.bits_in(*src, Rax);
                self.put_int(*dst, r);
            }
            Inst::Int { op, dst, a, b } => {
                match op {
                    IntOp::Shl | IntOp::Sar => {
                        let rb = self.int_in(*b, Rcx);
                        self.asm.mov_rr(Rcx, rb);
                        let d = self.int_target(*dst, Rax);
                        let ra = self.int_in(*a, d);
                        self.asm.mov_rr(d, ra);
                        self.asm.shift_cl(matches!(op, IntOp::Shl), d);
                        self.put_int(*dst, d);
                    }
                    IntOp::SDiv | IntOp::SRem | IntOp::UDiv | IntOp::URem => {
                        let rb = self.int_in(*b, Rcx);
                        self.asm.mov_rr(Rcx, rb);
                        let ra = self.int_in(*a, Rax);
                        self.asm.mov_rr(Rax, ra);
                        if matches!(op, IntOp::UDiv | IntOp::URem) {
                            self.asm.zero_div(Rcx);
                        } else {
                            self.asm.cqo_idiv(Rcx);
                        }
                        let out = if matches!(op, IntOp::SDiv | IntOp::UDiv) {
                            Rax
                        } else {
                            Rdx
                        };
                        self.put_int(*dst, out);
                    }
                    _ => {
                        // dst = a; dst op= b — unless dst aliases b's register.
                        let (mut a, mut b) = (*a, *b);
                        let commutative = !matches!(op, IntOp::Sub);
                        if commutative
                            && self.loc(b) == self.loc(*dst)
                            && self.loc(b) != Loc::Slot
                            && a != b
                        {
                            // dst op= a instead of copying a over b's register.
                            std::mem::swap(&mut a, &mut b);
                        }
                        let (a, b) = (&a, &b);
                        let rb = self.int_in(*b, Rcx);
                        let mut d = self.int_target(*dst, Rax);
                        if d == rb && a != b {
                            d = Rax;
                        }
                        let ra = self.int_in(*a, d);
                        self.asm.mov_rr(d, ra);
                        match op {
                            IntOp::Add => self.asm.alu_rr(Alu::Add, d, rb),
                            IntOp::Sub => self.asm.alu_rr(Alu::Sub, d, rb),
                            IntOp::And => self.asm.alu_rr(Alu::And, d, rb),
                            IntOp::Or => self.asm.alu_rr(Alu::Or, d, rb),
                            IntOp::Xor => self.asm.alu_rr(Alu::Xor, d, rb),
                            IntOp::Mul => self.asm.imul_rr(d, rb),
                            _ => unreachable!(),
                        }
                        self.put_int(*dst, d);
                    }
                }
            }
            Inst::IntImm { op, dst, a, imm } => self.int_imm(*op, *dst, *a, *imm),
            Inst::TestImm { zero, dst, a, imm } => {
                let ra = self.int_in(*a, Rax);
                self.asm.alu_ri(Alu::Test, ra, *imm);
                let d = self.int_target(*dst, Rax);
                self.asm.setcc(if *zero { Cond::E } else { Cond::Ne }, d);
                self.put_int(*dst, d);
            }
            Inst::ICmpImm { cond, dst, a, imm } => {
                let ra = self.int_in(*a, Rax);
                self.asm.cmp_ri(ra, *imm);
                let d = self.int_target(*dst, Rax);
                self.asm.setcc(icond(*cond), d);
                self.put_int(*dst, d);
            }
            Inst::Popcnt { dst, src } => {
                let r = self.int_in(*src, Rax);
                let d = self.int_target(*dst, Rax);
                self.asm.popcnt(d, r);
                self.put_int(*dst, d);
            }
            Inst::Neg { dst, src } | Inst::Not { dst, src } => {
                let d = self.int_target(*dst, Rax);
                let r = self.int_in(*src, d);
                self.asm.mov_rr(d, r);
                if matches!(inst, Inst::Neg { .. }) {
                    self.asm.neg(d);
                } else {
                    self.asm.not(d);
                }
                self.put_int(*dst, d);
            }
            Inst::MulOverflow { dst, ovf, a, b } => {
                let rb = self.int_in(*b, Rcx);
                self.asm.mov_rr(Rcx, rb);
                let ra = self.int_in(*a, Rax);
                self.asm.mov_rr(Rax, ra);
                self.asm.imul_rr(Rax, Rcx);
                self.asm.setcc(Cond::O, Rdx);
                self.put_int(*dst, Rax);
                self.put_int(*ovf, Rdx);
            }
            Inst::Float { op, dst, a, b } => {
                let xb = self.float_in(*b, X1);
                let mut d = self.float_target(*dst, X0);
                if d == xb && a != b {
                    d = X0;
                }
                let xa = self.float_in(*a, d);
                self.asm.movapd(d, xa);
                let sse = match op {
                    FloatOp::Add => Sse::Add,
                    FloatOp::Sub => Sse::Sub,
                    FloatOp::Mul => Sse::Mul,
                    FloatOp::Div => Sse::Div,
                };
                self.asm.sse(sse, d, xb);
                self.put_float(*dst, d);
            }
            Inst::ICmp { cond, dst, a, b } => {
                let ra = self.int_in(*a, Rax);
                self.alu_with(Alu::Cmp, ra, *b);
                let d = self.int_target(*dst, Rax);
                self.asm.setcc(icond(*cond), d);
                self.put_int(*dst, d);
            }
            Inst::FCmp { cond, dst, a, b } => {
                let xa = self.float_in(*a, X0);
                let xb = self.float_in(*b, X1);
                match cond {
                    // `a < b` ⇔ `b > a`: `ucomisd b, a` + `above` (false when unordered).
                    FCond::Olt => {
                        self.asm.ucomisd(xb, xa);
                        self.asm.setcc(Cond::A, Rax);
                    }
                    FCond::Ole => {
                        self.asm.ucomisd(xb, xa);
                        self.asm.setcc(Cond::Ae, Rax);
                    }
                    FCond::Ogt => {
                        self.asm.ucomisd(xa, xb);
                        self.asm.setcc(Cond::A, Rax);
                    }
                    FCond::Oge => {
                        self.asm.ucomisd(xa, xb);
                        self.asm.setcc(Cond::Ae, Rax);
                    }
                    FCond::Oeq => {
                        self.asm.ucomisd(xa, xb);
                        self.asm.setcc(Cond::E, Rax);
                        self.asm.setcc(Cond::Np, Rcx);
                        self.asm.alu_rr(Alu::And, Rax, Rcx);
                    }
                    FCond::Une => {
                        self.asm.ucomisd(xa, xb);
                        self.asm.setcc(Cond::Ne, Rax);
                        self.asm.setcc(Cond::P, Rcx);
                        self.asm.alu_rr(Alu::Or, Rax, Rcx);
                    }
                }
                self.put_int(*dst, Rax);
            }
            Inst::Select { dst, cond, a, b }
                if self.loc(*dst) == self.loc(*b) && matches!(self.loc(*b), Loc::Gpr(_)) =>
            {
                // dst already holds the fallback: only a conditional move.
                let Loc::Gpr(d) = self.loc(*dst) else {
                    unreachable!()
                };
                let ra = self.int_in(*a, Rcx);
                let rc = self.int_in(*cond, Rdx);
                self.asm.alu_rr(Alu::Test, rc, rc);
                self.asm.cmov(Cond::Ne, d, ra);
            }
            Inst::Select { dst, cond, a, b } => {
                let rb = self.int_in(*b, Rax);
                self.asm.mov_rr(Rax, rb);
                let ra = self.int_in(*a, Rcx);
                let rc = self.int_in(*cond, Rdx);
                self.asm.alu_rr(Alu::Test, rc, rc);
                self.asm.cmov(Cond::Ne, Rax, ra);
                self.put_int(*dst, Rax);
            }
            Inst::IntToFloat { dst, src } => {
                let r = self.int_in(*src, Rax);
                let d = self.float_target(*dst, X0);
                self.asm.xorpd(d, d); // break cvtsi2sd's false dependency
                self.asm.cvtsi2sd(d, r);
                self.put_float(*dst, d);
            }
            Inst::FloatToInt { dst, src } => {
                let x = self.float_in(*src, X0);
                let d = self.int_target(*dst, Rax);
                self.asm.cvttsd2si(d, x);
                self.put_int(*dst, d);
            }
            Inst::Load { dst, base, offset } => {
                let rb = self.int_in(*base, Rax);
                let m = Mem {
                    base: rb,
                    disp: *offset,
                };
                match self.func.class(*dst) {
                    Class::Int => {
                        let d = self.int_target(*dst, Rcx);
                        self.asm.load(d, m);
                        self.put_int(*dst, d);
                    }
                    Class::Float => {
                        let d = self.float_target(*dst, X0);
                        self.asm.movsd_load(d, m);
                        self.put_float(*dst, d);
                    }
                }
            }
            Inst::Store { src, base, offset } => {
                let rb = self.int_in(*base, Rax);
                let m = Mem {
                    base: rb,
                    disp: *offset,
                };
                match self.func.class(*src) {
                    Class::Int => {
                        let r = self.int_in(*src, Rcx);
                        self.asm.store(m, r);
                    }
                    Class::Float => {
                        let x = self.float_in(*src, X0);
                        self.asm.movsd_store(m, x);
                    }
                }
            }
            Inst::LoadIndex { dst, base, index } => {
                let rb = self.int_in(*base, Rax);
                let ri = self.int_in(*index, Rcx);
                let m = MemIdx {
                    base: rb,
                    index: ri,
                    disp: 0,
                };
                match self.func.class(*dst) {
                    Class::Int => {
                        let d = self.int_target(*dst, Rdx);
                        self.asm.load_idx(d, m);
                        self.put_int(*dst, d);
                    }
                    Class::Float => {
                        let d = self.float_target(*dst, X0);
                        self.asm.movsd_load_idx(d, m);
                        self.put_float(*dst, d);
                    }
                }
            }
            Inst::StoreIndex { src, base, index } => {
                let rb = self.int_in(*base, Rax);
                let ri = self.int_in(*index, Rcx);
                let m = MemIdx {
                    base: rb,
                    index: ri,
                    disp: 0,
                };
                match self.func.class(*src) {
                    Class::Int => {
                        let r = self.int_in(*src, Rdx);
                        self.asm.store_idx(m, r);
                    }
                    Class::Float => {
                        let x = self.float_in(*src, X0);
                        self.asm.movsd_store_idx(m, x);
                    }
                }
            }
            Inst::Call {
                target,
                args,
                dst,
                ret32,
            } => {
                // Caller-saved registers holding values needed after the call, and every
                // register-resident argument, go to their home slots; arguments are then loaded
                // from slots so filling one argument register can't clobber another's source.
                let across: Vec<VReg> = self
                    .alloc
                    .live_across
                    .get(&(bi, ii))
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|v| is_caller_saved(self.loc(*v)))
                    .collect();
                for v in &across {
                    self.save(*v);
                }
                for a in args {
                    if !across.contains(a) {
                        self.save(*a);
                    }
                }
                // The runtime table pointer may itself sit in an argument register.
                let rt = self.int_in(super::ir::RT, Rax);
                self.asm.mov_rr(Rax, rt);
                let (mut ni, mut nf) = (0usize, 0u8);
                for a in args {
                    match self.func.class(*a) {
                        Class::Int => {
                            self.asm.load(INT_ARGS[ni], slot(*a));
                            ni += 1;
                        }
                        Class::Float => {
                            self.asm.movsd_load(Xmm(nf), slot(*a));
                            nf += 1;
                        }
                    }
                }
                self.asm.call_mem(Mem {
                    base: Rax,
                    disp: 8 * (*target as i32),
                });
                if let Some(d) = dst {
                    match self.func.class(*d) {
                        Class::Int => {
                            if *ret32 {
                                self.asm.mov_rr32(Rax, Rax);
                            }
                            self.put_int(*d, Rax);
                        }
                        Class::Float => self.put_float(*d, X0),
                    }
                }
                for v in &across {
                    self.restore(*v);
                }
            }
        }
    }
}
