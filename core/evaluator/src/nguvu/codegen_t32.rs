//! Thumb-2 code generation for Cortex-M (ARMv7E-M with the `fpv5-d16` floating-point unit,
//! AAPCS-VFP) from allocated IR, through the [`t32`] encoder — the device target for strict
//! code (`pata jenga --lengo cortex-m`, see [`super::device`]).
//!
//! The IR's integers are 64-bit; here each lives in a *pair* of 32-bit registers (low word
//! first), so every integer instruction becomes a short sequence over both words with exactly
//! the IR's 64-bit results. The allocator is handed only callee-saved registers — the pairs
//! `r4:r5`, `r6:r7`, `r8:r9` and `d8`–`d15` — so nothing allocated is ever clobbered by a call:
//! `r0`–`r3`, `r10`–`r12`, `lr` and `d0`–`d7` are scratch (`lr` forms large addresses).
//!
//! Frame: `push {r3-r11, lr}`, `vpush {d8-d15}` when an allocated double register is used,
//! then virtual register `v`'s home slot at `[sp + 8v]` and the direct-call buffer above the
//! slots. Calls to the device runtime and to the EABI 64-bit division helpers are relocations
//! (`bl asili_kifaa_*`, `bl __aeabi_ldivmod`); calls between functions of one image are linked
//! by [`super::device`].

use super::ir::{Class, FCond, FloatOp, Func, ICond, Inst, IntOp, RtFn, Term, VReg};
use super::regalloc::{allocate, Allocation, Loc, Target};
use super::schedule::Step;
use super::t32::{Asm, Cond, Dp, Fop, Label, Reloc, Shift, LR, SP};

pub const TARGET: Target = Target {
    int_callee_saved: &[2, 3, 4],
    int_caller_saved: &[],
    float_callee_saved: &[8, 9, 10, 11, 12, 13, 14, 15],
    float_caller_saved: &[],
};

/// The registers of integer pair `k`: `(low word, high word)`.
fn pair(k: u8) -> (u8, u8) {
    match k {
        0 => (0, 1),
        1 => (2, 3),
        2 => (4, 5),
        3 => (6, 7),
        4 => (8, 9),
        _ => (10, 11),
    }
}

type Pair = (u8, u8);

/// Operand scratch pairs.
const S0: Pair = (0, 1);
const S1: Pair = (2, 3);
const S2: Pair = (10, 11);
/// A single scratch word.
const T: u8 = 12;
/// Large-address formation (also scratch).
const ADDR: u8 = LR;
/// Double scratch registers.
const F0: u8 = 0;
const F1: u8 = 1;
/// Conversion temporaries (`d6` = `s12`/`s13`, `d7` = `s14`/`s15`, `d4` = `s8`/`s9`).
const C0: u8 = 6;
const C1: u8 = 7;
const C2: u8 = 4;

fn icond_swapped(c: ICond) -> (Cond, bool) {
    // (condition after comparing a with b — or b with a when `swap`, 64-bit compare via `sbcs`)
    match c {
        ICond::Lt => (Cond::Lt, false),
        ICond::Ge => (Cond::Ge, false),
        ICond::Gt => (Cond::Lt, true),
        ICond::Le => (Cond::Ge, true),
        ICond::Ult => (Cond::Lo, false),
        ICond::Ule => (Cond::Hs, true),
        ICond::Eq => (Cond::Eq, false),
        ICond::Ne => (Cond::Ne, false),
    }
}

/// After `vcmp` + `vmrs`, an unordered result sets `C` and `V`: these conditions are false for
/// NaN operands except `ne`, exactly the IR's semantics.
fn fcond(c: FCond) -> Cond {
    match c {
        FCond::Olt => Cond::Mi,
        FCond::Ole => Cond::Ls,
        FCond::Ogt => Cond::Gt,
        FCond::Oge => Cond::Ge,
        FCond::Oeq => Cond::Eq,
        FCond::Une => Cond::Ne,
    }
}

/// Symbol of the device runtime function implementing `target`, or `None` where the device
/// has no host to provide it (the build rejects the function).
pub fn runtime_symbol(target: RtFn) -> Option<&'static str> {
    Some(match target {
        // Only on an error path (an index out of range): the call fails, as in the host.
        // `device::check_host_calls` rejects any other instruction handed to the host.
        RtFn::Exec => "asili_kifaa_exec",
        RtFn::ListPtr => "asili_kifaa_list_ptr",
        RtFn::ListLen => "asili_kifaa_list_len",
        RtFn::Fmod => "asili_kifaa_fmod",
        RtFn::Pow => "asili_kifaa_pow",
        RtFn::FloatToIntSat => "asili_kifaa_float_to_int_sat",
        RtFn::ShiftAmount => "asili_kifaa_shift_amount",
        RtFn::CallHost => "asili_kifaa_call_host",
        RtFn::DepthError => "asili_kifaa_depth_error",
        // Inlined (`vrintm` / `vrintp`).
        RtFn::Floor | RtFn::Ceil => "",
        _ => return None,
    })
}

/// One function's code: bytes, direct calls to link (`bl` offset, callee), and relocations.
pub struct Code {
    pub bytes: Vec<u8>,
    pub calls: Vec<(usize, u32)>,
    pub relocs: Vec<(usize, String, Reloc)>,
}

struct Gen<'f> {
    asm: Asm,
    func: &'f Func,
    alloc: Allocation,
    /// Whether `d8`–`d15` are saved (some allocated double register is used).
    saves_floats: bool,
    frame: u32,
    calls: Vec<(usize, u32)>,
}

/// Thumb-2 code for `func`; an instruction the device cannot run is an error naming it.
pub fn generate(func: &Func) -> Result<Code, String> {
    let alloc = allocate(func, &TARGET);
    let saves_floats = alloc.loc.iter().any(|l| matches!(l, Loc::Float(_)));
    let frame = (8 * alloc.slots + func.call_buffer).div_ceil(8) * 8;
    if frame > 1 << 20 {
        return Err("kazi ina rejista nyingi mno kwa Cortex-M".into());
    }
    let mut g = Gen {
        asm: Asm::new(),
        func,
        alloc,
        saves_floats,
        frame,
        calls: Vec::new(),
    };
    g.prologue();
    let labels: Vec<Label> = (0..func.blocks.len()).map(|_| g.asm.new_label()).collect();
    let order = g.alloc.order.clone();
    for (k, &bi) in order.iter().enumerate() {
        let block = &func.blocks[bi];
        if g.alloc.loop_head[bi] {
            g.asm.align(4);
        }
        g.asm.bind(labels[bi]);
        let next = order.get(k + 1).map(|&b| labels[b]);
        let plan = super::schedule::plan(block, &g.alloc.uses);
        for step in plan.steps {
            match step {
                Step::Inst(i) => g.inst(&block.insts[i])?,
                Step::FlagSelects { cmp, run } => {
                    let cc = g.flags_for(&block.insts[cmp])?;
                    for i in run {
                        match &block.insts[i] {
                            Inst::Select { dst, a, b, .. } => g.select_on(cc, *dst, *a, *b),
                            other => g.inst(other)?,
                        }
                    }
                }
            }
        }
        match &block.term {
            Term::Jump(t) => {
                if Some(labels[t.0 as usize]) != next {
                    g.asm.b(labels[t.0 as usize]);
                }
            }
            Term::Branch { cond, then_, else_ } => {
                let (t, e) = (labels[then_.0 as usize], labels[else_.0 as usize]);
                let cc = if plan.fused_branch {
                    let last = block.insts.last().ok_or("nguvu: ulinganisho haupo")?;
                    g.flags_for(last)?
                } else {
                    g.nonzero_flags(*cond)
                };
                g.branch_on(cc, t, e, next);
            }
            Term::Return(v) => {
                let (lo, hi) = g.int_in(*v, S0);
                g.mov_pair(S0, (lo, hi));
                g.epilogue();
            }
            Term::ReturnNum(v) => {
                let d = g.float_in(*v, F0);
                if d != 0 {
                    g.asm.vmov(0, d);
                }
                g.asm.mov_imm(0, 0);
                g.asm.mov_imm(1, 0);
                g.epilogue();
            }
        }
    }
    let (bytes, relocs) = g.asm.finish()?;
    Ok(Code {
        bytes,
        calls: g.calls,
        relocs,
    })
}

impl<'f> Gen<'f> {
    fn loc(&self, v: VReg) -> Loc {
        self.alloc.loc[v.0 as usize]
    }

    fn slot(&self, v: VReg) -> u32 {
        8 * self.alloc.slot[v.0 as usize]
    }

    fn prologue(&mut self) {
        self.asm.push(0x4FF8); // r3-r11, lr: ten words keep `sp` 8-byte aligned
        if self.saves_floats {
            self.asm.vpush(8, 8);
        }
        self.asm.adjust_sp(self.frame, true, T);
        // Incoming arguments (pointer-sized, `r0` on): zero-extend into their homes.
        for (i, &v) in self.func.args.iter().enumerate() {
            let r = i as u8;
            self.asm.mov_imm(T, 0);
            let off = self.slot(v);
            self.st64(r, T, SP, off);
            if let Loc::Int(k) = self.loc(v) {
                let (lo, hi) = pair(k);
                self.asm.mov(lo, r);
                self.asm.mov_imm(hi, 0);
            }
        }
    }

    fn epilogue(&mut self) {
        self.asm.adjust_sp(self.frame, false, T);
        if self.saves_floats {
            self.asm.vpop(8, 8);
        }
        self.asm.pop(0x8FF8); // r3-r11, pc
    }

    fn branch_on(&mut self, cc: Cond, t: Label, e: Label, next: Option<Label>) {
        if Some(t) == next {
            self.asm.b_cond(cc.invert(), e);
        } else {
            self.asm.b_cond(cc, t);
            if Some(e) != next {
                self.asm.b(e);
            }
        }
    }

    // ----- memory ------------------------------------------------------------------------

    /// `[base + off]` as a base register and an offset of at most `limit` (forming the address
    /// in `lr` when `off` is larger).
    fn at(&mut self, base: u8, off: u32, limit: u32) -> (u8, u32) {
        if off <= limit {
            (base, off)
        } else {
            self.asm.add_any(ADDR, base, off);
            (ADDR, 0)
        }
    }

    /// Load the 64-bit word at `[base + off]` into `(lo, hi)`.
    fn ld64(&mut self, lo: u8, hi: u8, base: u8, off: u32) {
        let (b, o) = self.at(base, off, 1020);
        self.asm.ldrd(lo, hi, b, o);
    }

    fn st64(&mut self, lo: u8, hi: u8, base: u8, off: u32) {
        let (b, o) = self.at(base, off, 1020);
        self.asm.strd(lo, hi, b, o);
    }

    fn vld(&mut self, d: u8, base: u8, off: u32) {
        let (b, o) = self.at(base, off, 1020);
        self.asm.vldr(d, b, o);
    }

    fn vst(&mut self, d: u8, base: u8, off: u32) {
        let (b, o) = self.at(base, off, 1020);
        self.asm.vstr(d, b, o);
    }

    // ----- operands ----------------------------------------------------------------------

    fn int_in(&mut self, v: VReg, scratch: Pair) -> Pair {
        match self.loc(v) {
            Loc::Int(k) => pair(k),
            _ => {
                let off = self.slot(v);
                self.ld64(scratch.0, scratch.1, SP, off);
                scratch
            }
        }
    }

    fn float_in(&mut self, v: VReg, scratch: u8) -> u8 {
        match self.loc(v) {
            Loc::Float(d) => d,
            _ => {
                let off = self.slot(v);
                self.vld(scratch, SP, off);
                scratch
            }
        }
    }

    fn int_target(&self, v: VReg, scratch: Pair) -> Pair {
        match self.loc(v) {
            Loc::Int(k) => pair(k),
            _ => scratch,
        }
    }

    fn float_target(&self, v: VReg, scratch: u8) -> u8 {
        match self.loc(v) {
            Loc::Float(d) => d,
            _ => scratch,
        }
    }

    fn mov_pair(&mut self, dst: Pair, src: Pair) {
        if dst != src {
            self.asm.mov(dst.0, src.0);
            self.asm.mov(dst.1, src.1);
        }
    }

    fn put_int(&mut self, v: VReg, src: Pair) {
        match self.loc(v) {
            Loc::Int(k) => self.mov_pair(pair(k), src),
            _ => {
                let off = self.slot(v);
                self.st64(src.0, src.1, SP, off);
            }
        }
    }

    fn put_float(&mut self, v: VReg, src: u8) {
        match self.loc(v) {
            Loc::Float(d) => {
                if d != src {
                    self.asm.vmov(d, src);
                }
            }
            _ => {
                let off = self.slot(v);
                self.vst(src, SP, off);
            }
        }
    }

    /// `(lo, hi) = value` (64-bit).
    fn mov_imm64(&mut self, p: Pair, value: i64) {
        self.asm.mov_imm(p.0, value as u32);
        self.asm.mov_imm(p.1, (value >> 32) as u32);
    }

    /// `d = bits` (as a double).
    fn fconst(&mut self, d: u8, bits: u64) {
        self.asm.mov_imm(T, bits as u32);
        self.asm.mov_imm(ADDR, (bits >> 32) as u32);
        self.asm.vmov_from_core(d, T, ADDR);
    }

    // ----- comparisons -------------------------------------------------------------------

    /// Flags for `(lo, hi) != 0`, returning the condition meaning "true".
    fn nonzero_flags(&mut self, v: VReg) -> Cond {
        let (lo, hi) = self.int_in(v, S0);
        self.asm.dp(Dp::Orr, true, T, lo, hi);
        Cond::Ne
    }

    /// Flags comparing pairs `a` and `b` for `c`.
    fn compare_pairs(&mut self, c: ICond, a: Pair, b: Pair) -> Cond {
        let (cc, swap) = icond_swapped(c);
        let (x, y) = if swap { (b, a) } else { (a, b) };
        match c {
            ICond::Eq | ICond::Ne => {
                self.asm.cmp(x.1, y.1);
                self.asm.it(Cond::Eq, &[true]);
                self.asm.cmp(x.0, y.0);
            }
            _ => {
                self.asm.cmp(x.0, y.0);
                self.asm.dp(Dp::Sbc, true, T, x.1, y.1);
            }
        }
        cc
    }

    /// Set the flags for a comparison and return the condition meaning "true".
    fn flags_for(&mut self, cmp: &Inst) -> Result<Cond, String> {
        Ok(match cmp {
            Inst::ICmp { cond, a, b, .. } => {
                let pa = self.int_in(*a, S0);
                let pb = self.int_in(*b, S1);
                self.compare_pairs(*cond, pa, pb)
            }
            Inst::ICmpImm { cond, a, imm, .. } => {
                let pa = self.int_in(*a, S0);
                self.mov_imm64(S1, *imm as i64);
                self.compare_pairs(*cond, pa, S1)
            }
            Inst::TestImm { zero, a, imm, .. } => {
                let (lo, hi) = self.int_in(*a, S0);
                self.asm.mov_imm(T, *imm as u32);
                if *imm >= 0 {
                    self.asm.tst(lo, T);
                } else {
                    // The mask's high word is all ones: test both words.
                    self.asm.dp(Dp::And, false, T, lo, T);
                    self.asm.dp(Dp::Orr, true, T, T, hi);
                }
                if *zero {
                    Cond::Eq
                } else {
                    Cond::Ne
                }
            }
            Inst::FCmp { cond, a, b, .. } => {
                let da = self.float_in(*a, F0);
                let db = self.float_in(*b, F1);
                self.asm.vcmp(da, db);
                fcond(*cond)
            }
            _ => return Err("nguvu: si ulinganisho".into()),
        })
    }

    /// `dst = cc ? a : b` with the flags already set (no instruction here touches them).
    fn select_on(&mut self, cc: Cond, dst: VReg, a: VReg, b: VReg) {
        if self.func.class(dst) == Class::Float {
            let da = self.float_in(a, F0);
            let db = self.float_in(b, F1);
            let d = self.float_target(dst, F0);
            self.asm.it(cc, &[true, false]);
            self.asm.vmov(d, da);
            self.asm.vmov(d, db);
            self.put_float(dst, d);
            return;
        }
        let pa = self.int_in(a, S1);
        let pb = self.int_in(b, S2);
        let d = self.int_target(dst, S0);
        self.asm.it(cc, &[true, true, false, false]);
        self.asm.mov(d.0, pa.0);
        self.asm.mov(d.1, pa.1);
        self.asm.mov(d.0, pb.0);
        self.asm.mov(d.1, pb.1);
        self.put_int(dst, d);
    }

    // ----- integer arithmetic ------------------------------------------------------------

    /// `dst = a op b` on 64-bit pairs; `a` and `b` may be the scratch pairs `S0`/`S1`.
    fn int_op(&mut self, op: IntOp, dst: VReg, a: Pair, b: Pair) {
        match op {
            IntOp::Add | IntOp::Sub | IntOp::And | IntOp::Or | IntOp::Xor => {
                let d = self.int_target(dst, S0);
                let (lo_op, hi_op, s) = match op {
                    IntOp::Add => (Dp::Add, Dp::Adc, true),
                    IntOp::Sub => (Dp::Sub, Dp::Sbc, true),
                    IntOp::And => (Dp::And, Dp::And, false),
                    IntOp::Or => (Dp::Orr, Dp::Orr, false),
                    _ => (Dp::Eor, Dp::Eor, false),
                };
                // The low word first: `d` may share registers with `a` or `b`, whose low word
                // is no longer needed once the high word reads only high words.
                self.asm.dp(lo_op, s, d.0, a.0, b.0);
                self.asm.dp(hi_op, false, d.1, a.1, b.1);
                self.put_int(dst, d);
            }
            IntOp::Mul => {
                // Low 64 bits of the product: lo*lo (64-bit) + cross terms into the high word.
                self.asm.umull(S2.0, S2.1, a.0, b.0);
                self.asm.mla(S2.1, a.0, b.1, S2.1);
                self.asm.mla(S2.1, a.1, b.0, S2.1);
                self.put_int(dst, S2);
            }
            IntOp::Shl => {
                let n = b.0;
                self.asm.shift_reg(Shift::Lsl, false, S2.1, a.1, n);
                self.asm.dp_imm(Dp::Rsb, false, T, n, 32);
                self.asm.shift_reg(Shift::Lsr, false, T, a.0, T);
                self.asm.dp(Dp::Orr, false, S2.1, S2.1, T);
                self.asm.dp_imm(Dp::Sub, false, T, n, 32);
                self.asm.shift_reg(Shift::Lsl, false, T, a.0, T);
                self.asm.dp(Dp::Orr, false, S2.1, S2.1, T);
                self.asm.shift_reg(Shift::Lsl, false, S2.0, a.0, n);
                self.put_int(dst, S2);
            }
            IntOp::Sar => {
                let n = b.0;
                self.asm.shift_reg(Shift::Lsr, false, S2.0, a.0, n);
                self.asm.dp_imm(Dp::Rsb, false, T, n, 32);
                self.asm.shift_reg(Shift::Lsl, false, T, a.1, T);
                self.asm.dp(Dp::Orr, false, S2.0, S2.0, T);
                self.asm.dp_imm(Dp::Sub, true, T, n, 32);
                self.asm.it(Cond::Pl, &[true]);
                self.asm.shift_reg(Shift::Asr, false, S2.0, a.1, T);
                self.asm.shift_reg(Shift::Asr, false, S2.1, a.1, n);
                self.put_int(dst, S2);
            }
            IntOp::SDiv | IntOp::SRem | IntOp::UDiv | IntOp::URem => {
                // The EABI helpers: (r0:r1) / (r2:r3) → quotient r0:r1, remainder r2:r3.
                // `b` is in `S1` or an allocated pair, never `S0`: moving `a` first is safe.
                self.mov_pair(S0, a);
                self.mov_pair(S1, b);
                let signed = matches!(op, IntOp::SDiv | IntOp::SRem);
                self.asm.bl_symbol(if signed {
                    "__aeabi_ldivmod"
                } else {
                    "__aeabi_uldivmod"
                });
                let result = if matches!(op, IntOp::SDiv | IntOp::UDiv) {
                    S0
                } else {
                    S1
                };
                self.put_int(dst, result);
            }
        }
    }

    /// `d = (double) (lo, hi)`, exactly rounded: the high word scaled by 2^32 plus the low word
    /// (unsigned) is one rounding of the exact sum.
    fn int_to_float(&mut self, d: u8, p: Pair) {
        self.asm.vmov_s_from_core(14, p.1);
        self.asm.vcvt_f64_from_32(C1, 14, true);
        self.fconst(C0, 0x41F0_0000_0000_0000); // 2^32
        self.asm.fop(Fop::Mul, C1, C1, C0);
        self.asm.vmov_s_from_core(12, p.0);
        self.asm.vcvt_f64_from_32(C0, 12, false);
        self.asm.fop(Fop::Add, d, C1, C0);
    }

    /// `(lo, hi) = x` for an exact integer `x` (|x| < 2^63): the high word is `floor(x / 2^32)`,
    /// the low word the (exact) rest.
    fn float_to_int(&mut self, p: Pair, x: u8) {
        self.fconst(C0, 0x3DF0_0000_0000_0000); // 2^-32
        self.asm.fop(Fop::Mul, C1, x, C0);
        self.asm.vrintm(C1, C1);
        self.fconst(C0, 0x41F0_0000_0000_0000); // 2^32
        self.asm.fop(Fop::Mul, C0, C1, C0);
        self.asm.fop(Fop::Sub, C0, x, C0);
        let (s_lo, s_hi) = (2 * C2, 2 * C2 + 1);
        self.asm.vcvt_32_from_f64(s_lo, C0, false);
        self.asm.vcvt_32_from_f64(s_hi, C1, true);
        self.asm.vmov_s_to_core(p.0, s_lo);
        self.asm.vmov_s_to_core(p.1, s_hi);
    }

    /// Load argument `v` of a call into core register `r` (its low word) or double `d`.
    fn call_arg(&mut self, v: VReg, int_reg: &mut u8, float_reg: &mut u8) {
        match self.func.class(v) {
            Class::Int => {
                let r = *int_reg;
                match self.loc(v) {
                    Loc::Int(k) => self.asm.mov(r, pair(k).0),
                    _ => {
                        let off = self.slot(v);
                        let (b, o) = self.at(SP, off, 4092);
                        self.asm.ldr(r, b, o);
                    }
                }
                *int_reg += 1;
            }
            Class::Float => {
                let d = *float_reg;
                match self.loc(v) {
                    Loc::Float(f) => self.asm.vmov(d, f),
                    _ => {
                        let off = self.slot(v);
                        self.vld(d, SP, off);
                    }
                }
                *float_reg += 1;
            }
        }
    }

    fn inst(&mut self, inst: &Inst) -> Result<(), String> {
        match inst {
            Inst::IConst { dst, value } => {
                let d = self.int_target(*dst, S0);
                self.mov_imm64(d, *value);
                self.put_int(*dst, d);
            }
            Inst::FConst { dst, value } => {
                let d = self.float_target(*dst, F0);
                self.fconst(d, value.to_bits());
                self.put_float(*dst, d);
            }
            Inst::Mov { dst, src } => match self.func.class(*dst) {
                Class::Int => {
                    let p = self.int_in(*src, S0);
                    self.put_int(*dst, p);
                }
                Class::Float => {
                    let d = self.float_in(*src, F0);
                    self.put_float(*dst, d);
                }
            },
            Inst::FloatBits { dst, src } => {
                let x = self.float_in(*src, F0);
                let d = self.int_target(*dst, S0);
                self.asm.vmov_to_core(d.0, d.1, x);
                self.put_int(*dst, d);
            }
            Inst::BitsFloat { dst, src } => {
                let p = self.int_in(*src, S0);
                let d = self.float_target(*dst, F0);
                self.asm.vmov_from_core(d, p.0, p.1);
                self.put_float(*dst, d);
            }
            Inst::Int { op, dst, a, b } => {
                let pa = self.int_in(*a, S0);
                let pb = self.int_in(*b, S1);
                self.int_op(*op, *dst, pa, pb);
            }
            Inst::IntImm { op, dst, a, imm } => {
                let pa = self.int_in(*a, S0);
                self.mov_imm64(S1, *imm as i64);
                self.int_op(*op, *dst, pa, S1);
            }
            Inst::ICmp { .. } | Inst::ICmpImm { .. } | Inst::TestImm { .. } | Inst::FCmp { .. } => {
                let cc = self.flags_for(inst)?;
                let dst = inst.defs()[0];
                let d = self.int_target(dst, S2);
                self.asm.mov_imm(d.1, 0);
                self.asm.it(cc, &[true, false]);
                self.asm.mov_imm(d.0, 1);
                self.asm.mov_imm(d.0, 0);
                self.put_int(dst, d);
            }
            Inst::Neg { dst, src } => {
                let p = self.int_in(*src, S0);
                let d = self.int_target(*dst, S0);
                self.asm.mov_imm(T, 0);
                self.asm.dp(Dp::Sub, true, d.0, T, p.0);
                self.asm.dp(Dp::Sbc, false, d.1, T, p.1);
                self.put_int(*dst, d);
            }
            Inst::Not { dst, src } => {
                let p = self.int_in(*src, S0);
                let d = self.int_target(*dst, S0);
                self.asm.mvn(d.0, p.0);
                self.asm.mvn(d.1, p.1);
                self.put_int(*dst, d);
            }
            Inst::Popcnt { .. } => {
                return Err("nguvu: Cortex-M haina popcnt (jenga bila kuitambua)".into())
            }
            Inst::MulOverflow { .. } => {
                return Err("nguvu: MulOverflow haiungwi mkono kwa Cortex-M".into())
            }
            Inst::Float { op, dst, a, b } => {
                let da = self.float_in(*a, F0);
                let db = self.float_in(*b, F1);
                let d = self.float_target(*dst, F0);
                let op = match op {
                    FloatOp::Add => Fop::Add,
                    FloatOp::Sub => Fop::Sub,
                    FloatOp::Mul => Fop::Mul,
                    FloatOp::Div => Fop::Div,
                };
                self.asm.fop(op, d, da, db);
                self.put_float(*dst, d);
            }
            Inst::Select { dst, cond, a, b } => {
                let cc = self.nonzero_flags(*cond);
                self.select_on(cc, *dst, *a, *b);
            }
            Inst::IntToFloat { dst, src } => {
                let p = self.int_in(*src, S0);
                let d = self.float_target(*dst, F0);
                self.int_to_float(d, p);
                self.put_float(*dst, d);
            }
            Inst::FloatToInt { dst, src } => {
                let x = self.float_in(*src, F0);
                let d = self.int_target(*dst, S0);
                self.float_to_int(d, x);
                self.put_int(*dst, d);
            }
            Inst::Load { dst, base, offset } => {
                let b = self.int_in(*base, S1).0;
                let off = u32::try_from(*offset).map_err(|_| "nguvu: anwani hasi")?;
                match self.func.class(*dst) {
                    Class::Int => {
                        let d = self.int_target(*dst, S0);
                        self.ld64(d.0, d.1, b, off);
                        self.put_int(*dst, d);
                    }
                    Class::Float => {
                        let d = self.float_target(*dst, F0);
                        self.vld(d, b, off);
                        self.put_float(*dst, d);
                    }
                }
            }
            Inst::Store { src, base, offset } => {
                let b = self.int_in(*base, S1).0;
                let off = u32::try_from(*offset).map_err(|_| "nguvu: anwani hasi")?;
                match self.func.class(*src) {
                    Class::Int => {
                        let p = self.int_in(*src, S0);
                        self.st64(p.0, p.1, b, off);
                    }
                    Class::Float => {
                        let d = self.float_in(*src, F0);
                        self.vst(d, b, off);
                    }
                }
            }
            Inst::LoadIndex {
                dst,
                base,
                index,
                kind,
            } => {
                let b = self.int_in(*base, S1).0;
                let i = self.int_in(*index, S2).0;
                let width = kind.width() as u8;
                match self.func.class(*dst) {
                    Class::Float => {
                        self.asm
                            .dp_shifted(Dp::Add, false, ADDR, b, i, Shift::Lsl, 3);
                        let d = self.float_target(*dst, F0);
                        self.asm.vldr(d, ADDR, 0);
                        self.put_float(*dst, d);
                    }
                    Class::Int if width == 8 => {
                        self.asm
                            .dp_shifted(Dp::Add, false, ADDR, b, i, Shift::Lsl, 3);
                        let d = self.int_target(*dst, S0);
                        self.asm.ldrd(d.0, d.1, ADDR, 0);
                        self.put_int(*dst, d);
                    }
                    Class::Int => {
                        let d = self.int_target(*dst, S0);
                        let shift = width.trailing_zeros() as u8;
                        self.asm.ldr_idx(d.0, b, i, width, kind.signed(), shift);
                        if kind.signed() {
                            self.asm.shift_imm(Shift::Asr, d.1, d.0, 31);
                        } else {
                            self.asm.mov_imm(d.1, 0);
                        }
                        self.put_int(*dst, d);
                    }
                }
            }
            Inst::StoreIndex {
                src,
                base,
                index,
                kind,
            } => {
                let b = self.int_in(*base, S1).0;
                let i = self.int_in(*index, S2).0;
                let width = kind.width() as u8;
                match self.func.class(*src) {
                    Class::Float => {
                        let d = self.float_in(*src, F0);
                        self.asm
                            .dp_shifted(Dp::Add, false, ADDR, b, i, Shift::Lsl, 3);
                        self.asm.vstr(d, ADDR, 0);
                    }
                    Class::Int if width == 8 => {
                        let p = self.int_in(*src, S0);
                        self.asm
                            .dp_shifted(Dp::Add, false, ADDR, b, i, Shift::Lsl, 3);
                        self.asm.strd(p.0, p.1, ADDR, 0);
                    }
                    Class::Int => {
                        let p = self.int_in(*src, S0);
                        let shift = width.trailing_zeros() as u8;
                        self.asm.str_idx(p.0, b, i, width, shift);
                    }
                }
            }
            Inst::StackPointer { dst } => {
                let d = self.int_target(*dst, S0);
                self.asm.addw(d.0, SP, 0);
                self.asm.mov_imm(d.1, 0);
                self.put_int(*dst, d);
            }
            Inst::CallBuffer { dst } => {
                let d = self.int_target(*dst, S0);
                self.asm.add_any(d.0, SP, 8 * self.alloc.slots);
                self.asm.mov_imm(d.1, 0);
                self.put_int(*dst, d);
            }
            Inst::CallDirect {
                func,
                args,
                dst,
                result,
            } => {
                let (mut ni, mut nf) = (0u8, 0u8);
                for a in args {
                    self.call_arg(*a, &mut ni, &mut nf);
                }
                let at = self.asm.bl_placeholder();
                self.calls.push((at, *func));
                self.put_int(*dst, S0);
                self.put_float(*result, 0);
            }
            Inst::Call {
                target, args, dst, ..
            } => {
                let symbol = runtime_symbol(*target).ok_or_else(|| {
                    format!("inahitaji huduma ya mwenyeji ({target:?}) ambayo kifaa hakina")
                })?;
                if matches!(target, RtFn::Floor | RtFn::Ceil) {
                    let x = self.float_in(args[0], F0);
                    let Some(d) = dst else {
                        return Ok(());
                    };
                    let r = self.float_target(*d, F1);
                    if *target == RtFn::Floor {
                        self.asm.vrintm(r, x);
                    } else {
                        self.asm.vrintp(r, x);
                    }
                    self.put_float(*d, r);
                    return Ok(());
                }
                let (mut ni, mut nf) = (0u8, 0u8);
                for a in args {
                    self.call_arg(*a, &mut ni, &mut nf);
                }
                self.asm.bl_symbol(symbol);
                if let Some(d) = dst {
                    match self.func.class(*d) {
                        Class::Int => self.put_int(*d, S0),
                        Class::Float => self.put_float(*d, 0),
                    }
                }
            }
        }
        Ok(())
    }
}
