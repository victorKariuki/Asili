//! Target-independent shape of a block's code, shared by every code generator: which
//! comparisons are consumed straight by a branch or by conditional selects (so they set the
//! machine's condition flags once instead of materializing a 0/1 value), and in what order the
//! rest is emitted.

use super::ir::{BlockData, Inst, Term};

/// One unit of emission.
pub enum Step {
    /// Instruction `i` on its own.
    Inst(usize),
    /// Comparison `cmp` read only by the selects in `run` (copies may sit between them): set
    /// the flags once, then each select is one conditional move/select on them.
    FlagSelects {
        cmp: usize,
        run: std::ops::Range<usize>,
    },
}

pub struct Plan {
    /// The block's last instruction is a comparison read only by its branch: emit it as
    /// compare + conditional branch (it does not appear in `steps`).
    pub fused_branch: bool,
    pub steps: Vec<Step>,
}

fn compare_dst(inst: &Inst) -> Option<super::ir::VReg> {
    match inst {
        Inst::ICmp { dst, .. } | Inst::ICmpImm { dst, .. } | Inst::TestImm { dst, .. } => {
            Some(*dst)
        }
        _ => None,
    }
}

/// Plan `block` given how many times each virtual register is read in the function.
pub fn plan(block: &BlockData, uses: &[u32]) -> Plan {
    let fused_branch = matches!(
        (&block.term, block.insts.last()),
        (
            Term::Branch { cond, .. },
            Some(
                Inst::ICmp { dst, .. }
                | Inst::ICmpImm { dst, .. }
                | Inst::TestImm { dst, .. }
                | Inst::FCmp { dst, .. },
            ),
        ) if dst == cond && uses[cond.0 as usize] == 1
    );
    let body = if fused_branch {
        &block.insts[..block.insts.len() - 1]
    } else {
        &block.insts[..]
    };
    let mut steps = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let run = compare_dst(&body[i]).and_then(|c| {
            let len = body[i + 1..]
                .iter()
                .take_while(|x| match x {
                    Inst::Select { cond, dst, .. } => *cond == c && *dst != c,
                    Inst::Mov { dst, src } => *dst != c && *src != c,
                    _ => false,
                })
                .count();
            let selects = body[i + 1..i + 1 + len]
                .iter()
                .filter(|x| matches!(x, Inst::Select { .. }))
                .count();
            (selects > 0 && uses[c.0 as usize] as usize == selects).then_some(len)
        });
        match run {
            Some(len) => {
                steps.push(Step::FlagSelects {
                    cmp: i,
                    run: i + 1..i + 1 + len,
                });
                i += 1 + len;
            }
            None => {
                steps.push(Step::Inst(i));
                i += 1;
            }
        }
    }
    Plan {
        fused_branch,
        steps,
    }
}
