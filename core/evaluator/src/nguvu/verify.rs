//! IR verifier: checks that a [`Func`] is well formed — every register and block it names
//! exists, every instruction's operands and results have the register class it expects, and no
//! register is read on any path from the entry before it is written. Lowering's output and every
//! optimizer pass's output are checked in debug builds and tests (`nguvu::compile_with`), so an
//! optimization that breaks the IR fails at the pass that broke it instead of as wrong machine
//! code (docs/design/safety-critical-roadmap.md §4).

use super::ir::{Class, Func, Inst, Term, VReg};
use crate::numlist::Kind;

/// `Ok` when `func` is well formed, else what is wrong with it (in English: a compiler bug,
/// never shown to a program's author on its own).
pub fn verify(func: &Func) -> Result<(), String> {
    let regs = func.classes.len();
    let blocks = func.blocks.len();
    if blocks == 0 {
        return Err("no blocks".into());
    }
    if func.cold.len() != blocks {
        return Err(format!(
            "{} cold flags for {blocks} blocks",
            func.cold.len()
        ));
    }
    let reg = |v: VReg| -> Result<Class, String> {
        func.classes
            .get(v.0 as usize)
            .copied()
            .ok_or_else(|| format!("v{} out of range ({regs} registers)", v.0))
    };
    for &a in func.args {
        if reg(a)? != Class::Int {
            return Err(format!("argument v{} is not an integer register", a.0));
        }
    }
    for (b, block) in func.blocks.iter().enumerate() {
        let at = |i: usize| format!("block {b}, instruction {i}");
        for (i, inst) in block.insts.iter().enumerate() {
            for v in inst.uses().into_iter().chain(inst.defs()) {
                reg(v).map_err(|e| format!("{}: {e}", at(i)))?;
            }
            check_classes(func, inst).map_err(|e| format!("{}: {e} in {inst:?}", at(i)))?;
        }
        for s in block.term.successors() {
            if s.0 as usize >= blocks {
                return Err(format!("block {b} jumps to missing block {}", s.0));
            }
        }
        for v in block.term.uses() {
            reg(v).map_err(|e| format!("block {b} terminator: {e}"))?;
        }
        let expected = match block.term {
            Term::Jump(_) => None,
            Term::Branch { cond, .. } => Some((cond, Class::Int)),
            Term::Return(v) => Some((v, Class::Int)),
            Term::ReturnNum(v) => Some((v, Class::Float)),
        };
        if let Some((v, class)) = expected {
            if func.class(v) != class {
                return Err(format!("block {b} terminator reads v{} as {class:?}", v.0));
            }
        }
    }
    // A register live on entry is read on some path before anything writes it.
    let live = func.liveness();
    if let Some(v) = live.live_in[0].iter().find(|v| !func.args.contains(v)) {
        return Err(format!("v{} may be read before it is written", v.0));
    }
    Ok(())
}

fn check_classes(func: &Func, inst: &Inst) -> Result<(), String> {
    use Class::{Float as F, Int as I};
    let want = |v: VReg, c: Class| {
        if func.class(v) == c {
            Ok(())
        } else {
            Err(format!("v{} is {:?}, expected {c:?}", v.0, func.class(v)))
        }
    };
    match inst {
        Inst::IConst { dst, .. } | Inst::StackPointer { dst } | Inst::CallBuffer { dst } => {
            want(*dst, I)
        }
        Inst::FConst { dst, .. } => want(*dst, F),
        Inst::Mov { dst, src } => want(*src, func.class(*dst)),
        Inst::Int { dst, a, b, .. } | Inst::ICmp { dst, a, b, .. } => {
            want(*dst, I)?;
            want(*a, I)?;
            want(*b, I)
        }
        Inst::IntImm { dst, a, .. }
        | Inst::ICmpImm { dst, a, .. }
        | Inst::TestImm { dst, a, .. }
        | Inst::Neg { dst, src: a }
        | Inst::Not { dst, src: a }
        | Inst::Popcnt { dst, src: a } => {
            want(*dst, I)?;
            want(*a, I)
        }
        Inst::MulOverflow { dst, ovf, a, b } => {
            want(*dst, I)?;
            want(*ovf, I)?;
            want(*a, I)?;
            want(*b, I)
        }
        Inst::Float { dst, a, b, .. } => {
            want(*dst, F)?;
            want(*a, F)?;
            want(*b, F)
        }
        Inst::FCmp { dst, a, b, .. } => {
            want(*dst, I)?;
            want(*a, F)?;
            want(*b, F)
        }
        Inst::Select { dst, cond, a, b } => {
            want(*cond, I)?;
            want(*a, func.class(*dst))?;
            want(*b, func.class(*dst))
        }
        Inst::IntToFloat { dst, src } | Inst::BitsFloat { dst, src } => {
            want(*dst, F)?;
            want(*src, I)
        }
        Inst::FloatToInt { dst, src } | Inst::FloatBits { dst, src } => {
            want(*dst, I)?;
            want(*src, F)
        }
        Inst::Load { base, .. } | Inst::Store { base, .. } => want(*base, I),
        Inst::LoadIndex {
            dst,
            base,
            index,
            kind,
        }
        | Inst::StoreIndex {
            src: dst,
            base,
            index,
            kind,
        } => {
            want(*base, I)?;
            want(*index, I)?;
            want(*dst, if *kind == Kind::F64 { F } else { I })
        }
        Inst::Call { table, .. } => want(*table, I),
        Inst::CallDirect { dst, result, .. } => {
            want(*dst, I)?;
            want(*result, F)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nguvu::ir::{BlockData, IntOp, ENTRY_ARGS};

    /// `v4 = 1; v5 = v4 + v4; return v5` over the four entry arguments.
    fn good() -> Func {
        let mut classes = vec![Class::Int; 6];
        classes[4] = Class::Int;
        Func {
            classes,
            blocks: vec![BlockData {
                insts: vec![
                    Inst::IConst {
                        dst: VReg(4),
                        value: 1,
                    },
                    Inst::Int {
                        op: IntOp::Add,
                        dst: VReg(5),
                        a: VReg(4),
                        b: VReg(4),
                    },
                ],
                term: Term::Return(VReg(5)),
            }],
            cold: vec![false],
            call_buffer: 0,
            args: ENTRY_ARGS,
        }
    }

    // Verifies: REQ-COMP-1
    #[test]
    fn malformed_functions_are_rejected() {
        assert_eq!(verify(&good()), Ok(()));

        let mut f = good();
        f.blocks[0].insts.remove(0); // v4 read, never written
        assert!(verify(&f)
            .unwrap_err()
            .contains("read before it is written"));

        let mut f = good();
        f.classes[5] = Class::Float; // an integer add into a float register
        assert!(verify(&f).unwrap_err().contains("expected Int"));

        let mut f = good();
        f.blocks[0].term = Term::Jump(crate::nguvu::ir::Block(7));
        assert!(verify(&f).unwrap_err().contains("missing block"));

        let mut f = good();
        f.blocks[0].term = Term::Return(VReg(99));
        assert!(verify(&f).unwrap_err().contains("out of range"));

        let mut f = good();
        f.cold.clear();
        assert!(verify(&f).is_err());
    }
}
