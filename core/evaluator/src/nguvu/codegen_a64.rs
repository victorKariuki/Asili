//! AArch64 code generation (AAPCS64) from allocated IR, through the [`a64`] encoder.
//!
//! The same IR, allocation and block schedule as the x86-64 generator; only instruction
//! selection differs. AArch64 is three-address with 31 general registers, so values rarely
//! need shuffling: `x9`–`x11` are scratch for operands, `x16` holds the runtime table and call
//! target, `x17` forms large addresses, and `d16`–`d18` are float scratch. Every other register
//! except `x18` (platform), `x29`/`x30` (frame/link) and `sp` is allocatable.
//!
//! Frame: `stp x29, x30, [sp, #-16]!; mov x29, sp; sub sp, sp, #frame`, then virtual register
//! `v`'s home slot at `[sp + 8v]` and the callee-saved registers the allocation uses above the
//! slots.

use super::a64::{Alu, Asm, Cond, Fop, Label, SP, ZR};
use super::ir::{Class, FCond, FloatOp, Func, Home, ICond, Inst, IntOp, Term, VReg};
use super::regalloc::{allocate, Allocation, Loc, Target};
use super::schedule::Step;

pub const TARGET: Target = Target {
    int_callee_saved: &[19, 20, 21, 22, 23, 24, 25, 26, 27, 28],
    int_caller_saved: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 13, 14, 15],
    float_callee_saved: &[8, 9, 10, 11, 12, 13, 14, 15],
    float_caller_saved: &[
        0, 1, 2, 3, 4, 5, 6, 7, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    ],
};

/// Operand scratch registers.
const S0: u8 = 9;
const S1: u8 = 10;
const S2: u8 = 11;
/// Runtime table / call target.
const CALL: u8 = 16;
/// Large-offset address formation.
const ADDR: u8 = 17;
const F0: u8 = 16;
const F1: u8 = 17;

fn icond(c: ICond) -> Cond {
    match c {
        ICond::Eq => Cond::Eq,
        ICond::Ne => Cond::Ne,
        ICond::Lt => Cond::Lt,
        ICond::Le => Cond::Le,
        ICond::Gt => Cond::Gt,
        ICond::Ge => Cond::Ge,
        ICond::Ult => Cond::Lo,
        ICond::Ule => Cond::Ls,
    }
}

/// After `fcmp`, an unordered result sets `C` and `V`: these conditions are false for NaN
/// operands except `ne`, exactly the IR's semantics.
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

struct Gen<'f> {
    asm: Asm,
    func: &'f Func,
    alloc: Allocation,
    /// Callee-saved registers this function uses, saved in the frame: (register, is_float).
    saved: Vec<(u8, bool)>,
    frame: u32,
    /// Direct calls to link: (`bl` offset, callee).
    calls: Vec<(usize, u32)>,
    /// What each register's home slot holds at a call (see `Func::homes`).
    homes: Vec<Home>,
}

/// Machine code for `func`, position independent (runtime calls go through the `rt` table).
pub fn generate(func: &Func) -> Result<super::Code, String> {
    let alloc = allocate(func, &TARGET);
    let mut saved: Vec<(u8, bool)> = Vec::new();
    for loc in &alloc.loc {
        let entry = match *loc {
            Loc::Int(r) if TARGET.int_callee_saved.contains(&r) => (r, false),
            Loc::Float(r) if TARGET.float_callee_saved.contains(&r) => (r, true),
            _ => continue,
        };
        if !saved.contains(&entry) {
            saved.push(entry);
        }
    }
    saved.sort_unstable();
    let slots = alloc.slots;
    let frame = (8 * (slots + saved.len() as u32) + func.call_buffer).div_ceil(16) * 16;
    let mut g = Gen {
        asm: Asm::new(),
        func,
        alloc,
        saved,
        frame,
        calls: Vec::new(),
        homes: func.homes(),
    };
    g.prologue();
    let labels: Vec<Label> = (0..func.blocks.len()).map(|_| g.asm.new_label()).collect();
    let order = g.alloc.order.clone();
    for (k, &bi) in order.iter().enumerate() {
        let block = &func.blocks[bi];
        g.asm.bind(labels[bi]);
        let next = order.get(k + 1).map(|&b| labels[b]);
        let plan = super::schedule::plan(block, &g.alloc.uses);
        for step in plan.steps {
            match step {
                Step::Inst(i) => g.inst(bi, i, &block.insts[i]),
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
                    g.asm.b(labels[t.0 as usize]);
                }
            }
            Term::Branch { cond, then_, else_ } => {
                let (t, e) = (labels[then_.0 as usize], labels[else_.0 as usize]);
                if plan.fused_branch {
                    let cc = g.flags_for(block.insts.last().expect("compare"));
                    g.branch_on(cc, t, e, next);
                } else {
                    let r = g.int_in(*cond, S0);
                    g.asm.cmp_imm(r, 0);
                    g.branch_on(Cond::Ne, t, e, next);
                }
            }
            Term::Return(v) => {
                let r = g.int_in(*v, S0);
                g.asm.mov(0, r);
                g.epilogue();
            }
        }
    }
    Ok(super::Code {
        bytes: g.asm.finish()?,
        calls: g.calls,
    })
}

impl<'f> Gen<'f> {
    fn loc(&self, v: VReg) -> Loc {
        self.alloc.loc[v.0 as usize]
    }

    fn slot(&self, v: VReg) -> u32 {
        let i = self.alloc.slot[v.0 as usize];
        debug_assert!(i != u32::MAX, "{v:?} has no home slot");
        8 * i
    }

    /// `[base + off]` as a base register and a scaled-immediate offset.
    fn addr(&mut self, base: u8, off: u32) -> (u8, u32) {
        if off / 8 < 4096 {
            (base, off)
        } else {
            self.asm.add_imm(ADDR, base, off >> 12, true);
            (ADDR, off & 0xFFF)
        }
    }

    fn ldr_at(&mut self, rt: u8, base: u8, off: u32) {
        let (b, o) = self.addr(base, off);
        self.asm.ldr(rt, b, o);
    }

    fn str_at(&mut self, rt: u8, base: u8, off: u32) {
        let (b, o) = self.addr(base, off);
        self.asm.str(rt, b, o);
    }

    fn ldr_d_at(&mut self, dt: u8, base: u8, off: u32) {
        let (b, o) = self.addr(base, off);
        self.asm.ldr_d(dt, b, o);
    }

    fn str_d_at(&mut self, dt: u8, base: u8, off: u32) {
        let (b, o) = self.addr(base, off);
        self.asm.str_d(dt, b, o);
    }

    fn adjust_sp(&mut self, grow: bool) {
        let (hi, lo) = (self.frame >> 12, self.frame & 0xFFF);
        for (imm, shift) in [(hi, true), (lo, false)] {
            if imm != 0 {
                if grow {
                    self.asm.sub_imm(SP, SP, imm, shift);
                } else {
                    self.asm.add_imm(SP, SP, imm, shift);
                }
            }
        }
    }

    /// Offset from `sp` of the direct-call register buffer, above the saved registers.
    fn call_buffer_offset(&self) -> u32 {
        8 * (self.alloc.slots + self.saved.len() as u32)
    }

    fn saved_offset(&self, i: usize) -> u32 {
        8 * (self.alloc.slots + i as u32)
    }

    fn prologue(&mut self) {
        self.asm.push_frame_record();
        self.asm.add_imm(29, SP, 0, false);
        self.adjust_sp(true);
        for (i, (r, float)) in self.saved.clone().into_iter().enumerate() {
            let off = self.saved_offset(i);
            if float {
                self.str_d_at(r, SP, off);
            } else {
                self.str_at(r, SP, off);
            }
        }
        // Incoming `rt, vm, frame, nums` (x0–x3): park them in their home slots first (their
        // allocated registers may be other argument registers), then load each where it lives.
        for i in 0..4u32 {
            self.str_at(i as u8, SP, self.slot(VReg(i)));
        }
        for i in 0..4u32 {
            let v = VReg(i);
            if let Loc::Int(r) = self.loc(v) {
                self.ldr_at(r, SP, self.slot(v));
            }
        }
    }

    fn epilogue(&mut self) {
        for (i, (r, float)) in self.saved.clone().into_iter().enumerate() {
            let off = self.saved_offset(i);
            if float {
                self.ldr_d_at(r, SP, off);
            } else {
                self.ldr_at(r, SP, off);
            }
        }
        self.adjust_sp(false);
        self.asm.pop_frame_record();
        self.asm.ret();
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

    // ----- operands ----------------------------------------------------------------------

    fn int_in(&mut self, v: VReg, scratch: u8) -> u8 {
        match self.loc(v) {
            Loc::Int(r) => r,
            Loc::Slot => {
                self.ldr_at(scratch, SP, self.slot(v));
                scratch
            }
            Loc::Float(_) => unreachable!("integer value in a float register"),
        }
    }

    fn float_in(&mut self, v: VReg, scratch: u8) -> u8 {
        match self.loc(v) {
            Loc::Float(d) => d,
            Loc::Slot => {
                self.ldr_d_at(scratch, SP, self.slot(v));
                scratch
            }
            Loc::Int(_) => unreachable!("float value in a general register"),
        }
    }

    fn int_target(&self, v: VReg, scratch: u8) -> u8 {
        match self.loc(v) {
            Loc::Int(r) => r,
            _ => scratch,
        }
    }

    fn float_target(&self, v: VReg, scratch: u8) -> u8 {
        match self.loc(v) {
            Loc::Float(d) => d,
            _ => scratch,
        }
    }

    fn put_int(&mut self, v: VReg, src: u8) {
        match self.loc(v) {
            Loc::Int(r) => self.asm.mov(r, src),
            Loc::Float(d) => self.asm.fmov_from_x(d, src),
            Loc::Slot => self.str_at(src, SP, self.slot(v)),
        }
    }

    fn put_float(&mut self, v: VReg, src: u8) {
        match self.loc(v) {
            Loc::Float(d) => self.asm.fmov(d, src),
            Loc::Int(r) => self.asm.fmov_to_x(r, src),
            Loc::Slot => self.str_d_at(src, SP, self.slot(v)),
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
            Loc::Int(r) => self.str_at(r, SP, self.slot(v)),
            Loc::Float(d) => self.str_d_at(d, SP, self.slot(v)),
            Loc::Slot => {}
        }
    }

    fn restore(&mut self, v: VReg) {
        if let Home::Const(bits) = self.homes[v.0 as usize] {
            match self.loc(v) {
                Loc::Int(r) => self.asm.mov_imm(r, bits),
                Loc::Float(d) => {
                    self.asm.mov_imm(S0, bits);
                    self.asm.fmov_from_x(d, S0);
                }
                Loc::Slot => {}
            }
            return;
        }
        match self.loc(v) {
            Loc::Int(r) => self.ldr_at(r, SP, self.slot(v)),
            Loc::Float(d) => self.ldr_d_at(d, SP, self.slot(v)),
            Loc::Slot => {}
        }
    }

    /// `cmp r, #imm` for any 32-bit immediate.
    fn cmp_with(&mut self, r: u8, imm: i32) {
        if (0..4096).contains(&imm) {
            self.asm.cmp_imm(r, imm as u32);
        } else if (-4095..0).contains(&imm) {
            self.asm.cmn_imm(r, (-imm) as u32);
        } else {
            self.asm.mov_imm(S1, imm as i64);
            self.asm.alu(Alu::Subs, ZR, r, S1);
        }
    }

    /// Set the flags for a comparison and return the condition meaning "true".
    fn flags_for(&mut self, cmp: &Inst) -> Cond {
        match cmp {
            Inst::ICmp { cond, a, b, .. } => {
                let ra = self.int_in(*a, S0);
                let rb = self.int_in(*b, S1);
                self.asm.alu(Alu::Subs, ZR, ra, rb);
                icond(*cond)
            }
            Inst::ICmpImm { cond, a, imm, .. } => {
                let ra = self.int_in(*a, S0);
                self.cmp_with(ra, *imm);
                icond(*cond)
            }
            Inst::TestImm { zero, a, imm, .. } => {
                let ra = self.int_in(*a, S0);
                self.asm.mov_imm(S1, *imm as i64);
                self.asm.alu(Alu::Ands, ZR, ra, S1);
                if *zero {
                    Cond::Eq
                } else {
                    Cond::Ne
                }
            }
            Inst::FCmp { cond, a, b, .. } => {
                let da = self.float_in(*a, F0);
                let db = self.float_in(*b, F1);
                self.asm.fcmp(da, db);
                fcond(*cond)
            }
            _ => unreachable!("flags for a non-comparison"),
        }
    }

    /// `dst = cc ? a : b` with the flags already set.
    fn select_on(&mut self, cc: Cond, dst: VReg, a: VReg, b: VReg) {
        let ra = self.int_in(a, S1);
        let rb = self.int_in(b, S2);
        let d = self.int_target(dst, S0);
        self.asm.csel(d, ra, rb, cc);
        self.put_int(dst, d);
    }

    fn int_op(&mut self, op: IntOp, d: u8, ra: u8, rb: u8) {
        match op {
            IntOp::Add => self.asm.alu(Alu::Add, d, ra, rb),
            IntOp::Sub => self.asm.alu(Alu::Sub, d, ra, rb),
            IntOp::And => self.asm.alu(Alu::And, d, ra, rb),
            IntOp::Or => self.asm.alu(Alu::Orr, d, ra, rb),
            IntOp::Xor => self.asm.alu(Alu::Eor, d, ra, rb),
            IntOp::Mul => self.asm.alu(Alu::Mul, d, ra, rb),
            IntOp::Shl => self.asm.alu(Alu::Lslv, d, ra, rb),
            IntOp::Sar => self.asm.alu(Alu::Asrv, d, ra, rb),
            IntOp::SDiv => self.asm.alu(Alu::Sdiv, d, ra, rb),
            IntOp::UDiv => self.asm.alu(Alu::Udiv, d, ra, rb),
            IntOp::SRem | IntOp::URem => {
                let div = if op == IntOp::SRem {
                    Alu::Sdiv
                } else {
                    Alu::Udiv
                };
                self.asm.alu(div, S2, ra, rb);
                self.asm.msub(d, S2, rb, ra); // a - (a / b) * b
            }
        }
    }

    /// Before a call: caller-saved registers holding values needed after it, and every
    /// register-resident argument, go to their home slots. Returns the values to restore.
    fn save_for_call(&mut self, bi: usize, ii: usize, args: &[VReg]) -> Vec<VReg> {
        let across: Vec<VReg> = self
            .alloc
            .live_across
            .get(&(bi, ii))
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|v| TARGET.is_caller_saved(self.loc(*v)))
            .collect();
        for v in &across {
            self.save(*v);
        }
        for a in args {
            if !across.contains(a) {
                self.save(*a);
            }
        }
        across
    }

    /// Load call arguments from their home slots into `x0..`/`d0..` (leaves `x16` alone).
    fn load_call_args(&mut self, args: &[VReg]) {
        let (mut ni, mut nf) = (0u8, 0u8);
        for a in args {
            let constant = match self.homes[a.0 as usize] {
                Home::Const(bits) if self.loc(*a) != Loc::Slot => Some(bits),
                _ => None,
            };
            match (self.func.class(*a), constant) {
                (Class::Int, Some(bits)) => self.asm.mov_imm(ni, bits),
                (Class::Int, None) => self.ldr_at(ni, SP, self.slot(*a)),
                (Class::Float, Some(bits)) => {
                    self.asm.mov_imm(S0, bits);
                    self.asm.fmov_from_x(nf, S0);
                }
                (Class::Float, None) => self.ldr_d_at(nf, SP, self.slot(*a)),
            }
            match self.func.class(*a) {
                Class::Int => ni += 1,
                Class::Float => nf += 1,
            }
        }
    }

    fn inst(&mut self, bi: usize, ii: usize, inst: &Inst) {
        match inst {
            Inst::IConst { dst, value } => {
                let d = self.int_target(*dst, S0);
                self.asm.mov_imm(d, *value);
                self.put_int(*dst, d);
            }
            Inst::FConst { dst, value } => {
                self.asm.mov_imm(S0, value.to_bits() as i64);
                let d = self.float_target(*dst, F0);
                self.asm.fmov_from_x(d, S0);
                self.put_float(*dst, d);
            }
            Inst::Mov { dst, src } => match self.func.class(*dst) {
                Class::Int => {
                    let r = self.int_in(*src, S0);
                    self.put_int(*dst, r);
                }
                Class::Float => {
                    let d = self.float_in(*src, F0);
                    self.put_float(*dst, d);
                }
            },
            Inst::FloatBits { dst, src } => {
                let x = self.float_in(*src, F0);
                let d = self.int_target(*dst, S0);
                self.asm.fmov_to_x(d, x);
                self.put_int(*dst, d);
            }
            Inst::BitsFloat { dst, src } => {
                let r = self.int_in(*src, S0);
                let d = self.float_target(*dst, F0);
                self.asm.fmov_from_x(d, r);
                self.put_float(*dst, d);
            }
            Inst::Int { op, dst, a, b } => {
                let ra = self.int_in(*a, S0);
                let rb = self.int_in(*b, S1);
                let d = self.int_target(*dst, S0);
                self.int_op(*op, d, ra, rb);
                self.put_int(*dst, d);
            }
            Inst::IntImm { op, dst, a, imm } => {
                let ra = self.int_in(*a, S0);
                let d = self.int_target(*dst, S0);
                let imm = *imm;
                match op {
                    IntOp::Add | IntOp::Sub if (-4095..4096).contains(&imm) => {
                        let add = (*op == IntOp::Add) == (imm >= 0);
                        let n = imm.unsigned_abs();
                        if add {
                            self.asm.add_imm(d, ra, n, false);
                        } else {
                            self.asm.sub_imm(d, ra, n, false);
                        }
                    }
                    IntOp::Shl => self.asm.lsl_imm(d, ra, imm as u8),
                    IntOp::Sar => self.asm.asr_imm(d, ra, imm as u8),
                    // Unsigned division by a constant: a multiply by its reciprocal.
                    IntOp::UDiv | IntOp::URem if imm > 1 => {
                        let (magic, shift) = super::ir::div_magic(imm as u32);
                        self.asm.mov_imm(S1, magic as i64);
                        self.asm.alu(Alu::Umulh, S2, ra, S1);
                        if shift > 0 {
                            self.asm.lsr_imm(S2, S2, shift);
                        }
                        if *op == IntOp::UDiv {
                            self.asm.mov(d, S2);
                        } else {
                            self.asm.mov_imm(S1, imm as i64);
                            self.asm.msub(d, S2, S1, ra); // a - (a / imm) * imm
                        }
                    }
                    _ => {
                        self.asm.mov_imm(S1, imm as i64);
                        self.int_op(*op, d, ra, S1);
                    }
                }
                self.put_int(*dst, d);
            }
            Inst::ICmp { .. } | Inst::ICmpImm { .. } | Inst::TestImm { .. } | Inst::FCmp { .. } => {
                let cc = self.flags_for(inst);
                let dst = inst.defs()[0];
                let d = self.int_target(dst, S0);
                self.asm.cset(d, cc);
                self.put_int(dst, d);
            }
            Inst::Popcnt { dst, src } => {
                let r = self.int_in(*src, S0);
                self.asm.fmov_from_x(F0, r);
                self.asm.cnt8b(F0, F0);
                self.asm.addv8b(F0, F0);
                let d = self.int_target(*dst, S0);
                self.asm.fmov_to_x(d, F0);
                self.put_int(*dst, d);
            }
            Inst::Neg { dst, src } => {
                let r = self.int_in(*src, S0);
                let d = self.int_target(*dst, S0);
                self.asm.alu(Alu::Sub, d, ZR, r);
                self.put_int(*dst, d);
            }
            Inst::Not { dst, src } => {
                let r = self.int_in(*src, S0);
                let d = self.int_target(*dst, S0);
                self.asm.mvn(d, r);
                self.put_int(*dst, d);
            }
            Inst::MulOverflow { dst, ovf, a, b } => {
                let ra = self.int_in(*a, S0);
                let rb = self.int_in(*b, S1);
                self.asm.alu(Alu::Mul, S2, ra, rb);
                self.asm.alu(Alu::Smulh, S1, ra, rb);
                // Overflowed iff the high half is not the sign extension of the low half.
                self.asm.cmp_asr(S1, S2, 63);
                self.put_int(*dst, S2);
                let o = self.int_target(*ovf, S0);
                self.asm.cset(o, Cond::Ne);
                self.put_int(*ovf, o);
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
                let rc = self.int_in(*cond, S0);
                self.asm.cmp_imm(rc, 0);
                self.select_on(Cond::Ne, *dst, *a, *b);
            }
            Inst::IntToFloat { dst, src } => {
                let r = self.int_in(*src, S0);
                let d = self.float_target(*dst, F0);
                self.asm.scvtf(d, r);
                self.put_float(*dst, d);
            }
            Inst::FloatToInt { dst, src } => {
                let x = self.float_in(*src, F0);
                let d = self.int_target(*dst, S0);
                self.asm.fcvtzs(d, x);
                self.put_int(*dst, d);
            }
            Inst::Load { dst, base, offset } => {
                let rb = self.int_in(*base, S1);
                let off = u32::try_from(*offset).expect("non-negative load offset");
                match self.func.class(*dst) {
                    Class::Int => {
                        let d = self.int_target(*dst, S0);
                        self.ldr_at(d, rb, off);
                        self.put_int(*dst, d);
                    }
                    Class::Float => {
                        let d = self.float_target(*dst, F0);
                        self.ldr_d_at(d, rb, off);
                        self.put_float(*dst, d);
                    }
                }
            }
            Inst::Store { src, base, offset } => {
                let rb = self.int_in(*base, S1);
                let off = u32::try_from(*offset).expect("non-negative store offset");
                match self.func.class(*src) {
                    Class::Int => {
                        let r = self.int_in(*src, S0);
                        self.str_at(r, rb, off);
                    }
                    Class::Float => {
                        let d = self.float_in(*src, F0);
                        self.str_d_at(d, rb, off);
                    }
                }
            }
            Inst::LoadIndex { dst, base, index } => {
                let rb = self.int_in(*base, S1);
                let ri = self.int_in(*index, S2);
                match self.func.class(*dst) {
                    Class::Int => {
                        let d = self.int_target(*dst, S0);
                        self.asm.ldr_idx(d, rb, ri);
                        self.put_int(*dst, d);
                    }
                    Class::Float => {
                        let d = self.float_target(*dst, F0);
                        self.asm.ldr_d_idx(d, rb, ri);
                        self.put_float(*dst, d);
                    }
                }
            }
            Inst::StoreIndex { src, base, index } => {
                let rb = self.int_in(*base, S1);
                let ri = self.int_in(*index, S2);
                match self.func.class(*src) {
                    Class::Int => {
                        let r = self.int_in(*src, S0);
                        self.asm.str_idx(r, rb, ri);
                    }
                    Class::Float => {
                        let d = self.float_in(*src, F0);
                        self.asm.str_d_idx(d, rb, ri);
                    }
                }
            }
            Inst::StackPointer { dst } => {
                let d = self.int_target(*dst, S0);
                self.asm.add_imm(d, SP, 0, false);
                self.put_int(*dst, d);
            }
            Inst::CallBuffer { dst } => {
                let d = self.int_target(*dst, S0);
                let off = self.call_buffer_offset();
                self.asm.add_imm(d, SP, off & 0xFFF, false);
                if off >> 12 != 0 {
                    self.asm.add_imm(d, d, off >> 12, true);
                }
                self.put_int(*dst, d);
            }
            Inst::CallDirect { func, args, dst } => {
                let across = self.save_for_call(bi, ii, args);
                self.load_call_args(args);
                let at = self.asm.bl_placeholder();
                self.calls.push((at, *func));
                self.put_int(*dst, 0);
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
                // The runtime table pointer comes from its home slot (the prologue stores it
                // there): runtime calls read it implicitly, so its register need not be live.
                self.ldr_at(CALL, SP, self.slot(super::ir::RT));
                self.load_call_args(args);
                self.asm.ldr(CALL, CALL, 8 * (*target as u32));
                self.asm.blr(CALL);
                if let Some(d) = dst {
                    match self.func.class(*d) {
                        Class::Int => {
                            if *ret32 {
                                self.asm.mov32(0, 0);
                            }
                            self.put_int(*d, 0);
                        }
                        Class::Float => self.put_float(*d, 0),
                    }
                }
                for v in &across {
                    self.restore(*v);
                }
            }
        }
    }
}
