//! IR → x86-64 (System V calling convention).
//!
//! Frame layout: `rbp` frame pointer, callee-saved `rbx`, `r12`–`r15` pushed; the incoming
//! `rt`, `vm`, `frame`, `nums` arguments live in `r12`–`r15` for the whole call, and every other
//! virtual register has an 8-byte stack slot below the saved registers. Instructions load their
//! operands into scratch registers, compute, and store the result back.

use super::ir::{Class, FCond, FloatOp, Func, ICond, Inst, IntOp, Term, VReg};
use super::x64::{Alu, Asm, Cond, Gpr, Mem, MemIdx, Sse, Xmm};

const PINNED: [Gpr; 4] = [Gpr::R12, Gpr::R13, Gpr::R14, Gpr::R15];
const INT_ARGS: [Gpr; 6] = [Gpr::Rdi, Gpr::Rsi, Gpr::Rdx, Gpr::Rcx, Gpr::R8, Gpr::R9];
/// Bytes pushed below `rbp`: rbx, r12, r13, r14, r15.
const SAVED: i32 = 40;

struct Gen<'f> {
    asm: Asm,
    func: &'f Func,
}

fn slot(v: VReg) -> Mem {
    Mem {
        base: Gpr::Rbp,
        disp: -SAVED - 8 * (v.0 as i32 - 3),
    }
}

/// Machine code for `func`, position independent (runtime calls go through the `rt` table).
pub fn generate(func: &Func) -> Vec<u8> {
    let mut g = Gen {
        asm: Asm::new(),
        func,
    };
    let slots = func.classes.len().saturating_sub(4) as i32;
    // After `push rbp` the stack is 16-aligned; five more pushes leave it at 8 mod 16, so the
    // slot area must be 8 mod 16 for calls to see an aligned stack.
    let mut frame = 8 * slots;
    if frame % 16 != 8 {
        frame += 8;
    }
    let a = &mut g.asm;
    a.push(Gpr::Rbp);
    a.mov_rr(Gpr::Rbp, Gpr::Rsp);
    for r in [Gpr::Rbx, Gpr::R12, Gpr::R13, Gpr::R14, Gpr::R15] {
        a.push(r);
    }
    a.alu_ri(Alu::Sub, Gpr::Rsp, frame);
    for (i, r) in PINNED.iter().enumerate() {
        a.mov_rr(*r, INT_ARGS[i]);
    }
    let labels: Vec<_> = (0..func.blocks.len()).map(|_| g.asm.new_label()).collect();
    for (i, block) in func.blocks.iter().enumerate() {
        g.asm.bind(labels[i]);
        for inst in &block.insts {
            g.inst(inst);
        }
        let next = labels.get(i + 1).copied();
        match &block.term {
            Term::Jump(t) => {
                if Some(labels[t.0 as usize]) != next {
                    g.asm.jmp(labels[t.0 as usize]);
                }
            }
            Term::Branch { cond, then_, else_ } => {
                g.load_int(Gpr::Rax, *cond);
                g.asm.alu_rr(Alu::Test, Gpr::Rax, Gpr::Rax);
                let (t, e) = (labels[then_.0 as usize], labels[else_.0 as usize]);
                if Some(t) == next {
                    g.asm.jcc(Cond::E, e);
                } else {
                    g.asm.jcc(Cond::Ne, t);
                    if Some(e) != next {
                        g.asm.jmp(e);
                    }
                }
            }
            Term::Return(v) => {
                g.load_int(Gpr::Rax, *v);
                g.epilogue();
            }
        }
    }
    g.asm.finish()
}

impl<'f> Gen<'f> {
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

    fn load_int(&mut self, dst: Gpr, v: VReg) {
        if v.0 < 4 {
            self.asm.mov_rr(dst, PINNED[v.0 as usize]);
        } else {
            self.asm.load(dst, slot(v));
        }
    }

    fn store_int(&mut self, v: VReg, src: Gpr) {
        assert!(v.0 >= 4, "arguments are read-only");
        self.asm.store(slot(v), src);
    }

    fn load_f(&mut self, dst: Xmm, v: VReg) {
        self.asm.movsd_load(dst, slot(v));
    }

    fn store_f(&mut self, v: VReg, src: Xmm) {
        self.asm.movsd_store(slot(v), src);
    }

    fn inst(&mut self, inst: &Inst) {
        use Gpr::*;
        let x0 = Xmm(0);
        let x1 = Xmm(1);
        match inst {
            Inst::IConst { dst, value } => {
                self.asm.mov_ri(Rax, *value);
                self.store_int(*dst, Rax);
            }
            Inst::FConst { dst, value } => {
                self.asm.mov_ri(Rax, value.to_bits() as i64);
                self.store_int(*dst, Rax);
            }
            // Slots are untyped 8-byte cells, so copies and bit reinterpretations are moves.
            Inst::Mov { dst, src }
            | Inst::FloatBits { dst, src }
            | Inst::BitsFloat { dst, src } => {
                self.load_int(Rax, *src);
                self.store_int(*dst, Rax);
            }
            Inst::Int { op, dst, a, b } => {
                self.load_int(Rax, *a);
                self.load_int(Rcx, *b);
                match op {
                    IntOp::Add => self.asm.alu_rr(Alu::Add, Rax, Rcx),
                    IntOp::Sub => self.asm.alu_rr(Alu::Sub, Rax, Rcx),
                    IntOp::And => self.asm.alu_rr(Alu::And, Rax, Rcx),
                    IntOp::Or => self.asm.alu_rr(Alu::Or, Rax, Rcx),
                    IntOp::Xor => self.asm.alu_rr(Alu::Xor, Rax, Rcx),
                    IntOp::Mul => self.asm.imul_rr(Rax, Rcx),
                    IntOp::Shl => self.asm.shift_cl(true, Rax),
                    IntOp::Sar => self.asm.shift_cl(false, Rax),
                    IntOp::SDiv => self.asm.cqo_idiv(Rcx),
                    IntOp::SRem => {
                        self.asm.cqo_idiv(Rcx);
                        self.asm.mov_rr(Rax, Rdx);
                    }
                }
                self.store_int(*dst, Rax);
            }
            Inst::Neg { dst, src } => {
                self.load_int(Rax, *src);
                self.asm.neg(Rax);
                self.store_int(*dst, Rax);
            }
            Inst::Not { dst, src } => {
                self.load_int(Rax, *src);
                self.asm.not(Rax);
                self.store_int(*dst, Rax);
            }
            Inst::MulOverflow { dst, ovf, a, b } => {
                self.load_int(Rax, *a);
                self.load_int(Rcx, *b);
                self.asm.imul_rr(Rax, Rcx);
                self.asm.setcc(Cond::O, Rdx);
                self.store_int(*dst, Rax);
                self.store_int(*ovf, Rdx);
            }
            Inst::Float { op, dst, a, b } => {
                self.load_f(x0, *a);
                self.load_f(x1, *b);
                let sse = match op {
                    FloatOp::Add => Sse::Add,
                    FloatOp::Sub => Sse::Sub,
                    FloatOp::Mul => Sse::Mul,
                    FloatOp::Div => Sse::Div,
                };
                self.asm.sse(sse, x0, x1);
                self.store_f(*dst, x0);
            }
            Inst::ICmp { cond, dst, a, b } => {
                self.load_int(Rax, *a);
                self.load_int(Rcx, *b);
                self.asm.alu_rr(Alu::Cmp, Rax, Rcx);
                let cc = match cond {
                    ICond::Eq => Cond::E,
                    ICond::Ne => Cond::Ne,
                    ICond::Lt => Cond::L,
                    ICond::Le => Cond::Le,
                    ICond::Gt => Cond::G,
                    ICond::Ge => Cond::Ge,
                    ICond::Ult => Cond::B,
                    ICond::Ule => Cond::Be,
                };
                self.asm.setcc(cc, Rax);
                self.store_int(*dst, Rax);
            }
            Inst::FCmp { cond, dst, a, b } => {
                self.load_f(x0, *a);
                self.load_f(x1, *b);
                match cond {
                    // `a < b` ⇔ `b > a`: `ucomisd b, a` + `above` (false when unordered).
                    FCond::Olt => {
                        self.asm.ucomisd(x1, x0);
                        self.asm.setcc(Cond::A, Rax);
                    }
                    FCond::Ole => {
                        self.asm.ucomisd(x1, x0);
                        self.asm.setcc(Cond::Ae, Rax);
                    }
                    FCond::Ogt => {
                        self.asm.ucomisd(x0, x1);
                        self.asm.setcc(Cond::A, Rax);
                    }
                    FCond::Oge => {
                        self.asm.ucomisd(x0, x1);
                        self.asm.setcc(Cond::Ae, Rax);
                    }
                    FCond::Oeq => {
                        self.asm.ucomisd(x0, x1);
                        self.asm.setcc(Cond::E, Rax);
                        self.asm.setcc(Cond::Np, Rcx);
                        self.asm.alu_rr(Alu::And, Rax, Rcx);
                    }
                    FCond::Une => {
                        self.asm.ucomisd(x0, x1);
                        self.asm.setcc(Cond::Ne, Rax);
                        self.asm.setcc(Cond::P, Rcx);
                        self.asm.alu_rr(Alu::Or, Rax, Rcx);
                    }
                }
                self.store_int(*dst, Rax);
            }
            Inst::Select { dst, cond, a, b } => {
                self.load_int(Rax, *b);
                self.load_int(Rcx, *a);
                self.load_int(Rdx, *cond);
                self.asm.alu_rr(Alu::Test, Rdx, Rdx);
                self.asm.cmov(Cond::Ne, Rax, Rcx);
                self.store_int(*dst, Rax);
            }
            Inst::IntToFloat { dst, src } => {
                self.load_int(Rax, *src);
                self.asm.xorpd(x0, x0); // break the false dependency of cvtsi2sd
                self.asm.cvtsi2sd(x0, Rax);
                self.store_f(*dst, x0);
            }
            Inst::FloatToInt { dst, src } => {
                self.load_f(x0, *src);
                self.asm.cvttsd2si(Rax, x0);
                self.store_int(*dst, Rax);
            }
            Inst::Load { dst, base, offset } => {
                self.load_int(Rax, *base);
                self.asm.load(
                    Rcx,
                    Mem {
                        base: Rax,
                        disp: *offset,
                    },
                );
                self.store_int(*dst, Rcx);
            }
            Inst::Store { src, base, offset } => {
                self.load_int(Rax, *base);
                self.load_int(Rcx, *src);
                self.asm.store(
                    Mem {
                        base: Rax,
                        disp: *offset,
                    },
                    Rcx,
                );
            }
            Inst::LoadIndex { dst, base, index } => {
                self.load_int(Rax, *base);
                self.load_int(Rcx, *index);
                self.asm.load_idx(
                    Rdx,
                    MemIdx {
                        base: Rax,
                        index: Rcx,
                        disp: 0,
                    },
                );
                self.store_int(*dst, Rdx);
            }
            Inst::StoreIndex { src, base, index } => {
                self.load_int(Rax, *base);
                self.load_int(Rcx, *index);
                self.load_int(Rdx, *src);
                self.asm.store_idx(
                    MemIdx {
                        base: Rax,
                        index: Rcx,
                        disp: 0,
                    },
                    Rdx,
                );
            }
            Inst::Call {
                target,
                args,
                dst,
                ret32,
            } => {
                let (mut ni, mut nf) = (0usize, 0u8);
                for arg in args {
                    match self.func.class(*arg) {
                        Class::Int => {
                            self.load_int(INT_ARGS[ni], *arg);
                            ni += 1;
                        }
                        Class::Float => {
                            self.load_f(Xmm(nf), *arg);
                            nf += 1;
                        }
                    }
                }
                self.asm.call_mem(Mem {
                    base: PINNED[0],
                    disp: 8 * (*target as i32),
                });
                if let Some(d) = dst {
                    match self.func.class(*d) {
                        Class::Int => {
                            if *ret32 {
                                self.asm.mov_rr32(Rax, Rax);
                            }
                            self.store_int(*d, Rax);
                        }
                        Class::Float => self.store_f(*d, x0),
                    }
                }
            }
        }
    }
}
