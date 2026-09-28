//! AArch64 (A64) instruction encoder: the subset `codegen_a64` uses, with labels for forward
//! branches. Every instruction is one little-endian 32-bit word. Registers are numbered 0–30;
//! 31 means `xzr`/`wzr` in most operand slots and `sp` in the address/immediate-add slots noted
//! below.

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

/// `xzr` in data-processing operands, `sp` as a base address or in `add`/`sub` immediates.
pub const ZR: u8 = 31;
pub const SP: u8 = 31;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Label(u32);

/// Register-register integer operations (64-bit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Alu {
    Add = 0x8B00_0000,
    Sub = 0xCB00_0000,
    And = 0x8A00_0000,
    Orr = 0xAA00_0000,
    Eor = 0xCA00_0000,
    /// `subs` (with `rd = xzr`: `cmp`).
    Subs = 0xEB00_0000,
    /// `ands` (with `rd = xzr`: `tst`).
    Ands = 0xEA00_0000,
    Lslv = 0x9AC0_2000,
    Asrv = 0x9AC0_2800,
    Sdiv = 0x9AC0_0C00,
    Udiv = 0x9AC0_0800,
    /// `madd rd, rn, rm, xzr`.
    Mul = 0x9B00_7C00,
    Smulh = 0x9B40_7C00,
}

/// Scalar double-precision operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Fop {
    Add = 0x1E60_2800,
    Sub = 0x1E60_3800,
    Mul = 0x1E60_0800,
    Div = 0x1E60_1800,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fix {
    /// `b`: 26-bit word offset.
    B26,
    /// `b.cond`/`cbz`/`cbnz`: 19-bit word offset at bit 5.
    B19,
}

pub struct Asm {
    code: Vec<u8>,
    labels: Vec<Option<usize>>,
    fixups: Vec<(usize, Label, Fix)>,
}

impl Default for Asm {
    fn default() -> Self {
        Self::new()
    }
}

fn r(n: u8) -> u32 {
    debug_assert!(n < 32);
    n as u32
}

impl Asm {
    pub fn new() -> Self {
        Asm {
            code: Vec::new(),
            labels: Vec::new(),
            fixups: Vec::new(),
        }
    }

    pub fn word(&mut self, w: u32) {
        self.code.extend_from_slice(&w.to_le_bytes());
    }

    pub fn new_label(&mut self) -> Label {
        self.labels.push(None);
        Label(self.labels.len() as u32 - 1)
    }

    pub fn bind(&mut self, l: Label) {
        self.labels[l.0 as usize] = Some(self.code.len());
    }

    /// Resolve branches; fails if a branch cannot reach its target.
    pub fn finish(mut self) -> Result<Vec<u8>, String> {
        for &(at, l, kind) in &self.fixups {
            let target = self.labels[l.0 as usize].expect("unbound label");
            let delta = (target as i64 - at as i64) / 4;
            let (bits, shift) = match kind {
                Fix::B26 => (26, 0),
                Fix::B19 => (19, 5),
            };
            if delta < -(1 << (bits - 1)) || delta >= 1 << (bits - 1) {
                return Err("nguvu: kazi ni kubwa mno kwa tawi la AArch64".into());
            }
            let field = ((delta as u32) & ((1 << bits) - 1)) << shift;
            let mut w = u32::from_le_bytes(self.code[at..at + 4].try_into().unwrap());
            w |= field;
            self.code[at..at + 4].copy_from_slice(&w.to_le_bytes());
        }
        Ok(self.code)
    }

    fn branch(&mut self, w: u32, l: Label, kind: Fix) {
        self.fixups.push((self.code.len(), l, kind));
        self.word(w);
    }

    // ----- control flow ------------------------------------------------------------------

    pub fn b(&mut self, l: Label) {
        self.branch(0x1400_0000, l, Fix::B26);
    }

    pub fn b_cond(&mut self, c: Cond, l: Label) {
        self.branch(0x5400_0000 | c as u32, l, Fix::B19);
    }

    pub fn cbnz(&mut self, rt: u8, l: Label) {
        self.branch(0xB500_0000 | r(rt), l, Fix::B19);
    }

    pub fn blr(&mut self, rn: u8) {
        self.word(0xD63F_0000 | r(rn) << 5);
    }

    pub fn ret(&mut self) {
        self.word(0xD65F_03C0);
    }

    // ----- integer -----------------------------------------------------------------------

    pub fn alu(&mut self, op: Alu, rd: u8, rn: u8, rm: u8) {
        self.word(op as u32 | r(rm) << 16 | r(rn) << 5 | r(rd));
    }

    /// `rd = ra - rn * rm`.
    pub fn msub(&mut self, rd: u8, rn: u8, rm: u8, ra: u8) {
        self.word(0x9B00_8000 | r(rm) << 16 | r(ra) << 10 | r(rn) << 5 | r(rd));
    }

    /// `cmp rn, rm, asr #amount`.
    pub fn cmp_asr(&mut self, rn: u8, rm: u8, amount: u8) {
        self.word(0xEB80_0000 | r(rm) << 16 | (amount as u32 & 63) << 10 | r(rn) << 5 | 31);
    }

    pub fn mov(&mut self, rd: u8, rm: u8) {
        if rd != rm {
            self.alu(Alu::Orr, rd, ZR, rm);
        }
    }

    /// `mov wd, wm`: zero-extends the low 32 bits.
    pub fn mov32(&mut self, rd: u8, rm: u8) {
        self.word(0x2A00_03E0 | r(rm) << 16 | r(rd));
    }

    /// `mvn rd, rm` (`orn rd, xzr, rm`).
    pub fn mvn(&mut self, rd: u8, rm: u8) {
        self.word(0xAA20_03E0 | r(rm) << 16 | r(rd));
    }

    /// `add`/`sub` with a 12-bit immediate, optionally shifted left by 12. Register 31 is `sp`.
    pub fn add_imm(&mut self, rd: u8, rn: u8, imm12: u32, shift12: bool) {
        self.word(
            0x9100_0000 | (shift12 as u32) << 22 | (imm12 & 0xFFF) << 10 | r(rn) << 5 | r(rd),
        );
    }

    pub fn sub_imm(&mut self, rd: u8, rn: u8, imm12: u32, shift12: bool) {
        self.word(
            0xD100_0000 | (shift12 as u32) << 22 | (imm12 & 0xFFF) << 10 | r(rn) << 5 | r(rd),
        );
    }

    /// `cmp rn, #imm12` / `cmn rn, #imm12`.
    pub fn cmp_imm(&mut self, rn: u8, imm12: u32) {
        self.word(0xF100_001F | (imm12 & 0xFFF) << 10 | r(rn) << 5);
    }

    pub fn cmn_imm(&mut self, rn: u8, imm12: u32) {
        self.word(0xB100_001F | (imm12 & 0xFFF) << 10 | r(rn) << 5);
    }

    /// `lsl rd, rn, #s` (`ubfm rd, rn, #(-s mod 64), #(63 - s)`).
    pub fn lsl_imm(&mut self, rd: u8, rn: u8, s: u8) {
        let s = s as u32 & 63;
        let immr = (64 - s) & 63;
        let imms = 63 - s;
        self.word(0xD340_0000 | immr << 16 | imms << 10 | r(rn) << 5 | r(rd));
    }

    /// `asr rd, rn, #s` (`sbfm rd, rn, #s, #63`).
    pub fn asr_imm(&mut self, rd: u8, rn: u8, s: u8) {
        self.word(0x9340_FC00 | (s as u32 & 63) << 16 | r(rn) << 5 | r(rd));
    }

    /// `csel rd, rn, rm, c`: `rd = c ? rn : rm`.
    pub fn csel(&mut self, rd: u8, rn: u8, rm: u8, c: Cond) {
        self.word(0x9A80_0000 | r(rm) << 16 | (c as u32) << 12 | r(rn) << 5 | r(rd));
    }

    /// `cset rd, c` (`csinc rd, xzr, xzr, !c`).
    pub fn cset(&mut self, rd: u8, c: Cond) {
        self.word(0x9A9F_07E0 | (c.invert() as u32) << 12 | r(rd));
    }

    /// Load any 64-bit constant (`movz`/`movn` + `movk`s).
    pub fn mov_imm(&mut self, rd: u8, value: i64) {
        let v = value as u64;
        let halves = |x: u64| (0..4).map(move |i| ((x >> (16 * i)) & 0xFFFF) as u32);
        let zeros = halves(v).filter(|h| *h == 0).count();
        let ones = halves(v).filter(|h| *h == 0xFFFF).count();
        if ones > zeros {
            // movn writes !(imm16 << shift): start from all ones.
            let mut first = true;
            for (i, h) in halves(v).enumerate() {
                if h == 0xFFFF {
                    continue;
                }
                if first {
                    self.word(0x9280_0000 | (i as u32) << 21 | (!h & 0xFFFF) << 5 | r(rd));
                    first = false;
                } else {
                    self.word(0xF280_0000 | (i as u32) << 21 | h << 5 | r(rd));
                }
            }
            if first {
                self.word(0x9280_0000 | r(rd)); // movn rd, #0: all ones
            }
        } else {
            let mut first = true;
            for (i, h) in halves(v).enumerate() {
                if h == 0 {
                    continue;
                }
                let op = if first { 0xD280_0000 } else { 0xF280_0000 };
                self.word(op | (i as u32) << 21 | h << 5 | r(rd));
                first = false;
            }
            if first {
                self.word(0xD280_0000 | r(rd)); // movz rd, #0
            }
        }
    }

    // ----- memory ------------------------------------------------------------------------

    /// `ldr xt, [xn, #off]` for `0 <= off < 32768`, `off % 8 == 0`. Register 31 is `sp`.
    pub fn ldr(&mut self, rt: u8, rn: u8, off: u32) {
        debug_assert!(off.is_multiple_of(8) && off / 8 < 4096);
        self.word(0xF940_0000 | (off / 8) << 10 | r(rn) << 5 | r(rt));
    }

    pub fn str(&mut self, rt: u8, rn: u8, off: u32) {
        debug_assert!(off.is_multiple_of(8) && off / 8 < 4096);
        self.word(0xF900_0000 | (off / 8) << 10 | r(rn) << 5 | r(rt));
    }

    pub fn ldr_d(&mut self, dt: u8, rn: u8, off: u32) {
        debug_assert!(off.is_multiple_of(8) && off / 8 < 4096);
        self.word(0xFD40_0000 | (off / 8) << 10 | r(rn) << 5 | r(dt));
    }

    pub fn str_d(&mut self, dt: u8, rn: u8, off: u32) {
        debug_assert!(off.is_multiple_of(8) && off / 8 < 4096);
        self.word(0xFD00_0000 | (off / 8) << 10 | r(rn) << 5 | r(dt));
    }

    /// `ldr xt, [xn, xm, lsl #3]`.
    pub fn ldr_idx(&mut self, rt: u8, rn: u8, rm: u8) {
        self.word(0xF860_7800 | r(rm) << 16 | r(rn) << 5 | r(rt));
    }

    pub fn str_idx(&mut self, rt: u8, rn: u8, rm: u8) {
        self.word(0xF820_7800 | r(rm) << 16 | r(rn) << 5 | r(rt));
    }

    pub fn ldr_d_idx(&mut self, dt: u8, rn: u8, rm: u8) {
        self.word(0xFC60_7800 | r(rm) << 16 | r(rn) << 5 | r(dt));
    }

    pub fn str_d_idx(&mut self, dt: u8, rn: u8, rm: u8) {
        self.word(0xFC20_7800 | r(rm) << 16 | r(rn) << 5 | r(dt));
    }

    /// `stp x29, x30, [sp, #-16]!`.
    pub fn push_frame_record(&mut self) {
        self.word(0xA9BF_7BFD);
    }

    /// `ldp x29, x30, [sp], #16`.
    pub fn pop_frame_record(&mut self) {
        self.word(0xA8C1_7BFD);
    }

    // ----- floating point ----------------------------------------------------------------

    pub fn fop(&mut self, op: Fop, dd: u8, dn: u8, dm: u8) {
        self.word(op as u32 | r(dm) << 16 | r(dn) << 5 | r(dd));
    }

    pub fn fmov(&mut self, dd: u8, dn: u8) {
        if dd != dn {
            self.word(0x1E60_4000 | r(dn) << 5 | r(dd));
        }
    }

    pub fn fcmp(&mut self, dn: u8, dm: u8) {
        self.word(0x1E60_2000 | r(dm) << 16 | r(dn) << 5);
    }

    /// `scvtf dd, xn`.
    pub fn scvtf(&mut self, dd: u8, rn: u8) {
        self.word(0x9E62_0000 | r(rn) << 5 | r(dd));
    }

    /// `fcvtzs xd, dn` (truncating).
    pub fn fcvtzs(&mut self, rd: u8, dn: u8) {
        self.word(0x9E78_0000 | r(dn) << 5 | r(rd));
    }

    /// `fmov xd, dn` (bits).
    pub fn fmov_to_x(&mut self, rd: u8, dn: u8) {
        self.word(0x9E66_0000 | r(dn) << 5 | r(rd));
    }

    /// `fmov dd, xn` (bits).
    pub fn fmov_from_x(&mut self, dd: u8, rn: u8) {
        self.word(0x9E67_0000 | r(rn) << 5 | r(dd));
    }

    /// `cnt vd.8b, vn.8b`.
    pub fn cnt8b(&mut self, vd: u8, vn: u8) {
        self.word(0x0E20_5800 | r(vn) << 5 | r(vd));
    }

    /// `addv bd, vn.8b`.
    pub fn addv8b(&mut self, vd: u8, vn: u8) {
        self.word(0x0E31_B800 | r(vn) << 5 | r(vd));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(f: impl FnOnce(&mut Asm)) -> Vec<u32> {
        let mut a = Asm::new();
        f(&mut a);
        a.finish()
            .unwrap()
            .chunks(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }

    /// Expected words from GNU `as` (binutils 2.42, aarch64-linux-gnu) for the same instructions.
    #[test]
    fn encodings_match_gnu_as() {
        let cases: Vec<(Vec<u32>, &[u32])> = vec![
            (words(|a| a.alu(Alu::Add, 1, 2, 3)), &[0x8b030041]),
            (words(|a| a.alu(Alu::Sub, 19, 20, 28)), &[0xcb1c0293]),
            (words(|a| a.alu(Alu::And, 0, 1, 2)), &[0x8a020020]),
            (words(|a| a.alu(Alu::Orr, 5, 6, 7)), &[0xaa0700c5]),
            (words(|a| a.alu(Alu::Eor, 8, 9, 10)), &[0xca0a0128]),
            (words(|a| a.alu(Alu::Subs, ZR, 3, 4)), &[0xeb04007f]),
            (words(|a| a.alu(Alu::Ands, ZR, 3, 4)), &[0xea04007f]),
            (words(|a| a.alu(Alu::Lslv, 1, 2, 3)), &[0x9ac32041]),
            (words(|a| a.alu(Alu::Asrv, 1, 2, 3)), &[0x9ac32841]),
            (words(|a| a.alu(Alu::Sdiv, 1, 2, 3)), &[0x9ac30c41]),
            (words(|a| a.alu(Alu::Udiv, 1, 2, 3)), &[0x9ac30841]),
            (words(|a| a.alu(Alu::Mul, 1, 2, 3)), &[0x9b037c41]),
            (words(|a| a.alu(Alu::Smulh, 1, 2, 3)), &[0x9b437c41]),
            (words(|a| a.msub(1, 2, 3, 4)), &[0x9b039041]),
            (words(|a| a.cmp_asr(1, 2, 63)), &[0xeb82fc3f]),
            (words(|a| a.mov(3, 17)), &[0xaa1103e3]),
            (words(|a| a.mov32(0, 0)), &[0x2a0003e0]),
            (words(|a| a.mvn(1, 2)), &[0xaa2203e1]),
            (words(|a| a.add_imm(1, 2, 4095, false)), &[0x913ffc41]),
            (words(|a| a.add_imm(29, SP, 0, false)), &[0x910003fd]),
            (words(|a| a.sub_imm(SP, SP, 1, true)), &[0xd14007ff]),
            (words(|a| a.cmp_imm(5, 81)), &[0xf10144bf]),
            (words(|a| a.cmn_imm(5, 1)), &[0xb10004bf]),
            (words(|a| a.lsl_imm(1, 2, 3)), &[0xd37df041]),
            (words(|a| a.asr_imm(1, 2, 9)), &[0x9349fc41]),
            (words(|a| a.csel(1, 2, 3, Cond::Ne)), &[0x9a831041]),
            (words(|a| a.cset(1, Cond::Lt)), &[0x9a9fa7e1]),
            (words(|a| a.mov_imm(1, 0)), &[0xd2800001]),
            (
                words(|a| a.mov_imm(1, 0x1234_5678_9abc)),
                &[0xd2935781, 0xf2aacf01, 0xf2c24681],
            ),
            (words(|a| a.mov_imm(1, -1)), &[0x92800001]),
            (words(|a| a.mov_imm(1, -2)), &[0x92800021]),
            (words(|a| a.ldr(1, SP, 16)), &[0xf9400be1]),
            (words(|a| a.str(1, 2, 32760)), &[0xf93ffc41]),
            (words(|a| a.ldr_d(3, SP, 8)), &[0xfd4007e3]),
            (words(|a| a.str_d(3, SP, 8)), &[0xfd0007e3]),
            (words(|a| a.ldr_idx(1, 2, 3)), &[0xf8637841]),
            (words(|a| a.str_idx(1, 2, 3)), &[0xf8237841]),
            (words(|a| a.ldr_d_idx(1, 2, 3)), &[0xfc637841]),
            (words(|a| a.str_d_idx(1, 2, 3)), &[0xfc237841]),
            (words(|a| a.push_frame_record()), &[0xa9bf7bfd]),
            (words(|a| a.pop_frame_record()), &[0xa8c17bfd]),
            (words(|a| a.fop(Fop::Add, 1, 2, 3)), &[0x1e632841]),
            (words(|a| a.fop(Fop::Sub, 1, 2, 3)), &[0x1e633841]),
            (words(|a| a.fop(Fop::Mul, 1, 2, 3)), &[0x1e630841]),
            (words(|a| a.fop(Fop::Div, 1, 2, 3)), &[0x1e631841]),
            (words(|a| a.fmov(1, 2)), &[0x1e604041]),
            (words(|a| a.fcmp(1, 2)), &[0x1e622020]),
            (words(|a| a.scvtf(1, 2)), &[0x9e620041]),
            (words(|a| a.fcvtzs(1, 2)), &[0x9e780041]),
            (words(|a| a.fmov_to_x(1, 2)), &[0x9e660041]),
            (words(|a| a.fmov_from_x(1, 2)), &[0x9e670041]),
            (words(|a| a.cnt8b(16, 16)), &[0x0e205a10]),
            (words(|a| a.addv8b(16, 16)), &[0x0e31ba10]),
            (words(|a| a.blr(16)), &[0xd63f0200]),
            (words(|a| a.ret()), &[0xd65f03c0]),
        ];
        for (i, (got, want)) in cases.iter().enumerate() {
            assert_eq!(got.as_slice(), *want, "case {i}: {got:08x?} vs {want:08x?}");
        }
    }

    #[test]
    fn branches_resolve_both_directions() {
        let mut a = Asm::new();
        let top = a.new_label();
        let end = a.new_label();
        a.bind(top);
        a.b_cond(Cond::Eq, end); // +3 words
        a.cbnz(2, end); // +2
        a.b(top); // -3
        a.bind(end);
        let w: Vec<u32> = a
            .finish()
            .unwrap()
            .chunks(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(w, vec![0x54000060, 0xb5000042, 0x17fffffe]);
    }
}
