//! Checks a loaded program before anything runs it: native code reaches numeric and list
//! registers by address, so a damaged or hand-edited `.asb` whose instructions name a register,
//! jump target or function that does not exist must be rejected at load time — it could
//! otherwise read or write outside a frame (docs/design/safety-critical-roadmap.md §4).
//! Generic (`vals`) registers and constants are reached through checked indexing in the host,
//! so a bad index there is a runtime error, not memory corruption; they are checked here too.

// Runtime code never panics on its own (see docs/design/safety-critical-roadmap.md §3).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

use crate::bytecode::{BytecodeFunc, BytecodeProgram, Opcode, Operand, Reg, Ty};

/// `Ok` when every function's instructions stay within its own registers, code and the
/// program's functions and constants; else what is wrong, in Swahili (a user sees it when a
/// `kilele` file is damaged).
pub(crate) fn verify(program: &BytecodeProgram) -> Result<(), String> {
    let functions = program.functions.len();
    for &index in [program.init, program.safe_state].iter().flatten() {
        if index as usize >= functions {
            return Err(format!("kazi {index} haipo"));
        }
    }
    for f in &program.functions {
        verify_function(program, f).map_err(|e| format!("kazi '{}': {e}", f.name))?;
    }
    Ok(())
}

fn verify_function(program: &BytecodeProgram, f: &BytecodeFunc) -> Result<(), String> {
    let num = |r: Reg| check(r, f.num_regs, "ya namba");
    let list = |r: Reg| check(r, f.list_regs, "ya orodha");
    let val = |r: Reg| check(r, f.val_regs, "ya thamani");
    let operand = |o: &Operand| match o.ty {
        Ty::Num | Ty::Bool => num(o.reg),
        Ty::List => list(o.reg),
        _ => val(o.reg),
    };
    for p in &f.params {
        operand(p)?;
    }
    for &(r, _) in &f.num_consts {
        num(r)?;
    }
    let len = f.code.len();
    for (pc, op) in f.code.iter().enumerate() {
        let at = |e: String| format!("agizo {pc}: {e}");
        for r in crate::nguvu::lower::reads(op)
            .into_iter()
            .chain(crate::nguvu::lower::writes(op))
        {
            num(r).map_err(at)?;
        }
        for r in list_operands(op) {
            list(r).map_err(at)?;
        }
        if let Some(t) = crate::native::jump_target(op) {
            if t >= len {
                return Err(at(format!("ruka hadi agizo {t} lisilokuwepo")));
            }
        }
        match op {
            Opcode::Call(call) => {
                let callee = program
                    .functions
                    .get(call.function as usize)
                    .ok_or_else(|| at(format!("kazi {} haipo", call.function)))?;
                if callee.params.len() != call.args.len() {
                    return Err(at(format!(
                        "kazi '{}' inapewa hoja zisizo sahihi",
                        callee.name
                    )));
                }
                for a in call.args.iter().chain([&call.dst]) {
                    operand(a).map_err(at)?;
                }
            }
            Opcode::ConstVal { k, .. } if *k as usize >= program.constants.len() => {
                return Err(at(format!("thabiti {k} haipo")));
            }
            Opcode::Return { src } => operand(src).map_err(at)?,
            Opcode::ListMethod(call) => {
                for &r in &call.args {
                    val(r).map_err(at)?;
                }
                operand(&call.dst).map_err(at)?;
            }
            Opcode::Line { binds, .. } => {
                for (_, o) in binds.iter() {
                    operand(o).map_err(at)?;
                }
            }
            _ => {}
        }
    }
    // Control must not run off the end of the code.
    match f.code.last() {
        Some(Opcode::Jump { .. } | Opcode::Return { .. } | Opcode::ReturnTupu) => Ok(()),
        _ => Err("msimbo hauishii kwa kurudi".into()),
    }
}

fn check(r: Reg, count: u32, bank: &str) -> Result<(), String> {
    if r < count {
        Ok(())
    } else {
        Err(format!("rejista {bank} {r} nje ya mipaka ({count})"))
    }
}

/// List registers an instruction names (native code reaches them by address).
fn list_operands(op: &Opcode) -> Vec<Reg> {
    let mut regs = crate::native::list_writes(op);
    match op {
        Opcode::ListMethod(call) => regs.push(call.list),
        Opcode::ListGet { list, .. }
        | Opcode::ListGetTokeo { list, .. }
        | Opcode::ListSet { list, .. }
        | Opcode::ListLen { list, .. } => regs.push(*list),
        Opcode::ListMov { src, .. } | Opcode::ListToVal { src, .. } => regs.push(*src),
        _ => {}
    }
    regs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program() -> BytecodeProgram {
        let source = "kazi pili(x: Namba) -> Namba {
                ikiwa x < 1 { rejesha 0 }
                rejesha pili(x - 1) + 2
            }
            kazi t() -> Orodha<Namba> {
                weka r: Orodha<Namba> = []
                kwa i kutoka 0 hadi 3 { r.ongeza(pili(i)) }
                rejesha r
            }";
        let tokens = asili_lexer::tokenize(source).expect("tokenize");
        let module = asili_parser::parse_tokens(&tokens).expect("parse");
        crate::bytecode::compile_module_explained(&module).expect("compile")
    }

    fn function(p: &mut BytecodeProgram, name: &str) -> usize {
        p.functions
            .iter()
            .position(|f| f.name == name)
            .expect("function")
    }

    // Verifies: REQ-COMP-4
    #[test]
    fn what_the_compiler_emits_passes() {
        assert_eq!(verify(&program()), Ok(()));
    }

    // Verifies: REQ-COMP-4
    #[test]
    fn damaged_programs_are_rejected() {
        let mut p = program();
        let t = function(&mut p, "t");
        p.functions[t].num_regs = 0;
        assert!(verify(&p).unwrap_err().contains("rejista ya namba"));

        let mut p = program();
        p.functions[t].list_regs = 0;
        assert!(verify(&p).unwrap_err().contains("rejista ya orodha"));

        let mut p = program();
        let len = p.functions[t].code.len() as u32;
        let jump = p.functions[t]
            .code
            .iter_mut()
            .find_map(|op| match op {
                Opcode::JumpIfNot { target, .. } | Opcode::Jump { target } => Some(target),
                _ => None,
            })
            .expect("a loop jump");
        *jump = len + 5;
        assert!(verify(&p).unwrap_err().contains("lisilokuwepo"));

        let mut p = program();
        for op in p.functions.iter_mut().flat_map(|f| &mut f.code) {
            if let Opcode::Call(call) = op {
                call.function = 99;
            }
        }
        assert!(verify(&p).unwrap_err().contains("kazi 99 haipo"));

        let mut p = program();
        if let Some(last) = p.functions[t].code.last_mut() {
            *last = Opcode::Mov { dst: 0, src: 0 };
        }
        assert!(verify(&p).unwrap_err().contains("hauishii"));
    }

    // Verifies: REQ-COMP-4
    #[test]
    fn a_damaged_kilele_file_does_not_load() {
        let mut p = program();
        let t = function(&mut p, "t");
        p.functions[t].num_regs = 1;
        let bytes = crate::asb::emit_bytecode_bytes(&p, "");
        let err = crate::asb::load_asb_bytecode(&bytes)
            .unwrap_err()
            .to_string();
        assert!(err.contains("kilele kimeharibika"), "{err}");
    }
}
