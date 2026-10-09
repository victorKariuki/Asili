//! Thumb-2 (T32) and FPv5 double-precision instruction encoder for ARMv7-M (Cortex-M4F/M7
//! with the `fpv5-d16` unit): the subset `codegen_t32` uses, with labels for branches and
//! relocations for calls to symbols the linker resolves. A 32-bit instruction is two
//! little-endian halfwords, the first holding the opcode's high bits. Core registers are
//! numbered 0–15 (13 = `sp`, 14 = `lr`), double registers 0–15. Every encoding here is checked
//! against `llvm-mc` (`tests::encodings_match_llvm`).

/// Condition codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    Eq = 0,
    Ne = 1,
    /// Unsigned `>=` (carry set).
    Hs = 2,
    /// Unsigned `<`.
    Lo = 3,
    Mi = 4,
    Pl = 5,
    Vs = 6,
    Vc = 7,
    Hi = 8,
    Ls = 9,
    Ge = 10,
    Lt = 11,
    Gt = 12,
    Le = 13,
}

impl Cond {
    pub fn invert(self) -> Cond {
        use Cond::*;
        match self {
            Eq => Ne,
            Ne => Eq,
            Hs => Lo,
            Lo => Hs,
            Mi => Pl,
            Pl => Mi,
            Vs => Vc,
            Vc => Vs,
            Hi => Ls,
            Ls => Hi,
            Ge => Lt,
            Lt => Ge,
            Gt => Le,
            Le => Gt,
        }
    }
}

pub const SP: u8 = 13;
pub const LR: u8 = 14;
const PC: u8 = 15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Label(u32);

/// Data-processing operations (register or shifted-register form).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dp {
    And = 0,
    Bic = 1,
    Orr = 2,
    Orn = 3,
    Eor = 4,
    Add = 8,
    Adc = 10,
    Sbc = 11,
    Sub = 13,
    Rsb = 14,
}

/// Shift types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shift {
    Lsl = 0,
    Lsr = 1,
    Asr = 2,
}

/// Double-precision arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fop {
    Add,
    Sub,
    Mul,
    Div,
}

/// How a relocation patches the code (ELF `R_ARM_*` types).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reloc {
    /// `bl` to a symbol (`R_ARM_THM_CALL`).
    Call = 10,
    /// `movw` / `movt` of a symbol's address (`R_ARM_THM_MOVW_ABS_NC` / `R_ARM_THM_MOVT_ABS`).
    MovwAbs = 47,
    MovtAbs = 48,
}

#[derive(Clone, Copy)]
enum Fix {
    /// `b.w` (±16 MiB).
    B,
    /// `b<cond>.w` (±1 MiB).
    BCond,
}

/// Relocations: `(offset, symbol, kind)`.
pub type Relocs = Vec<(usize, String, Reloc)>;

pub struct Asm {
    code: Vec<u8>,
    labels: Vec<Option<usize>>,
    fixups: Vec<(usize, Label, Fix)>,
    /// To be resolved by the linker.
    pub relocs: Relocs,
}

impl Default for Asm {
    fn default() -> Self {
        Self::new()
    }
}

fn r(n: u8) -> u16 {
    debug_assert!(n < 16);
    n as u16
}

fn d(n: u8) -> u16 {
    debug_assert!(n < 16, "fpv5-d16 has d0-d15");
    n as u16
}

/// `imm32`'s Thumb modified-immediate encoding (`i:imm3:imm8`), if it has one.
fn modified_imm(v: u32) -> Option<u16> {
    let b = v & 0xFF;
    if v < 0x100 {
        return Some(v as u16);
    }
    if v == b << 16 | b {
        return Some(0x100 | b as u16);
    }
    let b1 = (v >> 8) & 0xFF;
    if v == b1 << 24 | b1 << 8 {
        return Some(0x200 | b1 as u16);
    }
    if v == b * 0x0101_0101 {
        return Some(0x300 | b as u16);
    }
    // An 8-bit value with its top bit set, rotated right by 8..=31.
    for rot in 8..32 {
        let unrot = v.rotate_left(rot);
        if unrot < 0x100 && unrot & 0x80 != 0 {
            let enc = (rot << 7) | (unrot & 0x7F);
            return Some(enc as u16);
        }
    }
    None
}

impl Asm {
    pub fn new() -> Self {
        Asm {
            code: Vec::new(),
            labels: Vec::new(),
            fixups: Vec::new(),
            relocs: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    fn h(&mut self, hw: u16) {
        self.code.extend_from_slice(&hw.to_le_bytes());
    }

    fn w(&mut self, hw1: u16, hw2: u16) {
        self.h(hw1);
        self.h(hw2);
    }

    pub fn new_label(&mut self) -> Label {
        self.labels.push(None);
        Label(self.labels.len() as u32 - 1)
    }

    pub fn bind(&mut self, l: Label) {
        self.labels[l.0 as usize] = Some(self.code.len());
    }

    /// Pad with `nop`s up to a multiple of `n` bytes (a power of two, at least 2).
    pub fn align(&mut self, n: usize) {
        while !self.code.len().is_multiple_of(n) {
            if self.code.len() % 4 == 2 || n == 2 {
                self.h(0xBF00);
            } else {
                self.w(0xF3AF, 0x8000);
            }
        }
    }

    /// Resolve branches; fails if a branch cannot reach its target.
    pub fn finish(mut self) -> Result<(Vec<u8>, Relocs), String> {
        for &(at, l, kind) in &self.fixups {
            let target = self.labels[l.0 as usize].ok_or("nguvu: lebo haijawekwa")?;
            let delta = target as i64 - (at as i64 + 4);
            let (hw1, hw2) = match kind {
                Fix::B => {
                    if !(-(1 << 24)..(1 << 24)).contains(&delta) {
                        return Err("nguvu: kazi ni kubwa mno kwa tawi la Thumb".into());
                    }
                    branch_bits(delta, true)
                }
                Fix::BCond => {
                    if !(-(1 << 20)..(1 << 20)).contains(&delta) {
                        return Err("nguvu: kazi ni kubwa mno kwa tawi la Thumb".into());
                    }
                    let v = delta as u32;
                    let s = (v >> 20) & 1;
                    let j2 = (v >> 19) & 1;
                    let j1 = (v >> 18) & 1;
                    let imm6 = (v >> 12) & 0x3F;
                    let imm11 = (v >> 1) & 0x7FF;
                    (
                        (s << 10 | imm6) as u16,
                        (j1 << 13 | j2 << 11 | imm11) as u16,
                    )
                }
            };
            let old1 = u16::from_le_bytes([self.code[at], self.code[at + 1]]);
            let old2 = u16::from_le_bytes([self.code[at + 2], self.code[at + 3]]);
            self.code[at..at + 2].copy_from_slice(&(old1 | hw1).to_le_bytes());
            self.code[at + 2..at + 4].copy_from_slice(&(old2 | hw2).to_le_bytes());
        }
        Ok((self.code, self.relocs))
    }

    // ----- control flow ------------------------------------------------------------------

    pub fn b(&mut self, l: Label) {
        self.fixups.push((self.code.len(), l, Fix::B));
        self.w(0xF000, 0x9000);
    }

    pub fn b_cond(&mut self, c: Cond, l: Label) {
        self.fixups.push((self.code.len(), l, Fix::BCond));
        self.w(0xF000 | (c as u16) << 6, 0x8000);
    }

    /// `bl` with a zero offset, linked later (a call within the image); returns its offset.
    pub fn bl_placeholder(&mut self) -> usize {
        let at = self.code.len();
        self.w(0xF000, 0xD000);
        at
    }

    /// `bl symbol`, resolved by the linker (the REL addend `-4` is encoded in place).
    pub fn bl_symbol(&mut self, symbol: &str) {
        self.relocs
            .push((self.code.len(), symbol.to_string(), Reloc::Call));
        self.w(0xF7FF, 0xFFFE);
    }

    pub fn blx(&mut self, rm: u8) {
        self.h(0x4780 | r(rm) << 3);
    }

    pub fn bx_lr(&mut self) {
        self.h(0x4770);
    }

    /// `it<mask> cond`: `pattern` lists, for each following instruction, whether it runs on
    /// `c` (true) or on its inverse (false); the first is always `true`.
    pub fn it(&mut self, c: Cond, pattern: &[bool]) {
        debug_assert!(!pattern.is_empty() && pattern.len() <= 4 && pattern[0]);
        let low = c as u16 & 1;
        let mut mask: u16 = 0;
        for (i, &then) in pattern.iter().enumerate().skip(1) {
            let bit = if then { low } else { low ^ 1 };
            mask |= bit << (4 - i);
        }
        mask |= 1 << (4 - pattern.len());
        self.h(0xBF00 | (c as u16) << 4 | mask);
    }

    // ----- integer -----------------------------------------------------------------------

    /// `op{s}.w rd, rn, rm[, shift #amount]`.
    pub fn dp(&mut self, op: Dp, s: bool, rd: u8, rn: u8, rm: u8) {
        self.dp_shifted(op, s, rd, rn, rm, Shift::Lsl, 0);
    }

    #[allow(clippy::too_many_arguments)] // one per encoding field
    pub fn dp_shifted(&mut self, op: Dp, s: bool, rd: u8, rn: u8, rm: u8, sh: Shift, n: u8) {
        debug_assert!(n < 32);
        let n = n as u16;
        self.w(
            0xEA00 | (op as u16) << 5 | (s as u16) << 4 | r(rn),
            (n >> 2) << 12 | r(rd) << 8 | (n & 3) << 6 | (sh as u16) << 4 | r(rm),
        );
    }

    /// `op{s}.w rd, rn, #imm` for an immediate with a modified-immediate encoding.
    pub fn dp_imm(&mut self, op: Dp, s: bool, rd: u8, rn: u8, imm: u32) -> bool {
        let Some(e) = modified_imm(imm) else {
            return false;
        };
        self.w(
            0xF000 | (e >> 11) << 10 | (op as u16) << 5 | (s as u16) << 4 | r(rn),
            ((e >> 8) & 7) << 12 | r(rd) << 8 | (e & 0xFF),
        );
        true
    }

    pub fn mov(&mut self, rd: u8, rm: u8) {
        self.dp(Dp::Orr, false, rd, PC, rm);
    }

    pub fn mvn(&mut self, rd: u8, rm: u8) {
        self.dp(Dp::Orn, false, rd, PC, rm);
    }

    /// `rd = rm <shift> #n` (`n` in 1..=31).
    pub fn shift_imm(&mut self, sh: Shift, rd: u8, rm: u8, n: u8) {
        self.dp_shifted(Dp::Orr, false, rd, PC, rm, sh, n);
    }

    /// `rd = rn <shift> rm` (amount from `rm`'s low byte; 32 and up shifts everything out).
    pub fn shift_reg(&mut self, sh: Shift, s: bool, rd: u8, rn: u8, rm: u8) {
        self.w(
            0xFA00 | (sh as u16) << 5 | (s as u16) << 4 | r(rn),
            0xF000 | r(rd) << 8 | r(rm),
        );
    }

    pub fn cmp(&mut self, rn: u8, rm: u8) {
        self.dp(Dp::Sub, true, PC, rn, rm);
    }

    pub fn tst(&mut self, rn: u8, rm: u8) {
        self.dp(Dp::And, true, PC, rn, rm);
    }

    /// `cmp rn, #imm`, if `imm` has a modified-immediate encoding.
    pub fn cmp_imm(&mut self, rn: u8, imm: u32) -> bool {
        self.dp_imm(Dp::Sub, true, PC, rn, imm)
    }

    /// `movw rd, #lo16`.
    pub fn movw(&mut self, rd: u8, v: u16) {
        self.w(
            0xF240 | ((v >> 11) & 1) << 10 | (v >> 12),
            ((v >> 8) & 7) << 12 | r(rd) << 8 | (v & 0xFF),
        );
    }

    /// `movt rd, #hi16`.
    pub fn movt(&mut self, rd: u8, v: u16) {
        self.w(
            0xF2C0 | ((v >> 11) & 1) << 10 | (v >> 12),
            ((v >> 8) & 7) << 12 | r(rd) << 8 | (v & 0xFF),
        );
    }

    /// `rd = v`.
    pub fn mov_imm(&mut self, rd: u8, v: u32) {
        if let Some(e) = modified_imm(v) {
            // mov.w rd, #imm
            self.w(
                0xF04F | (e >> 11) << 10,
                ((e >> 8) & 7) << 12 | r(rd) << 8 | (e & 0xFF),
            );
        } else if let Some(e) = modified_imm(!v) {
            // mvn.w rd, #imm
            self.w(
                0xF06F | (e >> 11) << 10,
                ((e >> 8) & 7) << 12 | r(rd) << 8 | (e & 0xFF),
            );
        } else {
            self.movw(rd, v as u16);
            if v >> 16 != 0 {
                self.movt(rd, (v >> 16) as u16);
            }
        }
    }

    /// `movw`/`movt rd, symbol` (an absolute address the linker fills in).
    pub fn mov_symbol(&mut self, rd: u8, symbol: &str) {
        self.relocs
            .push((self.code.len(), symbol.to_string(), Reloc::MovwAbs));
        self.movw(rd, 0);
        self.relocs
            .push((self.code.len(), symbol.to_string(), Reloc::MovtAbs));
        self.movt(rd, 0);
    }

    /// `addw rd, rn, #imm12` / `subw`.
    pub fn addw(&mut self, rd: u8, rn: u8, imm: u16) {
        self.addsubw(0xF200, rd, rn, imm);
    }

    /// `rd = rn + imm` for any `imm` (`rd` must differ from `rn` when `imm` is 4096 or more).
    pub fn add_any(&mut self, rd: u8, rn: u8, imm: u32) {
        if imm < 4096 {
            self.addw(rd, rn, imm as u16);
        } else {
            debug_assert!(rd != rn);
            self.mov_imm(rd, imm);
            self.dp(Dp::Add, false, rd, rn, rd);
        }
    }

    /// `sp = sp - imm` (`grow`) or `sp + imm`, using `scratch` for large frames.
    pub fn adjust_sp(&mut self, imm: u32, grow: bool, scratch: u8) {
        if imm == 0 {
            return;
        }
        if imm < 4096 {
            if grow {
                self.subw(SP, SP, imm as u16);
            } else {
                self.addw(SP, SP, imm as u16);
            }
        } else {
            self.mov_imm(scratch, imm);
            self.dp(if grow { Dp::Sub } else { Dp::Add }, false, SP, SP, scratch);
        }
    }

    pub fn subw(&mut self, rd: u8, rn: u8, imm: u16) {
        self.addsubw(0xF2A0, rd, rn, imm);
    }

    fn addsubw(&mut self, base: u16, rd: u8, rn: u8, imm: u16) {
        debug_assert!(imm < 4096);
        self.w(
            base | (imm >> 11) << 10 | r(rn),
            ((imm >> 8) & 7) << 12 | r(rd) << 8 | (imm & 0xFF),
        );
    }

    pub fn mul(&mut self, rd: u8, rn: u8, rm: u8) {
        self.w(0xFB00 | r(rn), 0xF000 | r(rd) << 8 | r(rm));
    }

    /// `rd = ra + rn * rm`.
    pub fn mla(&mut self, rd: u8, rn: u8, rm: u8, ra: u8) {
        self.w(0xFB00 | r(rn), r(ra) << 12 | r(rd) << 8 | r(rm));
    }

    /// `rdhi:rdlo = rn * rm` (unsigned).
    pub fn umull(&mut self, rdlo: u8, rdhi: u8, rn: u8, rm: u8) {
        self.w(0xFBA0 | r(rn), r(rdlo) << 12 | r(rdhi) << 8 | r(rm));
    }

    // ----- memory ------------------------------------------------------------------------

    /// `ldr.w rt, [rn, #off]` (`off` < 4096).
    pub fn ldr(&mut self, rt: u8, rn: u8, off: u32) {
        debug_assert!(off < 4096);
        self.w(0xF8D0 | r(rn), r(rt) << 12 | off as u16);
    }

    pub fn str(&mut self, rt: u8, rn: u8, off: u32) {
        debug_assert!(off < 4096);
        self.w(0xF8C0 | r(rn), r(rt) << 12 | off as u16);
    }

    /// `ldrd rt, rt2, [rn, #off]` (`off` a multiple of 4 below 1024).
    pub fn ldrd(&mut self, rt: u8, rt2: u8, rn: u8, off: u32) {
        debug_assert!(off < 1024 && off.is_multiple_of(4));
        self.w(0xE9D0 | r(rn), r(rt) << 12 | r(rt2) << 8 | (off / 4) as u16);
    }

    pub fn strd(&mut self, rt: u8, rt2: u8, rn: u8, off: u32) {
        debug_assert!(off < 1024 && off.is_multiple_of(4));
        self.w(0xE9C0 | r(rn), r(rt) << 12 | r(rt2) << 8 | (off / 4) as u16);
    }

    /// `ldr{b,sb,h,sh}.w / ldr.w rt, [rn, rm, lsl #shift]` for `width` bytes (1, 2 or 4).
    pub fn ldr_idx(&mut self, rt: u8, rn: u8, rm: u8, width: u8, signed: bool, shift: u8) {
        let base = match (width, signed) {
            (1, false) => 0xF810,
            (1, true) => 0xF910,
            (2, false) => 0xF830,
            (2, true) => 0xF930,
            _ => 0xF850,
        };
        self.w(base | r(rn), r(rt) << 12 | (shift as u16) << 4 | r(rm));
    }

    pub fn str_idx(&mut self, rt: u8, rn: u8, rm: u8, width: u8, shift: u8) {
        let base = match width {
            1 => 0xF800,
            2 => 0xF820,
            _ => 0xF840,
        };
        self.w(base | r(rn), r(rt) << 12 | (shift as u16) << 4 | r(rm));
    }

    /// `push.w {regs}` / `pop.w {regs}` (bit mask of core registers).
    pub fn push(&mut self, mask: u16) {
        self.w(0xE92D, mask);
    }

    pub fn pop(&mut self, mask: u16) {
        self.w(0xE8BD, mask);
    }

    // ----- floating point ----------------------------------------------------------------

    /// `vldr dd, [rn, #off]` (`off` a multiple of 4 below 1024).
    pub fn vldr(&mut self, dd: u8, rn: u8, off: u32) {
        debug_assert!(off < 1024 && off.is_multiple_of(4));
        self.w(0xED90 | r(rn), d(dd) << 12 | 0xB00 | (off / 4) as u16);
    }

    pub fn vstr(&mut self, dd: u8, rn: u8, off: u32) {
        debug_assert!(off < 1024 && off.is_multiple_of(4));
        self.w(0xED80 | r(rn), d(dd) << 12 | 0xB00 | (off / 4) as u16);
    }

    /// `vpush {d<first>..d<first+count-1>}` / `vpop`.
    pub fn vpush(&mut self, first: u8, count: u8) {
        self.w(0xED2D, d(first) << 12 | 0xB00 | (2 * count as u16));
    }

    pub fn vpop(&mut self, first: u8, count: u8) {
        self.w(0xECBD, d(first) << 12 | 0xB00 | (2 * count as u16));
    }

    pub fn fop(&mut self, op: Fop, dd: u8, dn: u8, dm: u8) {
        let (hw1, sub) = match op {
            Fop::Add => (0xEE30, 0),
            Fop::Sub => (0xEE30, 0x40),
            Fop::Mul => (0xEE20, 0),
            Fop::Div => (0xEE80, 0),
        };
        self.w(hw1 | d(dn), d(dd) << 12 | 0xB00 | sub | d(dm));
    }

    pub fn vmov(&mut self, dd: u8, dm: u8) {
        self.w(0xEEB0, d(dd) << 12 | 0xB40 | d(dm));
    }

    pub fn vneg(&mut self, dd: u8, dm: u8) {
        self.w(0xEEB1, d(dd) << 12 | 0xB40 | d(dm));
    }

    /// `vcmp.f64 dd, dm` then `vmrs APSR_nzcv, fpscr`: an unordered result sets `C` and `V`.
    pub fn vcmp(&mut self, dd: u8, dm: u8) {
        self.w(0xEEB4, d(dd) << 12 | 0xB40 | d(dm));
        self.w(0xEEF1, 0xFA10);
    }

    /// `vmov dm, rt, rt2` (`rt` the low word).
    pub fn vmov_from_core(&mut self, dm: u8, rt: u8, rt2: u8) {
        self.w(0xEC40 | r(rt2), r(rt) << 12 | 0xB10 | d(dm));
    }

    /// `vmov rt, rt2, dm`.
    pub fn vmov_to_core(&mut self, rt: u8, rt2: u8, dm: u8) {
        self.w(0xEC50 | r(rt2), r(rt) << 12 | 0xB10 | d(dm));
    }

    /// Single-precision register `s` (0–31) as the `Vx:x` field pair: `(vx, x)`.
    fn s(s: u8) -> (u16, u16) {
        debug_assert!(s < 32);
        ((s >> 1) as u16, (s & 1) as u16)
    }

    /// `vmov s, rt`.
    pub fn vmov_s_from_core(&mut self, s: u8, rt: u8) {
        let (vn, n) = Self::s(s);
        self.w(0xEE00 | vn, r(rt) << 12 | 0xA10 | n << 7);
    }

    /// `vmov rt, s`.
    pub fn vmov_s_to_core(&mut self, rt: u8, s: u8) {
        let (vn, n) = Self::s(s);
        self.w(0xEE10 | vn, r(rt) << 12 | 0xA10 | n << 7);
    }

    /// `vcvt.f64.s32 dd, s` / `vcvt.f64.u32 dd, s`.
    pub fn vcvt_f64_from_32(&mut self, dd: u8, s: u8, signed: bool) {
        let (vm, m) = Self::s(s);
        self.w(
            0xEEB8,
            d(dd) << 12 | 0xB40 | (signed as u16) << 7 | m << 5 | vm,
        );
    }

    /// `vcvt.s32.f64 s, dm` / `vcvt.u32.f64 s, dm`, rounding toward zero.
    pub fn vcvt_32_from_f64(&mut self, s: u8, dm: u8, signed: bool) {
        let (vd, dbit) = Self::s(s);
        self.w(0xEEBC | dbit << 6 | signed as u16, vd << 12 | 0xBC0 | d(dm));
    }

    /// `vrintm.f64` (toward −∞) / `vrintp.f64` (toward +∞) / `vrintz.f64` (toward zero).
    pub fn vrintm(&mut self, dd: u8, dm: u8) {
        self.w(0xFEBB, d(dd) << 12 | 0xB40 | d(dm));
    }

    pub fn vrintp(&mut self, dd: u8, dm: u8) {
        self.w(0xFEBA, d(dd) << 12 | 0xB40 | d(dm));
    }

    pub fn vrintz(&mut self, dd: u8, dm: u8) {
        self.w(0xEEB6, d(dd) << 12 | 0xBC0 | d(dm));
    }
}

/// Offset fields of `b.w`/`bl` (`delta` from the instruction's address + 4).
fn branch_bits(delta: i64, b: bool) -> (u16, u16) {
    let _ = b;
    let v = delta as u32;
    let s = (v >> 24) & 1;
    let i1 = (v >> 23) & 1;
    let i2 = (v >> 22) & 1;
    let j1 = (!(i1 ^ s)) & 1;
    let j2 = (!(i2 ^ s)) & 1;
    let imm10 = (v >> 12) & 0x3FF;
    let imm11 = (v >> 1) & 0x7FF;
    (
        (s << 10 | imm10) as u16,
        (j1 << 13 | j2 << 11 | imm11) as u16,
    )
}

/// Patch the `bl` at `code[at..]` to reach `target` (offsets within one code buffer).
pub fn link_bl(code: &mut [u8], at: usize, target: usize) -> Result<(), String> {
    let delta = target as i64 - (at as i64 + 4);
    if !(-(1 << 24)..(1 << 24)).contains(&delta) {
        return Err("nguvu: picha ni kubwa mno kwa mwito wa moja kwa moja".into());
    }
    let (hw1, hw2) = branch_bits(delta, false);
    code[at..at + 2].copy_from_slice(&(0xF000 | hw1).to_le_bytes());
    code[at + 2..at + 4].copy_from_slice(&(0xD000 | hw2).to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(f: impl FnOnce(&mut Asm)) -> Vec<u8> {
        let mut a = Asm::new();
        f(&mut a);
        a.finish().unwrap().0
    }

    /// Each encoding against the bytes `llvm-mc` gives for the same instruction (skipped where
    /// `llvm-mc` is not installed).
    #[test]
    fn encodings_match_llvm() {
        type Case = (&'static str, fn(&mut Asm));
        let cases: Vec<Case> = vec![
            ("add.w r0, r1, r2", |a| a.dp(Dp::Add, false, 0, 1, 2)),
            ("adds.w r4, r5, r6", |a| a.dp(Dp::Add, true, 4, 5, 6)),
            ("adc.w r4, r5, r6", |a| a.dp(Dp::Adc, false, 4, 5, 6)),
            ("subs.w r8, r9, r10", |a| a.dp(Dp::Sub, true, 8, 9, 10)),
            ("sbc.w r11, r12, lr", |a| a.dp(Dp::Sbc, false, 11, 12, 14)),
            ("sbcs.w r0, r1, r2", |a| a.dp(Dp::Sbc, true, 0, 1, 2)),
            ("and.w r0, r1, r2", |a| a.dp(Dp::And, false, 0, 1, 2)),
            ("orr.w r3, r4, r5", |a| a.dp(Dp::Orr, false, 3, 4, 5)),
            ("eor.w r3, r4, r5", |a| a.dp(Dp::Eor, false, 3, 4, 5)),
            ("orrs.w r3, r4, r5", |a| a.dp(Dp::Orr, true, 3, 4, 5)),
            ("rsb.w r0, r1, r2", |a| a.dp(Dp::Rsb, false, 0, 1, 2)),
            ("rsbs.w r0, r1, #0", |a| {
                assert!(a.dp_imm(Dp::Rsb, true, 0, 1, 0));
            }),
            ("mov.w r4, r12", |a| a.mov(4, 12)),
            ("mvn.w r4, r12", |a| a.mvn(4, 12)),
            ("orr.w r0, r1, r2, lsl #7", |a| {
                a.dp_shifted(Dp::Orr, false, 0, 1, 2, Shift::Lsl, 7)
            }),
            ("cmp.w r0, r1, asr #31", |a| {
                a.dp_shifted(Dp::Sub, true, 15, 0, 1, Shift::Asr, 31)
            }),
            ("lsl.w r0, r1, #5", |a| a.shift_imm(Shift::Lsl, 0, 1, 5)),
            ("lsr.w r0, r1, #31", |a| a.shift_imm(Shift::Lsr, 0, 1, 31)),
            ("asr.w r2, r3, #1", |a| a.shift_imm(Shift::Asr, 2, 3, 1)),
            ("lsl.w r0, r1, r2", |a| {
                a.shift_reg(Shift::Lsl, false, 0, 1, 2)
            }),
            ("lsr.w r4, r5, r6", |a| {
                a.shift_reg(Shift::Lsr, false, 4, 5, 6)
            }),
            ("asrs.w r4, r5, r6", |a| {
                a.shift_reg(Shift::Asr, true, 4, 5, 6)
            }),
            ("cmp.w r3, r4", |a| a.cmp(3, 4)),
            ("tst.w r3, r4", |a| a.tst(3, 4)),
            ("cmp.w r5, #255", |a| assert!(a.cmp_imm(5, 255))),
            ("cmp.w r5, #0x10000", |a| assert!(a.cmp_imm(5, 0x10000))),
            ("and.w r1, r2, #0xff00ff00", |a| {
                assert!(a.dp_imm(Dp::And, false, 1, 2, 0xFF00_FF00))
            }),
            ("sub.w r1, r2, #0x3fc", |a| {
                assert!(a.dp_imm(Dp::Sub, false, 1, 2, 0x3FC))
            }),
            ("movw r7, #0xbeef", |a| a.movw(7, 0xBEEF)),
            ("movt r7, #0xdead", |a| a.movt(7, 0xDEAD)),
            ("mov.w r2, #0x55555555", |a| a.mov_imm(2, 0x5555_5555)),
            ("mov.w r2, #0xffffffff", |a| a.mov_imm(2, 0xFFFF_FFFF)),
            ("mvn.w r2, #0x100", |a| a.mov_imm(2, 0xFFFF_FEFF)),
            ("addw r0, sp, #1000", |a| a.addw(0, SP, 1000)),
            ("subw sp, sp, #4095", |a| a.subw(SP, SP, 4095)),
            ("mul r0, r1, r2", |a| a.mul(0, 1, 2)),
            ("mla r0, r1, r2, r3", |a| a.mla(0, 1, 2, 3)),
            ("umull r0, r1, r2, r3", |a| a.umull(0, 1, 2, 3)),
            ("ldr.w r0, [sp, #4092]", |a| a.ldr(0, SP, 4092)),
            ("str.w lr, [r1, #8]", |a| a.str(14, 1, 8)),
            ("ldrd r0, r1, [sp, #1020]", |a| a.ldrd(0, 1, SP, 1020)),
            ("strd r12, lr, [r4, #16]", |a| a.strd(12, 14, 4, 16)),
            ("ldr.w r0, [r1, r2, lsl #2]", |a| {
                a.ldr_idx(0, 1, 2, 4, false, 2)
            }),
            ("ldrsb.w r0, [r1, r2]", |a| a.ldr_idx(0, 1, 2, 1, true, 0)),
            ("ldrh.w r0, [r1, r2, lsl #1]", |a| {
                a.ldr_idx(0, 1, 2, 2, false, 1)
            }),
            ("ldrsh.w r0, [r1, r2, lsl #1]", |a| {
                a.ldr_idx(0, 1, 2, 2, true, 1)
            }),
            ("ldrb.w r0, [r1, r2]", |a| a.ldr_idx(0, 1, 2, 1, false, 0)),
            ("strb.w r0, [r1, r2]", |a| a.str_idx(0, 1, 2, 1, 0)),
            ("strh.w r0, [r1, r2, lsl #1]", |a| a.str_idx(0, 1, 2, 2, 1)),
            ("str.w r0, [r1, r2, lsl #3]", |a| a.str_idx(0, 1, 2, 4, 3)),
            ("push.w {r4, r5, r6, r7, r8, r9, r10, r11, lr}", |a| {
                a.push(0x4FF0)
            }),
            ("pop.w {r4, r5, r6, r7, r8, r9, r10, r11, pc}", |a| {
                a.pop(0x8FF0)
            }),
            ("vldr d3, [sp, #1016]", |a| a.vldr(3, SP, 1016)),
            ("vstr d15, [r2, #8]", |a| a.vstr(15, 2, 8)),
            ("vpush {d8, d9, d10, d11, d12, d13, d14, d15}", |a| {
                a.vpush(8, 8)
            }),
            ("vpop {d8, d9}", |a| a.vpop(8, 2)),
            ("vadd.f64 d0, d1, d2", |a| a.fop(Fop::Add, 0, 1, 2)),
            ("vsub.f64 d15, d14, d13", |a| a.fop(Fop::Sub, 15, 14, 13)),
            ("vmul.f64 d3, d4, d5", |a| a.fop(Fop::Mul, 3, 4, 5)),
            ("vdiv.f64 d6, d7, d8", |a| a.fop(Fop::Div, 6, 7, 8)),
            ("vmov.f64 d1, d9", |a| a.vmov(1, 9)),
            ("vneg.f64 d1, d9", |a| a.vneg(1, 9)),
            ("vcmp.f64 d1, d2\n vmrs APSR_nzcv, fpscr", |a| a.vcmp(1, 2)),
            ("vmov d5, r2, r3", |a| a.vmov_from_core(5, 2, 3)),
            ("vmov r12, lr, d15", |a| a.vmov_to_core(12, 14, 15)),
            ("vmov s13, r4", |a| a.vmov_s_from_core(13, 4)),
            ("vmov r4, s12", |a| a.vmov_s_to_core(4, 12)),
            ("vcvt.f64.s32 d2, s13", |a| a.vcvt_f64_from_32(2, 13, true)),
            ("vcvt.f64.u32 d2, s12", |a| a.vcvt_f64_from_32(2, 12, false)),
            ("vcvt.s32.f64 s13, d2", |a| a.vcvt_32_from_f64(13, 2, true)),
            ("vcvt.u32.f64 s12, d9", |a| a.vcvt_32_from_f64(12, 9, false)),
            ("vrintm.f64 d0, d1", |a| a.vrintm(0, 1)),
            ("vrintp.f64 d2, d3", |a| a.vrintp(2, 3)),
            ("vrintz.f64 d4, d5", |a| a.vrintz(4, 5)),
            ("blx r12", |a| a.blx(12)),
            ("bx lr", |a| a.bx_lr()),
            ("ite gt", |a| a.it(Cond::Gt, &[true, false])),
            ("itte lo", |a| a.it(Cond::Lo, &[true, true, false])),
            ("itett eq", |a| a.it(Cond::Eq, &[true, false, true, true])),
            ("it ne", |a| a.it(Cond::Ne, &[true])),
        ];
        let mut text = String::from(".syntax unified\n.thumb\n");
        for (asm, _) in &cases {
            text.push_str(asm);
            text.push('\n');
            // An `it` block needs its conditional instructions after it.
            if let Some(rest) = asm.strip_prefix("it") {
                let (mask, cond) = rest.split_once(' ').unwrap();
                let c = cond.trim();
                let inv = match c {
                    "gt" => "le",
                    "lo" => "hs",
                    "eq" => "ne",
                    "ne" => "eq",
                    _ => unreachable!(),
                };
                text.push_str(&format!("nop{c}\n"));
                for ch in mask.chars() {
                    text.push_str(&format!("nop{}\n", if ch == 't' { c } else { inv }));
                }
            }
        }
        let dir = std::env::temp_dir().join(format!("asili-t32-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("t.s");
        std::fs::write(&src, &text).unwrap();
        let Ok(out) = std::process::Command::new("llvm-mc")
            .args([
                "--triple=thumbv7em-none-eabihf",
                "-mattr=+fp-armv8d16,+vfp4d16",
                "--show-encoding",
            ])
            .arg(&src)
            .output()
        else {
            return; // no llvm-mc
        };
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let listing = String::from_utf8_lossy(&out.stdout);
        let mut expected = listing.lines().filter_map(|l| {
            let enc = l.split_once("encoding: [")?.1.split_once(']')?.0;
            Some(
                enc.split(',')
                    .map(|b| u8::from_str_radix(b.trim().trim_start_matches("0x"), 16).unwrap())
                    .collect::<Vec<u8>>(),
            )
        });
        for (asm, f) in &cases {
            let mut want = Vec::new();
            for _ in 0..asm.lines().count() {
                want.extend(expected.next().unwrap());
            }
            if asm.starts_with("it") {
                let n = asm.split_once(' ').unwrap().0.len() - 1;
                for _ in 0..n {
                    expected.next();
                }
            }
            assert_eq!(bytes(*f), want, "{asm}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn branches_reach_their_labels() {
        // Check against llvm-mc's own branch encodings for the same distances.
        let mut a = Asm::new();
        let back = a.new_label();
        a.bind(back);
        a.w(0xF3AF, 0x8000); // nop.w
        let fwd = a.new_label();
        a.b(fwd);
        a.b_cond(Cond::Ne, back);
        a.bind(fwd);
        a.b(back);
        let (code, _) = a.finish().unwrap();
        // b.w +4 (to offset 12 from 4); bne.w -12 (to 0 from 8); b.w -16 (to 0 from 12).
        assert_eq!(&code[4..8], &[0x00, 0xF0, 0x02, 0xB8]);
        assert_eq!(&code[8..12], &[0x7F, 0xF4, 0xFA, 0xAF]);
        assert_eq!(&code[12..16], &[0xFF, 0xF7, 0xF8, 0xBF]);
    }

    #[test]
    fn modified_immediates() {
        for v in [
            0u32,
            1,
            255,
            0x00AB_00AB,
            0xAB00_AB00,
            0xABAB_ABAB,
            0x8000_0000,
            0x3FC,
            0xFF00_0000,
        ] {
            assert!(modified_imm(v).is_some(), "{v:#x}");
        }
        for v in [0x101u32, 0x1234, 0xFFFF_FFFE] {
            assert!(modified_imm(v).is_none(), "{v:#x}");
        }
    }
}
