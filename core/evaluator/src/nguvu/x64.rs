//! Minimal x86-64 machine-code assembler: exactly the instructions `codegen` emits, encoded by
//! hand (REX prefixes, ModRM/SIB, rel32 branches with label fixups). No external assembler.

/// General-purpose registers, numbered as in the ISA encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Gpr {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
    Rsp = 4,
    Rbp = 5,
    Rsi = 6,
    Rdi = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}

/// SSE registers `xmm0`..`xmm15`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Xmm(pub u8);

/// Condition codes (the low nibble of `Jcc`/`SETcc`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Cond {
    O = 0x0,
    B = 0x2,
    Ae = 0x3,
    E = 0x4,
    Ne = 0x5,
    Be = 0x6,
    A = 0x7,
    P = 0xA,
    Np = 0xB,
    L = 0xC,
    Ge = 0xD,
    Le = 0xE,
    G = 0xF,
}

/// A memory operand `[base + disp]`.
#[derive(Clone, Copy, Debug)]
pub struct Mem {
    pub base: Gpr,
    pub disp: i32,
}

/// `[base + index*8 + disp]`.
#[derive(Clone, Copy, Debug)]
pub struct MemIdx {
    pub base: Gpr,
    pub index: Gpr,
    pub disp: i32,
    /// Index scale: 1, 2, 4 or 8.
    pub scale: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

/// Two-operand integer ALU instructions of the form `op r/m64, r64`.
#[derive(Clone, Copy, Debug)]
pub enum Alu {
    Add = 0x01,
    Or = 0x09,
    And = 0x21,
    Sub = 0x29,
    Xor = 0x31,
    Cmp = 0x39,
    Test = 0x85,
}

/// Scalar double SSE2 arithmetic (`F2 0F op`).
#[derive(Clone, Copy, Debug)]
pub enum Sse {
    Add = 0x58,
    Mul = 0x59,
    Sub = 0x5C,
    Div = 0x5E,
}

#[derive(Default)]
pub struct Asm {
    pub code: Vec<u8>,
    labels: Vec<Option<usize>>,
    fixups: Vec<(usize, Label)>,
}

fn low(r: u8) -> u8 {
    r & 7
}

fn hi(r: u8) -> u8 {
    (r >> 3) & 1
}

impl Asm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_label(&mut self) -> Label {
        self.labels.push(None);
        Label(self.labels.len() as u32 - 1)
    }

    pub fn bind(&mut self, label: Label) {
        self.labels[label.0 as usize] = Some(self.code.len());
    }

    /// Pad with multi-byte `nop`s up to a multiple of `n` bytes (a power of two).
    pub fn align(&mut self, n: usize) {
        const NOPS: [&[u8]; 9] = [
            &[0x90],
            &[0x66, 0x90],
            &[0x0F, 0x1F, 0x00],
            &[0x0F, 0x1F, 0x40, 0x00],
            &[0x0F, 0x1F, 0x44, 0x00, 0x00],
            &[0x66, 0x0F, 0x1F, 0x44, 0x00, 0x00],
            &[0x0F, 0x1F, 0x80, 0x00, 0x00, 0x00, 0x00],
            &[0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00],
            &[0x66, 0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00],
        ];
        let mut pad = self.code.len().wrapping_neg() & (n - 1);
        while pad > 0 {
            let k = pad.min(NOPS.len());
            self.bytes(NOPS[k - 1]);
            pad -= k;
        }
    }

    /// Resolve every rel32 fixup; call once after the last instruction.
    pub fn finish(mut self) -> Vec<u8> {
        for (at, label) in std::mem::take(&mut self.fixups) {
            let target = self.labels[label.0 as usize].expect("unbound label");
            let rel = target as i64 - (at as i64 + 4);
            self.code[at..at + 4].copy_from_slice(&(rel as i32).to_le_bytes());
        }
        self.code
    }

    fn byte(&mut self, b: u8) {
        self.code.push(b);
    }

    fn bytes(&mut self, bs: &[u8]) {
        self.code.extend_from_slice(bs);
    }

    fn rex(&mut self, w: bool, r: u8, x: u8, b: u8, force: bool) {
        let v = 0x40 | ((w as u8) << 3) | (hi(r) << 2) | (hi(x) << 1) | hi(b);
        if v != 0x40 || force {
            self.byte(v);
        }
    }

    /// The ModRM `mod` field for `disp` off `base`: none, 8 or 32 bits (`rbp`/`r13` have no
    /// displacement-free form).
    fn disp_mode(base: u8, disp: i32) -> u8 {
        if disp == 0 && low(base) != 5 {
            0x00
        } else if i8::try_from(disp).is_ok() {
            0x40
        } else {
            0x80
        }
    }

    fn disp(&mut self, mode: u8, disp: i32) {
        match mode {
            0x00 => {}
            0x40 => self.byte(disp as u8),
            _ => self.bytes(&disp.to_le_bytes()),
        }
    }

    /// ModRM (+SIB) + displacement for `[base + disp]` with `reg` in the reg field.
    fn modrm_mem(&mut self, reg: u8, m: Mem) {
        let base = m.base as u8;
        let mode = Self::disp_mode(base, m.disp);
        self.byte(mode | (low(reg) << 3) | low(base));
        if low(base) == 4 {
            // rsp/r12 as a base always needs a SIB byte.
            self.byte(0x24);
        }
        self.disp(mode, m.disp);
    }

    fn modrm_idx(&mut self, reg: u8, m: MemIdx) {
        let mode = Self::disp_mode(m.base as u8, m.disp);
        self.byte(mode | (low(reg) << 3) | 4);
        let ss = m.scale.trailing_zeros() as u8;
        self.byte(ss << 6 | (low(m.index as u8) << 3) | low(m.base as u8));
        self.disp(mode, m.disp);
    }

    fn modrm_rr(&mut self, reg: u8, rm: u8) {
        self.byte(0xC0 | (low(reg) << 3) | low(rm));
    }

    // ----- integer -----------------------------------------------------------------------

    pub fn mov_rr(&mut self, dst: Gpr, src: Gpr) {
        if dst == src {
            return;
        }
        self.rex(true, src as u8, 0, dst as u8, false);
        self.byte(0x89);
        self.modrm_rr(src as u8, dst as u8);
    }

    /// `mov r32, r32`: zero-extends into the full register.
    pub fn mov_rr32(&mut self, dst: Gpr, src: Gpr) {
        self.rex(false, src as u8, 0, dst as u8, false);
        self.byte(0x89);
        self.modrm_rr(src as u8, dst as u8);
    }

    pub fn mov_ri(&mut self, dst: Gpr, imm: i64) {
        if imm == 0 {
            // xor r32, r32 zero-extends and is shorter.
            self.rex(false, dst as u8, 0, dst as u8, false);
            self.byte(0x31);
            self.modrm_rr(dst as u8, dst as u8);
        } else if (0..=u32::MAX as i64).contains(&imm) {
            self.rex(false, 0, 0, dst as u8, false);
            self.byte(0xB8 + low(dst as u8));
            self.bytes(&(imm as u32).to_le_bytes());
        } else if i32::try_from(imm).is_ok() {
            self.rex(true, 0, 0, dst as u8, false);
            self.byte(0xC7);
            self.modrm_rr(0, dst as u8);
            self.bytes(&(imm as i32).to_le_bytes());
        } else {
            self.rex(true, 0, 0, dst as u8, false);
            self.byte(0xB8 + low(dst as u8));
            self.bytes(&imm.to_le_bytes());
        }
    }

    pub fn load(&mut self, dst: Gpr, m: Mem) {
        self.rex(true, dst as u8, 0, m.base as u8, false);
        self.byte(0x8B);
        self.modrm_mem(dst as u8, m);
    }

    pub fn store(&mut self, m: Mem, src: Gpr) {
        self.rex(true, src as u8, 0, m.base as u8, false);
        self.byte(0x89);
        self.modrm_mem(src as u8, m);
    }

    pub fn load_idx(&mut self, dst: Gpr, m: MemIdx) {
        self.rex(true, dst as u8, m.index as u8, m.base as u8, false);
        self.byte(0x8B);
        self.modrm_idx(dst as u8, m);
    }

    pub fn store_idx(&mut self, m: MemIdx, src: Gpr) {
        self.rex(true, src as u8, m.index as u8, m.base as u8, false);
        self.byte(0x89);
        self.modrm_idx(src as u8, m);
    }

    pub fn lea(&mut self, dst: Gpr, m: Mem) {
        self.rex(true, dst as u8, 0, m.base as u8, false);
        self.byte(0x8D);
        self.modrm_mem(dst as u8, m);
    }

    /// `op dst, src` for the two-operand ALU group (`cmp`/`test` only set flags).
    pub fn alu_rr(&mut self, op: Alu, dst: Gpr, src: Gpr) {
        self.rex(true, src as u8, 0, dst as u8, false);
        self.byte(op as u8);
        self.modrm_rr(src as u8, dst as u8);
    }

    /// `op dst, [m]` for add/or/and/sub/xor/cmp/test.
    pub fn alu_rm(&mut self, op: Alu, dst: Gpr, m: Mem) {
        self.rex(true, dst as u8, 0, m.base as u8, false);
        self.byte(match op {
            Alu::Test => 0x85,
            op => op as u8 + 2,
        });
        self.modrm_mem(dst as u8, m);
    }

    /// `cmp dst, imm`; `test dst, dst` for 0 (same flags, shorter).
    pub fn cmp_ri(&mut self, dst: Gpr, imm: i32) {
        if imm == 0 {
            self.alu_rr(Alu::Test, dst, dst);
        } else {
            self.alu_ri(Alu::Cmp, dst, imm);
        }
    }

    /// `op dst, imm32` (sign-extended), for add/or/and/sub/xor/cmp.
    pub fn alu_ri(&mut self, op: Alu, dst: Gpr, imm: i32) {
        let ext = match op {
            Alu::Add => 0,
            Alu::Or => 1,
            Alu::And => 4,
            Alu::Sub => 5,
            Alu::Xor => 6,
            Alu::Cmp => 7,
            Alu::Test => {
                self.rex(true, 0, 0, dst as u8, false);
                self.byte(0xF7);
                self.modrm_rr(0, dst as u8);
                self.bytes(&imm.to_le_bytes());
                return;
            }
        };
        self.rex(true, 0, 0, dst as u8, false);
        if (-128..=127).contains(&imm) {
            self.byte(0x83);
            self.modrm_rr(ext, dst as u8);
            self.byte(imm as i8 as u8);
        } else {
            self.byte(0x81);
            self.modrm_rr(ext, dst as u8);
            self.bytes(&imm.to_le_bytes());
        }
    }

    pub fn imul_rr(&mut self, dst: Gpr, src: Gpr) {
        self.rex(true, dst as u8, 0, src as u8, false);
        self.bytes(&[0x0F, 0xAF]);
        self.modrm_rr(dst as u8, src as u8);
    }

    /// `dst = src * imm`.
    pub fn imul_rri(&mut self, dst: Gpr, src: Gpr, imm: i32) {
        self.rex(true, dst as u8, 0, src as u8, false);
        self.byte(0x69);
        self.modrm_rr(dst as u8, src as u8);
        self.bytes(&imm.to_le_bytes());
    }

    /// Unsigned `rdx:rax = rax * r`.
    pub fn mul(&mut self, r: Gpr) {
        self.group3(4, r);
    }

    /// Unary group 3 (`F7 /ext`): `mul` = 4, `not` = 2, `neg` = 3, `idiv` = 7.
    fn group3(&mut self, ext: u8, r: Gpr) {
        self.rex(true, 0, 0, r as u8, false);
        self.byte(0xF7);
        self.modrm_rr(ext, r as u8);
    }

    pub fn neg(&mut self, r: Gpr) {
        self.group3(3, r);
    }

    pub fn not(&mut self, r: Gpr) {
        self.group3(2, r);
    }

    /// `popcnt dst, src` (requires the POPCNT extension).
    pub fn popcnt(&mut self, dst: Gpr, src: Gpr) {
        self.byte(0xF3);
        self.rex(true, dst as u8, 0, src as u8, false);
        self.bytes(&[0x0F, 0xB8]);
        self.modrm_rr(dst as u8, src as u8);
    }

    /// `rdx:rax / r` → quotient in `rax`, remainder in `rdx` (after `cqo`).
    /// `rax, rdx = rdx:rax / r` unsigned, with `rdx` cleared first.
    pub fn zero_div(&mut self, r: Gpr) {
        self.alu_rr(Alu::Xor, Gpr::Rdx, Gpr::Rdx);
        self.group3(6, r);
    }

    pub fn cqo_idiv(&mut self, r: Gpr) {
        self.bytes(&[0x48, 0x99]);
        self.group3(7, r);
    }

    /// Shift by `cl`: `shl` = 4, `sar` = 7.
    pub fn shift_cl(&mut self, left: bool, r: Gpr) {
        self.rex(true, 0, 0, r as u8, false);
        self.byte(0xD3);
        self.modrm_rr(if left { 4 } else { 7 }, r as u8);
    }

    pub fn shift_ri(&mut self, left: bool, r: Gpr, n: u8) {
        self.rex(true, 0, 0, r as u8, false);
        self.byte(0xC1);
        self.modrm_rr(if left { 4 } else { 7 }, r as u8);
        self.byte(n);
    }

    /// `dst = cond ? 1 : 0`.
    pub fn setcc(&mut self, cond: Cond, dst: Gpr) {
        // setcc r8 (REX forces sil/dil/r8b..), then movzx r32, r8.
        self.rex(false, 0, 0, dst as u8, true);
        self.bytes(&[0x0F, 0x90 | cond as u8]);
        self.modrm_rr(0, dst as u8);
        self.rex(false, dst as u8, 0, dst as u8, true);
        self.bytes(&[0x0F, 0xB6]);
        self.modrm_rr(dst as u8, dst as u8);
    }

    pub fn cmov(&mut self, cond: Cond, dst: Gpr, src: Gpr) {
        self.rex(true, dst as u8, 0, src as u8, false);
        self.bytes(&[0x0F, 0x40 | cond as u8]);
        self.modrm_rr(dst as u8, src as u8);
    }

    pub fn push(&mut self, r: Gpr) {
        self.rex(false, 0, 0, r as u8, false);
        self.byte(0x50 + low(r as u8));
    }

    pub fn pop(&mut self, r: Gpr) {
        self.rex(false, 0, 0, r as u8, false);
        self.byte(0x58 + low(r as u8));
    }

    pub fn ret(&mut self) {
        self.byte(0xC3);
    }

    /// `call [m]`.
    /// `call rel32` with a zero displacement to be linked later; returns the displacement's
    /// offset.
    pub fn call_rel32(&mut self) -> usize {
        self.byte(0xE8);
        let at = self.code.len();
        self.bytes(&[0, 0, 0, 0]);
        at
    }

    pub fn call_mem(&mut self, m: Mem) {
        self.rex(false, 0, 0, m.base as u8, false);
        self.byte(0xFF);
        self.modrm_mem(2, m);
    }

    pub fn jmp(&mut self, target: Label) {
        self.byte(0xE9);
        self.fixups.push((self.code.len(), target));
        self.bytes(&[0; 4]);
    }

    pub fn jcc(&mut self, cond: Cond, target: Label) {
        self.bytes(&[0x0F, 0x80 | cond as u8]);
        self.fixups.push((self.code.len(), target));
        self.bytes(&[0; 4]);
    }

    // ----- SSE2 --------------------------------------------------------------------------

    fn sse_rr(&mut self, prefix: Option<u8>, w: bool, op: u8, reg: u8, rm: u8) {
        if let Some(p) = prefix {
            self.byte(p);
        }
        self.rex(w, reg, 0, rm, false);
        self.bytes(&[0x0F, op]);
        self.modrm_rr(reg, rm);
    }

    pub fn movsd_load(&mut self, dst: Xmm, m: Mem) {
        self.byte(0xF2);
        self.rex(false, dst.0, 0, m.base as u8, false);
        self.bytes(&[0x0F, 0x10]);
        self.modrm_mem(dst.0, m);
    }

    /// `movups xmm, [m]`: all 128 bits (callee-saved xmm registers on Win64).
    pub fn movups_load(&mut self, dst: Xmm, m: Mem) {
        self.rex(false, dst.0, 0, m.base as u8, false);
        self.bytes(&[0x0F, 0x10]);
        self.modrm_mem(dst.0, m);
    }

    pub fn movups_store(&mut self, m: Mem, src: Xmm) {
        self.rex(false, src.0, 0, m.base as u8, false);
        self.bytes(&[0x0F, 0x11]);
        self.modrm_mem(src.0, m);
    }

    /// `test [m], r` — touches memory without changing it (stack probes).
    pub fn test_mem(&mut self, m: Mem, r: Gpr) {
        self.rex(true, r as u8, 0, m.base as u8, false);
        self.byte(0x85);
        self.modrm_mem(r as u8, m);
    }

    pub fn movsd_store(&mut self, m: Mem, src: Xmm) {
        self.byte(0xF2);
        self.rex(false, src.0, 0, m.base as u8, false);
        self.bytes(&[0x0F, 0x11]);
        self.modrm_mem(src.0, m);
    }

    /// `dst = base[index]` extended to 64 bits for 1-, 2- or 4-byte elements: `movsx`/`movsxd`
    /// when `signed`, else `movzx` or a 32-bit `mov` (both zero the upper bits).
    pub fn load_idx_ext(&mut self, dst: Gpr, m: MemIdx, width: u8, signed: bool) {
        self.rex(signed, dst as u8, m.index as u8, m.base as u8, false);
        match (width, signed) {
            (1, true) => self.bytes(&[0x0F, 0xBE]),
            (2, true) => self.bytes(&[0x0F, 0xBF]),
            (_, true) => self.byte(0x63),
            (1, false) => self.bytes(&[0x0F, 0xB6]),
            (2, false) => self.bytes(&[0x0F, 0xB7]),
            (_, false) => self.byte(0x8B),
        }
        self.modrm_idx(dst as u8, m);
    }

    /// `base[index] = low bytes of src` for 1-, 2- or 4-byte elements.
    pub fn store_idx_narrow(&mut self, m: MemIdx, src: Gpr, width: u8) {
        if width == 2 {
            self.byte(0x66);
        }
        // A byte store from `spl`/`bpl`/`sil`/`dil` needs a REX prefix to name them.
        let force = width == 1 && (4..8).contains(&(src as u8));
        self.rex(false, src as u8, m.index as u8, m.base as u8, force);
        self.byte(if width == 1 { 0x88 } else { 0x89 });
        self.modrm_idx(src as u8, m);
    }

    pub fn movsd_load_idx(&mut self, dst: Xmm, m: MemIdx) {
        self.byte(0xF2);
        self.rex(false, dst.0, m.index as u8, m.base as u8, false);
        self.bytes(&[0x0F, 0x10]);
        self.modrm_idx(dst.0, m);
    }

    pub fn movsd_store_idx(&mut self, m: MemIdx, src: Xmm) {
        self.byte(0xF2);
        self.rex(false, src.0, m.index as u8, m.base as u8, false);
        self.bytes(&[0x0F, 0x11]);
        self.modrm_idx(src.0, m);
    }

    /// `movapd dst, src` (register copy).
    pub fn movapd(&mut self, dst: Xmm, src: Xmm) {
        if dst != src {
            self.sse_rr(Some(0x66), false, 0x28, dst.0, src.0);
        }
    }

    /// `movq xmm, r64`.
    pub fn movq_xr(&mut self, dst: Xmm, src: Gpr) {
        self.sse_rr(Some(0x66), true, 0x6E, dst.0, src as u8);
    }

    /// `movq r64, xmm`.
    pub fn movq_rx(&mut self, dst: Gpr, src: Xmm) {
        self.sse_rr(Some(0x66), true, 0x7E, src.0, dst as u8);
    }

    pub fn sse(&mut self, op: Sse, dst: Xmm, src: Xmm) {
        self.sse_rr(Some(0xF2), false, op as u8, dst.0, src.0);
    }

    /// `ucomisd a, b`: flags as if comparing `a` with `b`; unordered sets ZF, PF and CF.
    pub fn ucomisd(&mut self, a: Xmm, b: Xmm) {
        self.sse_rr(Some(0x66), false, 0x2E, a.0, b.0);
    }

    /// `cvtsi2sd xmm, r64`.
    pub fn cvtsi2sd(&mut self, dst: Xmm, src: Gpr) {
        self.sse_rr(Some(0xF2), true, 0x2A, dst.0, src as u8);
    }

    /// `cvttsd2si r64, xmm` (truncating; out of range gives `i64::MIN`).
    pub fn cvttsd2si(&mut self, dst: Gpr, src: Xmm) {
        self.sse_rr(Some(0xF2), true, 0x2C, dst as u8, src.0);
    }

    pub fn xorpd(&mut self, dst: Xmm, src: Xmm) {
        self.sse_rr(Some(0x66), false, 0x57, dst.0, src.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(f: impl FnOnce(&mut Asm)) -> Vec<u8> {
        let mut a = Asm::new();
        f(&mut a);
        a.finish()
    }

    /// Reference encodings (checked against GNU as).
    #[test]
    fn encodings_match_the_reference_assembler() {
        assert_eq!(enc(|a| a.mov_rr(Gpr::Rax, Gpr::R12)), [0x4C, 0x89, 0xE0]);
        assert_eq!(enc(|a| a.mov_rr(Gpr::R15, Gpr::Rcx)), [0x49, 0x89, 0xCF]);
        assert_eq!(
            enc(|a| a.load(
                Gpr::Rax,
                Mem {
                    base: Gpr::Rbp,
                    disp: -8
                }
            )),
            [0x48, 0x8B, 0x45, 0xF8]
        );
        assert_eq!(
            enc(|a| a.load(
                Gpr::Rcx,
                Mem {
                    base: Gpr::R12,
                    disp: 8
                }
            )),
            [0x49, 0x8B, 0x4C, 0x24, 0x08]
        );
        assert_eq!(
            enc(|a| a.load_idx(
                Gpr::Rax,
                MemIdx {
                    base: Gpr::Rcx,
                    index: Gpr::Rdx,
                    disp: 0,
                    scale: 8
                }
            )),
            [0x48, 0x8B, 0x04, 0xD1]
        );
        // Displacement sizes: none, 8 and 32 bits; `rbp`/`r13` bases always carry one.
        let idx = |base, disp| MemIdx {
            base,
            index: Gpr::Rdx,
            disp,
            scale: 8,
        };
        assert_eq!(
            enc(|a| a.load_idx(Gpr::Rax, idx(Gpr::R13, 0))),
            [0x49, 0x8B, 0x44, 0xD5, 0x00]
        );
        assert_eq!(
            enc(|a| a.load_idx(Gpr::Rax, idx(Gpr::Rcx, 4096))),
            [0x48, 0x8B, 0x84, 0xD1, 0x00, 0x10, 0x00, 0x00]
        );
        // Narrow list elements (reference: GNU as).
        let w = |base, index, scale| MemIdx {
            base,
            index,
            disp: 0,
            scale,
        };
        assert_eq!(
            enc(|a| a.load_idx_ext(Gpr::Rax, w(Gpr::Rcx, Gpr::Rdx, 1), 1, true)),
            [0x48, 0x0F, 0xBE, 0x04, 0x11]
        );
        assert_eq!(
            enc(|a| a.load_idx_ext(Gpr::R9, w(Gpr::Rbx, Gpr::R8, 2), 2, true)),
            [0x4E, 0x0F, 0xBF, 0x0C, 0x43]
        );
        assert_eq!(
            enc(|a| a.load_idx_ext(Gpr::Rsi, w(Gpr::R13, Gpr::Rdx, 4), 4, true)),
            [0x49, 0x63, 0x74, 0x95, 0x00]
        );
        assert_eq!(
            enc(|a| a.load_idx_ext(Gpr::Rax, w(Gpr::Rcx, Gpr::Rdx, 1), 1, false)),
            [0x0F, 0xB6, 0x04, 0x11]
        );
        assert_eq!(
            enc(|a| a.load_idx_ext(Gpr::R9, w(Gpr::Rbx, Gpr::R8, 2), 2, false)),
            [0x46, 0x0F, 0xB7, 0x0C, 0x43]
        );
        assert_eq!(
            enc(|a| a.load_idx_ext(Gpr::Rsi, w(Gpr::R13, Gpr::Rdx, 4), 4, false)),
            [0x41, 0x8B, 0x74, 0x95, 0x00]
        );
        assert_eq!(
            enc(|a| a.store_idx_narrow(w(Gpr::Rcx, Gpr::Rdx, 1), Gpr::Rsi, 1)),
            [0x40, 0x88, 0x34, 0x11]
        );
        assert_eq!(
            enc(|a| a.store_idx_narrow(w(Gpr::R12, Gpr::R9, 1), Gpr::Rax, 1)),
            [0x43, 0x88, 0x04, 0x0C]
        );
        assert_eq!(
            enc(|a| a.store_idx_narrow(w(Gpr::Rbx, Gpr::R8, 2), Gpr::R10, 2)),
            [0x66, 0x46, 0x89, 0x14, 0x43]
        );
        assert_eq!(
            enc(|a| a.store_idx_narrow(w(Gpr::Rcx, Gpr::Rdx, 4), Gpr::Rdi, 4)),
            [0x89, 0x3C, 0x91]
        );
        let mem = |base, disp| Mem { base, disp };
        assert_eq!(
            enc(|a| a.load(Gpr::Rax, mem(Gpr::R13, 0))),
            [0x49, 0x8B, 0x45, 0x00]
        );
        assert_eq!(
            enc(|a| a.load(Gpr::Rax, mem(Gpr::Rbx, 0))),
            [0x48, 0x8B, 0x03]
        );
        assert_eq!(
            enc(|a| a.load(Gpr::Rax, mem(Gpr::Rbx, -1024))),
            [0x48, 0x8B, 0x83, 0x00, 0xFC, 0xFF, 0xFF]
        );
        assert_eq!(
            enc(|a| a.alu_rr(Alu::Add, Gpr::Rax, Gpr::Rcx)),
            [0x48, 0x01, 0xC8]
        );
        assert_eq!(
            enc(|a| a.alu_rr(Alu::Cmp, Gpr::R8, Gpr::Rax)),
            [0x49, 0x39, 0xC0]
        );
        assert_eq!(
            enc(|a| a.imul_rr(Gpr::Rax, Gpr::R9)),
            [0x49, 0x0F, 0xAF, 0xC1]
        );
        assert_eq!(
            enc(|a| a.setcc(Cond::L, Gpr::Rax)),
            [0x40, 0x0F, 0x9C, 0xC0, 0x40, 0x0F, 0xB6, 0xC0]
        );
        assert_eq!(enc(|a| a.shift_cl(true, Gpr::Rax)), [0x48, 0xD3, 0xE0]);
        assert_eq!(
            enc(|a| a.cqo_idiv(Gpr::Rcx)),
            [0x48, 0x99, 0x48, 0xF7, 0xF9]
        );
        assert_eq!(enc(|a| a.push(Gpr::R12)), [0x41, 0x54]);
        assert_eq!(enc(|a| a.pop(Gpr::Rbx)), [0x5B]);
        assert_eq!(
            enc(|a| a.call_mem(Mem {
                base: Gpr::R12,
                disp: 16
            })),
            [0x41, 0xFF, 0x54, 0x24, 0x10]
        );
        assert_eq!(
            enc(|a| a.sse(Sse::Add, Xmm(0), Xmm(1))),
            [0xF2, 0x0F, 0x58, 0xC1]
        );
        assert_eq!(
            enc(|a| a.sse(Sse::Mul, Xmm(9), Xmm(1))),
            [0xF2, 0x44, 0x0F, 0x59, 0xC9]
        );
        assert_eq!(
            enc(|a| a.cvtsi2sd(Xmm(0), Gpr::Rax)),
            [0xF2, 0x48, 0x0F, 0x2A, 0xC0]
        );
        assert_eq!(
            enc(|a| a.cvttsd2si(Gpr::Rax, Xmm(1))),
            [0xF2, 0x48, 0x0F, 0x2C, 0xC1]
        );
        assert_eq!(
            enc(|a| a.movq_xr(Xmm(0), Gpr::Rax)),
            [0x66, 0x48, 0x0F, 0x6E, 0xC0]
        );
        assert_eq!(
            enc(|a| a.movq_rx(Gpr::Rax, Xmm(0))),
            [0x66, 0x48, 0x0F, 0x7E, 0xC0]
        );
        assert_eq!(enc(|a| a.ucomisd(Xmm(0), Xmm(1))), [0x66, 0x0F, 0x2E, 0xC1]);
        assert_eq!(
            enc(|a| a.movsd_load(
                Xmm(0),
                Mem {
                    base: Gpr::Rbp,
                    disp: -16
                }
            )),
            [0xF2, 0x0F, 0x10, 0x45, 0xF0]
        );
        assert_eq!(enc(|a| a.mov_ri(Gpr::Rax, 5)), [0xB8, 5, 0, 0, 0]);
        assert_eq!(
            enc(|a| a.mov_ri(Gpr::Rax, -1)),
            [0x48, 0xC7, 0xC0, 0xFF, 0xFF, 0xFF, 0xFF]
        );
        assert_eq!(
            enc(|a| a.alu_ri(Alu::Cmp, Gpr::Rax, 81)),
            [0x48, 0x83, 0xF8, 81]
        );
    }
}
