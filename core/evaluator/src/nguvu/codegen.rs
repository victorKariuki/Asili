//! IR → x86-64, for the System V (Linux, macOS, BSD) and Microsoft x64 (Windows) calling
//! conventions ([`Abi`]).
//!
//! Frame layout: `rbp` frame pointer, the ABI's callee-saved general registers pushed, the
//! callee-saved xmm registers this function uses (Win64 only, all 128 bits), then one 8-byte
//! home slot per virtual register, then the callee's shadow space (Win64). Virtual registers live where [`regalloc`] put them; operands
//! are used in place when they sit in a register, and `rax`/`rcx`/`rdx`, `xmm0`/`xmm1` are
//! scratch. A comparison feeding the block's branch is fused into `cmp` + `jcc`.

use super::ir::{Class, FCond, FloatOp, Func, Home, ICond, Inst, IntOp, Term, VReg};
use super::regalloc::{allocate, Allocation, Target, LOOP_ALIGN};
use super::schedule::Step;

/// Where a value lives, in x86-64 terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Loc {
    Gpr(Gpr),
    Xmm(Xmm),
    Slot,
}

/// A calling convention: which registers carry arguments and survive calls, and what the
/// caller owes the callee on the stack. `rax`, `rcx`, `rdx`, `xmm0`, `xmm1` are always scratch
/// and `rsp`/`rbp` frame the stack.
pub struct Abi {
    pub target: Target,
    /// Callee-saved general registers, pushed after `rbp` in the prologue when used.
    pushed: &'static [Gpr],
    /// Integer argument registers.
    args: &'static [Gpr],
    /// Arguments take the register of their position whatever their class (Win64), rather than
    /// the next free register of their class (System V).
    positional: bool,
    /// Bytes the caller reserves above the return address for the callee (Win64: 32).
    shadow: i32,
    /// Frames larger than a page must touch each page in order (Windows' guard pages).
    probe: bool,
}

/// System V AMD64 (Linux, macOS, BSD): every xmm register is caller-saved.
pub const SYSV: Abi = Abi {
    target: Target {
        int_callee_saved: &[3, 12, 13, 14, 15],  // rbx, r12–r15
        int_caller_saved: &[6, 7, 8, 9, 10, 11], // rsi, rdi, r8–r11
        float_callee_saved: &[],
        float_caller_saved: &[2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    pushed: &[Gpr::Rbx, Gpr::R12, Gpr::R13, Gpr::R14, Gpr::R15],
    args: &[Gpr::Rdi, Gpr::Rsi, Gpr::Rdx, Gpr::Rcx, Gpr::R8, Gpr::R9],
    positional: false,
    shadow: 0,
    probe: false,
};

/// Microsoft x64 (Windows): `rsi`, `rdi` and `xmm6`–`xmm15` are callee-saved too.
pub const WIN64: Abi = Abi {
    target: Target {
        int_callee_saved: &[3, 6, 7, 12, 13, 14, 15], // rbx, rsi, rdi, r12–r15
        int_caller_saved: &[8, 9, 10, 11],            // r8–r11
        float_callee_saved: &[6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        float_caller_saved: &[2, 3, 4, 5],
    },
    pushed: &[
        Gpr::Rbx,
        Gpr::Rsi,
        Gpr::Rdi,
        Gpr::R12,
        Gpr::R13,
        Gpr::R14,
        Gpr::R15,
    ],
    args: &[Gpr::Rcx, Gpr::Rdx, Gpr::R8, Gpr::R9],
    positional: true,
    shadow: 32,
    probe: true,
};

/// The convention of the platform this build runs on.
pub const HOST: &Abi = if cfg!(windows) { &WIN64 } else { &SYSV };

const GPRS: [Gpr; 16] = [
    Gpr::Rax,
    Gpr::Rcx,
    Gpr::Rdx,
    Gpr::Rbx,
    Gpr::Rsp,
    Gpr::Rbp,
    Gpr::Rsi,
    Gpr::Rdi,
    Gpr::R8,
    Gpr::R9,
    Gpr::R10,
    Gpr::R11,
    Gpr::R12,
    Gpr::R13,
    Gpr::R14,
    Gpr::R15,
];
use super::x64::{Alu, Asm, Cond, Gpr, Label, Mem, MemIdx, Sse, Xmm};

const X0: Xmm = Xmm(0);
const X1: Xmm = Xmm(1);

struct Gen<'f> {
    asm: Asm,
    func: &'f Func,
    alloc: Allocation,
    abi: &'static Abi,
    /// Callee-saved general registers this function uses, pushed after `rbp`.
    pushed: Vec<Gpr>,
    /// Callee-saved xmm registers this function uses (Win64), saved below the pushes.
    xmm_saved: Vec<u8>,
    /// Direct calls to link: (displacement offset, callee).
    calls: Vec<(usize, u32)>,
    /// What each register's home slot holds at a call (see `Func::homes`).
    homes: Vec<Home>,
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
pub fn generate(func: &Func, abi: &'static Abi) -> super::Code {
    let alloc = allocate(func, &abi.target);
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
    let mut xmm_saved: Vec<u8> = alloc
        .loc
        .iter()
        .filter_map(|l| match l {
            super::regalloc::Loc::Float(x) if abi.target.float_callee_saved.contains(x) => Some(*x),
            _ => None,
        })
        .collect();
    xmm_saved.sort_unstable();
    xmm_saved.dedup();
    let used = |r: Gpr| {
        alloc
            .loc
            .iter()
            .any(|l| matches!(l, super::regalloc::Loc::Int(i) if GPRS[*i as usize] == r))
    };
    let pushed = abi.pushed.iter().copied().filter(|r| used(*r)).collect();
    let mut g = Gen {
        asm: Asm::new(),
        func,
        alloc,
        abi,
        pushed,
        xmm_saved,
        calls: Vec::new(),
        homes: func.homes(),
    };
    g.prologue();
    let labels: Vec<_> = (0..func.blocks.len()).map(|_| g.asm.new_label()).collect();
    let order = g.alloc.order.clone();
    for (k, &bi) in order.iter().enumerate() {
        let block = &func.blocks[bi];
        if g.alloc.loop_head[bi] {
            g.asm.align(LOOP_ALIGN);
        }
        g.asm.bind(labels[bi]);
        let next = order.get(k + 1).map(|&b| labels[b]);
        let plan = super::schedule::plan(block, &g.alloc.uses);
        let fused = plan.fused_branch;
        for step in plan.steps {
            match step {
                Step::Inst(i) => g.inst(bi, i, &block.insts[i]),
                // Conditional moves and plain copies leave the flags intact.
                Step::FlagSelects { cmp, run } => {
                    let cc = g.flags_for(&block.insts[cmp]);
                    for i in run {
                        match &block.insts[i] {
                            Inst::Select { dst, a, b, .. } => g.select_on(cc, *dst, *a, *b),
                            other => g.inst(bi, i, other),
                        }
                    }
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
    super::Code {
        bytes: g.asm.finish(),
        calls: g.calls,
    }
}

impl<'f> Gen<'f> {
    /// Bytes between `rbp` and the first home slot: pushed registers and saved xmm registers.
    fn saved_bytes(&self) -> i32 {
        8 * self.pushed.len() as i32 + 16 * self.xmm_saved.len() as i32
    }

    /// The direct-call register buffer, below the home slots.
    fn call_buffer(&self) -> Mem {
        Mem {
            base: Gpr::Rbp,
            disp: -self.saved_bytes() - 8 * self.alloc.slots as i32 - self.func.call_buffer as i32,
        }
    }

    fn slot_index(&self, v: VReg) -> u32 {
        let i = self.alloc.slot[v.0 as usize];
        debug_assert!(i != u32::MAX, "{v:?} has no home slot");
        i
    }

    fn slot(&self, v: VReg) -> Mem {
        Mem {
            base: Gpr::Rbp,
            disp: -self.saved_bytes() - 8 * (self.slot_index(v) as i32 + 1),
        }
    }

    fn prologue(&mut self) {
        let slots = self.alloc.slots as i32;
        let pushed = self.pushed.len() as i32;
        // At entry `rsp` is 8 mod 16 (the return address); `push rbp` realigns it, so after the
        // pushes it is `8 * pushed` mod 16 and the rest of the frame must restore alignment for
        // calls.
        let mut frame = 16 * self.xmm_saved.len() as i32
            + 8 * slots
            + self.func.call_buffer as i32
            + self.abi.shadow;
        if (8 * pushed + frame) % 16 != 0 {
            frame += 8;
        }
        self.asm.push(Gpr::Rbp);
        self.asm.mov_rr(Gpr::Rbp, Gpr::Rsp);
        for r in self.pushed.clone() {
            self.asm.push(r);
        }
        if self.abi.probe && frame > 4096 {
            // Windows commits the stack one guard page at a time: touch each page in order.
            let mut left = frame;
            while left > 4096 {
                self.asm.alu_ri(Alu::Sub, Gpr::Rsp, 4096);
                self.asm.test_mem(
                    Mem {
                        base: Gpr::Rsp,
                        disp: 0,
                    },
                    Gpr::Rsp,
                );
                left -= 4096;
            }
            self.asm.alu_ri(Alu::Sub, Gpr::Rsp, left);
        } else {
            self.asm.alu_ri(Alu::Sub, Gpr::Rsp, frame);
        }
        for (i, x) in self.xmm_saved.clone().into_iter().enumerate() {
            let m = Mem {
                base: Gpr::Rbp,
                disp: -8 * pushed - 16 * (i as i32 + 1),
            };
            self.asm.movups_store(m, Xmm(x));
        }
        // Incoming arguments. One that lives in a callee-saved register moves straight there
        // (no argument register is callee-saved, so nothing is overwritten) and needs no home
        // slot: calls take it from that register. The others are parked in their home slots
        // first (their allocated registers may be other argument registers), then loaded.
        let args: Vec<Gpr> = self.abi.args.iter().take(4).copied().collect();
        for (i, r) in args.iter().enumerate() {
            if let Some(home) = self.kept(VReg(i as u32)) {
                self.asm.mov_rr(home, *r);
            }
        }
        for (i, r) in args.iter().enumerate() {
            if self.kept(VReg(i as u32)).is_none() {
                self.asm.store(self.slot(VReg(i as u32)), *r);
            }
        }
        for i in 0..4 {
            let v = VReg(i);
            if let (Loc::Gpr(r), None) = (self.loc(v), self.kept(v)) {
                self.asm.load(r, self.slot(v));
            }
        }
    }

    /// The callee-saved register holding integer `v` across calls, if it has one.
    fn kept(&self, v: VReg) -> Option<Gpr> {
        match self.loc(v) {
            Loc::Gpr(r) if self.pushed.contains(&r) => Some(r),
            _ => None,
        }
    }

    fn loc(&self, v: VReg) -> Loc {
        match self.alloc.loc[v.0 as usize] {
            super::regalloc::Loc::Int(r) => Loc::Gpr(GPRS[r as usize]),
            super::regalloc::Loc::Float(r) => Loc::Xmm(Xmm(r)),
            super::regalloc::Loc::Slot => Loc::Slot,
        }
    }

    fn epilogue(&mut self) {
        let pushed = self.pushed.len() as i32;
        for (i, x) in self.xmm_saved.clone().into_iter().enumerate() {
            let m = Mem {
                base: Gpr::Rbp,
                disp: -8 * pushed - 16 * (i as i32 + 1),
            };
            self.asm.movups_load(Xmm(x), m);
        }
        let a = &mut self.asm;
        a.lea(
            Gpr::Rsp,
            Mem {
                base: Gpr::Rbp,
                disp: -8 * pushed,
            },
        );
        for r in self.pushed.iter().rev() {
            a.pop(*r);
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
            Loc::Slot => self.asm.alu_rm(op, dst, self.slot(v)),
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
                self.asm.load(scratch, self.slot(v));
                scratch
            }
            Loc::Xmm(_) => unreachable!("integer value in an xmm register"),
        }
    }

    fn float_in(&mut self, v: VReg, scratch: Xmm) -> Xmm {
        match self.loc(v) {
            Loc::Xmm(x) => x,
            Loc::Slot => {
                self.asm.movsd_load(scratch, self.slot(v));
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
            Loc::Slot => self.asm.store(self.slot(v), src),
        }
    }

    fn put_float(&mut self, v: VReg, src: Xmm) {
        match self.loc(v) {
            Loc::Xmm(x) if x == src => {}
            Loc::Xmm(x) => self.asm.movapd(x, src),
            Loc::Gpr(r) => self.asm.movq_rx(r, src),
            Loc::Slot => self.asm.movsd_store(self.slot(v), src),
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
                self.asm.load(scratch, self.slot(v));
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
    /// Make `v`'s home slot current (a no-op when it already is, or `v` is a constant that
    /// `restore` and `load_call_args` rematerialize).
    fn save(&mut self, v: VReg) {
        if self.homes[v.0 as usize] != Home::Unknown {
            return;
        }
        match self.loc(v) {
            Loc::Gpr(r) => self.asm.store(self.slot(v), r),
            Loc::Xmm(x) => self.asm.movsd_store(self.slot(v), x),
            Loc::Slot => {}
        }
    }

    /// Reload `v` after a call (clobbers `rax` for a float constant).
    fn restore(&mut self, v: VReg) {
        if let Home::Const(bits) = self.homes[v.0 as usize] {
            match self.loc(v) {
                Loc::Gpr(r) => self.asm.mov_ri(r, bits),
                Loc::Xmm(x) => {
                    self.asm.mov_ri(Gpr::Rax, bits);
                    self.asm.movq_xr(x, Gpr::Rax);
                }
                Loc::Slot => {}
            }
            return;
        }
        match self.loc(v) {
            Loc::Gpr(r) => self.asm.load(r, self.slot(v)),
            Loc::Xmm(x) => self.asm.movsd_load(x, self.slot(v)),
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
        let (magic, shift) = super::ir::div_magic(d as u32);
        if unsigned {
            self.asm.mov_ri(Rdx, magic as i64);
            self.asm.mul(Rdx);
            if shift > 0 {
                self.asm.shift_ri(false, Rdx, shift); // rdx = a / d
            }
        } else {
            // m = a >> 63 (all ones if negative); |a| = (a ^ m) - m
            self.asm.mov_rr(Rcx, Rax);
            self.asm.shift_ri(false, Rcx, 63);
            self.asm.alu_rr(Alu::Xor, Rax, Rcx);
            self.asm.alu_rr(Alu::Sub, Rax, Rcx);
            self.asm.mov_ri(Rdx, magic as i64);
            self.asm.mul(Rdx); // rdx = |a| / d
            if shift > 0 {
                self.asm.shift_ri(false, Rdx, shift);
            }
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

    /// Before a call: caller-saved registers holding values needed after it, and every
    /// register-resident argument, go to their home slots (arguments are then loaded from
    /// slots, so filling one argument register can't clobber another's source). Returns the
    /// values to restore afterwards.
    fn save_for_call(&mut self, bi: usize, ii: usize, args: &[VReg]) -> Vec<VReg> {
        let across: Vec<VReg> = self
            .alloc
            .live_across
            .get(&(bi, ii))
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|v| {
                self.abi
                    .target
                    .is_caller_saved(self.alloc.loc[v.0 as usize])
            })
            .collect();
        for v in &across {
            self.save(*v);
        }
        for a in args {
            if !across.contains(a) && self.kept(*a).is_none() {
                self.save(*a);
            }
        }
        across
    }

    /// Load call arguments from their home slots into the ABI's argument registers (leaves
    /// `rax` alone).
    fn load_call_args(&mut self, args: &[VReg]) {
        let (mut ni, mut nf) = (0usize, 0u8);
        for (pos, a) in args.iter().enumerate() {
            // Win64 gives argument `pos` the pos-th register of its class; System V the next
            // free one.
            let (i, f) = if self.abi.positional {
                (pos, pos as u8)
            } else {
                (ni, nf)
            };
            let constant = match self.homes[a.0 as usize] {
                Home::Const(bits) if self.loc(*a) != Loc::Slot => Some(bits),
                _ => None,
            };
            match (self.func.class(*a), constant) {
                (Class::Int, Some(bits)) => self.asm.mov_ri(self.abi.args[i], bits),
                // A callee-saved register is never an argument register: no move clobbers it.
                (Class::Int, None) => match self.kept(*a) {
                    Some(r) => self.asm.mov_rr(self.abi.args[i], r),
                    None => self.asm.load(self.abi.args[i], self.slot(*a)),
                },
                // `r11` is caller-saved and never an argument register.
                (Class::Float, Some(bits)) => {
                    self.asm.mov_ri(Gpr::R11, bits);
                    self.asm.movq_xr(Xmm(f), Gpr::R11);
                }
                (Class::Float, None) => self.asm.movsd_load(Xmm(f), self.slot(*a)),
            }
            match self.func.class(*a) {
                Class::Int => ni += 1,
                Class::Float => nf += 1,
            }
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
            Inst::LoadIndex {
                dst,
                base,
                index,
                kind,
            } => {
                let width = kind.width() as u8;
                let rb = self.int_in(*base, Rax);
                let ri = self.int_in(*index, Rcx);
                let m = MemIdx {
                    base: rb,
                    index: ri,
                    disp: 0,
                    scale: width,
                };
                match self.func.class(*dst) {
                    Class::Int => {
                        let d = self.int_target(*dst, Rdx);
                        if width == 8 {
                            self.asm.load_idx(d, m);
                        } else {
                            self.asm.load_idx_ext(d, m, width, kind.signed());
                        }
                        self.put_int(*dst, d);
                    }
                    Class::Float => {
                        let d = self.float_target(*dst, X0);
                        self.asm.movsd_load_idx(d, m);
                        self.put_float(*dst, d);
                    }
                }
            }
            Inst::StoreIndex {
                src,
                base,
                index,
                kind,
            } => {
                let width = kind.width() as u8;
                let rb = self.int_in(*base, Rax);
                let ri = self.int_in(*index, Rcx);
                let m = MemIdx {
                    base: rb,
                    index: ri,
                    disp: 0,
                    scale: width,
                };
                match self.func.class(*src) {
                    Class::Int => {
                        let r = self.int_in(*src, Rdx);
                        if width == 8 {
                            self.asm.store_idx(m, r);
                        } else {
                            self.asm.store_idx_narrow(m, r, width);
                        }
                    }
                    Class::Float => {
                        let x = self.float_in(*src, X0);
                        self.asm.movsd_store_idx(m, x);
                    }
                }
            }
            Inst::StackPointer { dst } => self.put_int(*dst, Rsp),
            Inst::CallBuffer { dst } => {
                let d = self.int_target(*dst, Rax);
                self.asm.lea(d, self.call_buffer());
                self.put_int(*dst, d);
            }
            Inst::CallDirect { func, args, dst } => {
                let across = self.save_for_call(bi, ii, args);
                self.load_call_args(args);
                let at = self.asm.call_rel32();
                self.calls.push((at, *func));
                self.put_int(*dst, Rax);
                for v in &across {
                    self.restore(*v);
                }
            }
            Inst::Call {
                target,
                args,
                dst,
                ret32,
            } => {
                let across = self.save_for_call(bi, ii, args);
                // The runtime table pointer: its callee-saved register, else its home slot
                // (the prologue parks it there; runtime calls read it implicitly, so its
                // register need not be live).
                match self.kept(super::ir::RT) {
                    Some(r) => self.asm.mov_rr(Rax, r),
                    None => self.asm.load(Rax, self.slot(super::ir::RT)),
                }
                self.load_call_args(args);
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
