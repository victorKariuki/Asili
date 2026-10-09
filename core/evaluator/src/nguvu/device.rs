//! The Cortex-M device target: a program's strict (`#[salama]`) functions compiled to Thumb-2
//! ([`super::codegen_t32`]) as an ELF relocatable object a firmware links, plus a C header
//! declaring them (`pata jenga --lengo cortex-m`).
//!
//! Each strict function `f` is exported as
//! `int32_t asili_f(const asili_f_hoja *hoja, double *matokeo)`: `hoja` holds its arguments in
//! order (a `Namba` or `Ukweli` as a `double`, an `Orodha<Namba>` as `asili_orodha`, a pointer
//! to the caller's `double` array and its length, eight bytes each); the result is stored at
//! `matokeo`, and the return value is 0, or non-zero when the call failed (an index out of range,
//! say) — the firmware's cue to enter its safe state. The export is a small thunk that builds
//! the function's register frame on the stack and calls its native code; strict functions call
//! each other directly. The code calls into `asili-kifaa` (`driver/kifaa`), the device runtime
//! (list access, `fmod`, `pow`, conversions), and the EABI 64-bit division helpers, both
//! resolved when the firmware is linked.
//!
//! Only what a device can run without a host is accepted: numbers, truth values and the caller's
//! numeric lists, with no allocation — anything else in a strict function is a build error.

use super::codegen_t32::{self, Code};
use super::ir::{Func, Inst, RtFn};
use super::t32::{self, Asm, Cond, Reloc, SP};
use super::{lower, opt, verify};
use crate::bytecode::{BytecodeFunc, BytecodeProgram, Opcode, Operand, Ty};
use crate::numlist::Kind;

/// A built device object and its header.
pub struct Kifaa {
    /// ELF32 relocatable object (ARM, EABI5, hard float).
    pub object: Vec<u8>,
    /// C declarations of the exports.
    pub header: String,
    /// Exported function names, in order.
    pub exports: Vec<String>,
}

/// Status kinds in the high word of a function's return value (`native::STATUS_*`).
const STATUS_RETURN: u32 = 3;

/// Build `functions` (strict functions of `program`, by index) for Cortex-M; `jina` names the
/// header guard. Errors name the function and why it cannot run on a device.
pub fn build(program: &BytecodeProgram, functions: &[usize], jina: &str) -> Result<Kifaa, String> {
    for &i in functions {
        let f = &program.functions[i];
        signature(f).map_err(|e| kosa(f, &e))?;
    }
    // A strict function calling one that takes a list would go through the host: on a device
    // the callee's body is inlined instead (strict code has no recursion, so this ends).
    let mut inlined = program.clone();
    for &i in functions {
        inline_list_calls(&mut inlined, i).map_err(|e| kosa(&program.functions[i], &e))?;
    }
    crate::bytecode_verify::verify(&inlined).map_err(|e| format!("kosa la ndani: {e}"))?;
    let program = &inlined;
    let direct = super::direct_entries(program);
    // Ordinary entries of the exports, then the direct entries they reach.
    let mut units: Vec<(usize, bool, Code)> = Vec::new();
    let mut queue: Vec<(usize, bool)> = functions.iter().map(|&i| (i, false)).collect();
    let mut seen: std::collections::BTreeSet<(usize, bool)> = queue.iter().copied().collect();
    while let Some((index, entry_direct)) = queue.pop() {
        let f = &program.functions[index];
        let ctx = lower::Ctx {
            program,
            direct: &direct,
            entry_direct,
        };
        let code = compile(index, &ctx).map_err(|e| kosa(f, &e))?;
        for &(_, callee) in &code.calls {
            if seen.insert((callee as usize, true)) {
                queue.push((callee as usize, true));
            }
        }
        units.push((index, entry_direct, code));
    }
    units.sort_by_key(|(i, d, _)| (*d, *i));

    // Layout: each export's thunk, then every function's code.
    let mut text: Vec<u8> = Vec::new();
    let mut relocs: Vec<(usize, String, Reloc)> = Vec::new();
    let mut symbols: Vec<(String, usize, usize)> = Vec::new();
    let mut thunk_calls: Vec<(usize, usize)> = Vec::new(); // (bl offset, export index)
    for &i in functions {
        let f = &program.functions[i];
        align(&mut text);
        let start = text.len();
        let (bytes, call_at) = thunk(f)?;
        thunk_calls.push((start + call_at, i));
        text.extend_from_slice(&bytes);
        symbols.push((format!("asili_{}", f.name), start, bytes.len()));
    }
    let mut placed: std::collections::BTreeMap<(usize, bool), usize> = Default::default();
    for (index, entry_direct, code) in &units {
        align(&mut text);
        let base = text.len();
        placed.insert((*index, *entry_direct), base);
        text.extend_from_slice(&code.bytes);
        for (at, symbol, kind) in &code.relocs {
            relocs.push((base + at, symbol.clone(), *kind));
        }
    }
    for (index, entry_direct, code) in &units {
        let base = placed[&(*index, *entry_direct)];
        for &(at, callee) in &code.calls {
            let target = placed[&(callee as usize, true)];
            t32::link_bl(&mut text, base + at, target)?;
        }
    }
    for &(at, i) in &thunk_calls {
        t32::link_bl(&mut text, at, placed[&(i, false)])?;
    }
    Ok(Kifaa {
        object: elf_object(&text, &symbols, &relocs),
        header: header(program, functions, jina),
        exports: functions
            .iter()
            .map(|&i| program.functions[i].name.clone())
            .collect(),
    })
}

/// Inline, into function `caller`, every call to a function taking a list, until none is left.
fn inline_list_calls(program: &mut BytecodeProgram, caller: usize) -> Result<(), String> {
    for _ in 0..1000 {
        let found =
            program.functions[caller]
                .code
                .iter()
                .enumerate()
                .find_map(|(pc, op)| match op {
                    Opcode::Call(call)
                        if program.functions[call.function as usize]
                            .params
                            .iter()
                            .any(|p| p.ty == Ty::List) =>
                    {
                        Some((pc, (**call).clone()))
                    }
                    _ => None,
                });
        let Some((pc, call)) = found else {
            return Ok(());
        };
        let callee = program.functions[call.function as usize].clone();
        inline_at(&mut program.functions[caller], pc, &call, &callee)?;
    }
    Err("miito iliyo ndani ya miito ni mingi mno".into())
}

/// Replace the call at `pc` of `f` with `callee`'s body: its registers renamed past `f`'s,
/// its parameters bound to the arguments (a list parameter shares the caller's list, so the
/// callee must not write it), and each return a move into the call's destination and a jump
/// past the body.
fn inline_at(
    f: &mut BytecodeFunc,
    pc: usize,
    call: &crate::bytecode::CallOp,
    callee: &BytecodeFunc,
) -> Result<(), String> {
    let (num_base, list_base) = (f.num_regs, f.list_regs);
    let num = |r: u32| r + num_base;
    let written_lists: std::collections::BTreeSet<u32> = callee
        .code
        .iter()
        .flat_map(crate::native::list_writes)
        .chain(callee.code.iter().filter_map(|op| match op {
            Opcode::ListSet { list, .. } => Some(*list),
            _ => None,
        }))
        .collect();
    let mut list_map: std::collections::HashMap<u32, u32> = Default::default();
    let mut prologue = Vec::new();
    for (p, arg) in callee.params.iter().zip(&call.args) {
        match p.ty {
            Ty::List => {
                if written_lists.contains(&p.reg) {
                    return Err(format!(
                        "inaita '{}', inayoandika kwenye orodha iliyopokelewa",
                        callee.name
                    ));
                }
                list_map.insert(p.reg, arg.reg);
            }
            Ty::Num | Ty::Bool => prologue.push(Opcode::Mov {
                dst: num(p.reg),
                src: arg.reg,
            }),
            Ty::Val => return Err(format!("inaita '{}' kwa hoja isiyo ya namba", callee.name)),
        }
    }
    let list = |r: u32| list_map.get(&r).copied().unwrap_or(r + list_base);
    // The callee's constants: never-written ones join the caller's; others are set on entry
    // (each call starts with them), from a fresh constant register.
    let callee_writes: std::collections::BTreeSet<u32> =
        callee.code.iter().flat_map(lower::writes).collect();
    let mut extra_regs = 0;
    for &(r, v) in &callee.num_consts {
        if callee_writes.contains(&r) {
            let k = num_base + callee.num_regs + extra_regs;
            extra_regs += 1;
            f.num_consts.push((k, v));
            prologue.push(Opcode::Mov {
                dst: num(r),
                src: k,
            });
        } else {
            f.num_consts.push((num(r), v));
        }
    }
    // New positions of the callee's instructions (a return becomes a move and a jump).
    let start = pc + prologue.len();
    let mut at = Vec::with_capacity(callee.code.len() + 1);
    let mut next = start;
    for op in &callee.code {
        at.push(next);
        next += match op {
            Opcode::Return { src } if matches!(src.ty, Ty::Num | Ty::Bool) => 2,
            _ => 1,
        };
    }
    at.push(next);
    let after = next;
    let target = |t: u32| at[t as usize] as u32;
    let operand = |o: &Operand| -> Result<Operand, String> {
        Ok(Operand {
            ty: o.ty,
            reg: match o.ty {
                Ty::Num | Ty::Bool => num(o.reg),
                Ty::List => list(o.reg),
                Ty::Val => return Err("hoja isiyo ya namba".into()),
            },
        })
    };
    let mut body = Vec::with_capacity(after - pc);
    body.extend(prologue);
    for op in &callee.code {
        use Opcode as O;
        let renamed = match op {
            O::Mov { dst, src } => O::Mov {
                dst: num(*dst),
                src: num(*src),
            },
            O::Add { dst, a, b } => O::Add {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::Sub { dst, a, b } => O::Sub {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::Mul { dst, a, b } => O::Mul {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::Div { dst, a, b } => O::Div {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::Rem { dst, a, b } => O::Rem {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::Pow { dst, a, b } => O::Pow {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::BitAnd { dst, a, b } => O::BitAnd {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::BitOr { dst, a, b } => O::BitOr {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::BitXor { dst, a, b } => O::BitXor {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::Shl { dst, a, b } => O::Shl {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::Shr { dst, a, b } => O::Shr {
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::Neg { dst, src } => O::Neg {
                dst: num(*dst),
                src: num(*src),
            },
            O::BitNot { dst, src } => O::BitNot {
                dst: num(*dst),
                src: num(*src),
            },
            O::Not { dst, src } => O::Not {
                dst: num(*dst),
                src: num(*src),
            },
            O::Floor { dst, src } => O::Floor {
                dst: num(*dst),
                src: num(*src),
            },
            O::Ceil { dst, src } => O::Ceil {
                dst: num(*dst),
                src: num(*src),
            },
            O::Trunc { dst, src } => O::Trunc {
                dst: num(*dst),
                src: num(*src),
            },
            O::Cmp { op, dst, a, b } => O::Cmp {
                op: *op,
                dst: num(*dst),
                a: num(*a),
                b: num(*b),
            },
            O::Jump { target: t } => O::Jump { target: target(*t) },
            O::JumpIfFalse { cond, target: t } => O::JumpIfFalse {
                cond: num(*cond),
                target: target(*t),
            },
            O::JumpIfTrue { cond, target: t } => O::JumpIfTrue {
                cond: num(*cond),
                target: target(*t),
            },
            O::JumpIfNot {
                op,
                a,
                b,
                target: t,
            } => O::JumpIfNot {
                op: *op,
                a: num(*a),
                b: num(*b),
                target: target(*t),
            },
            O::ForStep {
                ctr,
                end,
                target: t,
            } => O::ForStep {
                ctr: num(*ctr),
                end: num(*end),
                target: target(*t),
            },
            O::ListGet {
                dst,
                list: l,
                idx,
                mode,
            } => O::ListGet {
                dst: num(*dst),
                list: list(*l),
                idx: num(*idx),
                mode: *mode,
            },
            O::ListSet { list: l, idx, src } => O::ListSet {
                list: list(*l),
                idx: num(*idx),
                src: num(*src),
            },
            O::ListLen { dst, list: l } => O::ListLen {
                dst: num(*dst),
                list: list(*l),
            },
            O::ListMov { dst, src } => O::ListMov {
                dst: list(*dst),
                src: list(*src),
            },
            O::CheckDepth => O::CheckDepth,
            O::Call(c) => O::Call(Box::new(crate::bytecode::CallOp {
                function: c.function,
                args: c.args.iter().map(operand).collect::<Result<_, _>>()?,
                dst: if c.dst.ty == Ty::Val {
                    c.dst
                } else {
                    operand(&c.dst)?
                },
            })),
            O::Return { src } if matches!(src.ty, Ty::Num | Ty::Bool) => {
                if matches!(call.dst.ty, Ty::Num | Ty::Bool) {
                    body.push(O::Mov {
                        dst: call.dst.reg,
                        src: num(src.reg),
                    });
                } else {
                    body.push(O::Jump {
                        target: (body.len() + pc + 1) as u32,
                    });
                }
                O::Jump {
                    target: after as u32,
                }
            }
            O::ReturnTupu => O::Jump {
                target: after as u32,
            },
            other => {
                return Err(format!(
                    "agizo `{}` ndani ya '{}'",
                    other.name(),
                    callee.name
                ))
            }
        };
        body.push(renamed);
    }
    debug_assert_eq!(pc + body.len(), after);
    // The caller's jumps past the call move by the body's extra length.
    let grow = (body.len() as u32).saturating_sub(1);
    for op in f.code.iter_mut() {
        if let Opcode::Jump { target: t }
        | Opcode::JumpIfFalse { target: t, .. }
        | Opcode::JumpIfTrue { target: t, .. }
        | Opcode::JumpIfNot { target: t, .. }
        | Opcode::ForStep { target: t, .. } = op
        {
            if *t as usize > pc {
                *t += grow;
            }
        }
    }
    f.code.splice(pc..pc + 1, body);
    f.num_regs += callee.num_regs + extra_regs;
    f.list_regs += callee.list_regs;
    Ok(())
}

fn kosa(f: &BytecodeFunc, e: &str) -> String {
    format!(
        "kazi salama '{}' haiwezi kujengwa kwa Cortex-M: {e}",
        f.name
    )
}

fn align(text: &mut Vec<u8>) {
    while !text.len().is_multiple_of(4) {
        text.extend_from_slice(&0xBF00u16.to_le_bytes()); // nop
    }
}

/// The parameters and result a device export can pass.
fn signature(f: &BytecodeFunc) -> Result<(), String> {
    if !f
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("jina lake si jina halali la C".into());
    }
    for p in &f.params {
        if !matches!(p.ty, Ty::Num | Ty::Bool | Ty::List) {
            return Err("hoja zake ziwe Namba, Ukweli au Orodha<Namba> tu".into());
        }
    }
    Ok(())
}

/// Lower, optimize (without instructions a Cortex-M lacks) and generate one entry.
fn compile(index: usize, ctx: &lower::Ctx) -> Result<Code, String> {
    let function = &ctx.program.functions[index];
    let mut func =
        lower::lower(index, function, ctx).ok_or("haikuweza kutafsiriwa kuwa msimbo wa mashine")?;
    check_lists(&func)?;
    check_host_calls(&func, function)?;
    verify::verify(&func).map_err(|e| format!("kosa la ndani: {e}"))?;
    opt::optimize_with(&mut func, false).map_err(|e| format!("kosa la ndani: {e}"))?;
    verify::verify(&func).map_err(|e| format!("kosa la ndani: {e}"))?;
    codegen_t32::generate(&func)
}

/// Constants defined by `IConst` in `func` (by register).
fn iconsts(func: &Func) -> std::collections::HashMap<u32, i64> {
    func.blocks
        .iter()
        .flat_map(|b| &b.insts)
        .filter_map(|i| match i {
            Inst::IConst { dst, value } => Some((dst.0, *value)),
            _ => None,
        })
        .collect()
}

/// An instruction native code hands to the host (`exec`) has no host to run it on a device:
/// name it.
fn check_host_calls(func: &Func, function: &BytecodeFunc) -> Result<(), String> {
    let consts = iconsts(func);
    for inst in func.blocks.iter().flat_map(|b| &b.insts) {
        let Inst::Call { target, args, .. } = inst else {
            continue;
        };
        let pc = (*target == RtFn::Exec)
            .then(|| args.get(3).and_then(|p| consts.get(&p.0)))
            .flatten();
        let op = pc.and_then(|&pc| function.code.get(pc as usize));
        // Indexing runs natively; the host only raises its out-of-range error, which on a
        // device is the call failing.
        let error_path_only = matches!(op, Some(Opcode::ListGet { .. } | Opcode::ListSet { .. }));
        if codegen_t32::runtime_symbol(*target).is_some()
            && (*target != RtFn::Exec || error_path_only)
        {
            continue;
        }
        let what = match op {
            Some(Opcode::Call(_)) => "mwito wa kazi inayopokea orodha (kifaa huita moja kwa moja \
                 kazi za Namba na Ukweli tu)"
                .to_string(),
            Some(Opcode::MakeNumList { .. } | Opcode::ListRepeat { .. }) => {
                "kuunda orodha (kifaa hakitengi kumbukumbu)".to_string()
            }
            Some(op) => format!("agizo `{}`", op.name()),
            None => format!("huduma ya mwenyeji `{target:?}`"),
        };
        return Err(format!("inahitaji mwenyeji kwa {what}"));
    }
    Ok(())
}

/// Lists reach the device as the caller's `double` arrays: a function whose analysis wants one
/// held narrower (as integers) cannot use them in place.
fn check_lists(func: &Func) -> Result<(), String> {
    let consts = iconsts(func);
    for inst in func.blocks.iter().flat_map(|b| &b.insts) {
        if let Inst::Call {
            target: RtFn::ListPtr,
            args,
            ..
        } = inst
        {
            let want = args.get(2).and_then(|w| consts.get(&w.0)).copied();
            if want != Some(Kind::F64.code() as i64) {
                return Err("inahifadhi orodha kama namba kamili, si kama Namba za kifaa".into());
            }
        }
    }
    Ok(())
}

/// The export of `f`: builds `f`'s frame on the stack from `hoja` (r0), calls its ordinary
/// entry, and stores the result at `matokeo` (r1). Returns the code and the offset of the `bl`
/// to link to the entry.
fn thunk(f: &BytecodeFunc) -> Result<(Vec<u8>, usize), String> {
    let nums = 8 * f.num_regs;
    let lists = 8 * f.list_regs;
    let host = nums + lists; // depth, runtime table, stack limit: 24 bytes, all zero
    let size = (host + 24).div_ceil(8) * 8;
    let mut a = Asm::new();
    a.push(0x40F8); // r3-r7, lr (six words: `sp` stays 8-byte aligned)
    a.mov(4, 0);
    a.mov(5, 1);
    a.adjust_sp(size, true, 12);
    // Zero the whole frame: registers, list references, host fields.
    a.mov_imm(0, 0);
    a.mov_imm(1, 0);
    for off in (0..size).step_by(8) {
        store_pair(&mut a, 0, 1, off);
    }
    // Constants the registers start with.
    for &(reg, value) in &f.num_consts {
        let bits = value.to_bits();
        a.mov_imm(0, bits as u32);
        a.mov_imm(1, (bits >> 32) as u32);
        store_pair(&mut a, 0, 1, 8 * reg);
    }
    // Arguments, eight bytes each in `hoja`.
    for (i, p) in f.params.iter().enumerate() {
        let from = 8 * i as u32;
        if from > 1020 {
            return Err("ina hoja nyingi mno".into());
        }
        a.ldrd(0, 1, 4, from);
        let to = if p.ty == Ty::List {
            nums + 8 * p.reg
        } else {
            8 * p.reg
        };
        store_pair(&mut a, 0, 1, to);
    }
    // entry(runtime: unused on a device, host, frame = list references, nums)
    a.mov_imm(0, 0);
    a.add_any(1, SP, host);
    a.add_any(2, SP, nums);
    a.addw(3, SP, 0);
    let call_at = a.bl_placeholder();
    // r1 = status kind, r0 = the pc of the `Return` reached.
    let fail = a.new_label();
    let done = a.new_label();
    a.cmp_imm(1, STATUS_RETURN);
    a.b_cond(Cond::Ne, fail);
    for (pc, op) in f.code.iter().enumerate() {
        let Opcode::Return { src } = op else {
            continue;
        };
        if !matches!(src.ty, Ty::Num | Ty::Bool) {
            continue;
        }
        let next = a.new_label();
        a.mov_imm(12, pc as u32);
        a.cmp(0, 12);
        a.b_cond(Cond::Ne, next);
        let off = 8 * src.reg;
        if off <= 1020 {
            a.vldr(0, SP, off);
        } else {
            a.add_any(12, SP, off);
            a.vldr(0, 12, 0);
        }
        a.vstr(0, 5, 0);
        a.b(done);
        a.bind(next);
    }
    a.bind(done);
    a.mov_imm(0, 0);
    let out = a.new_label();
    a.b(out);
    a.bind(fail);
    a.mov(0, 1);
    a.bind(out);
    a.adjust_sp(size, false, 12);
    a.pop(0x80F8); // r3-r7, pc
    let (bytes, _) = a.finish()?;
    Ok((bytes, call_at))
}

fn store_pair(a: &mut Asm, lo: u8, hi: u8, off: u32) {
    if off <= 1020 {
        a.strd(lo, hi, SP, off);
    } else {
        a.add_any(12, SP, off);
        a.strd(lo, hi, 12, 0);
    }
}

/// C declarations of the exports.
fn header(program: &BytecodeProgram, functions: &[usize], jina: &str) -> String {
    let guard: String = jina
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    let mut h = format!(
        "/* {jina}.h: kazi salama za Asili zilizojengwa kwa Cortex-M (ARMv7E-M, fpv5-d16,\n \
         * hard float) na `pata jenga --lengo cortex-m`. Unganisha na {jina}-cortex-m.o na\n \
         * libasili_kifaa.a. Usihariri: faili hili hutengenezwa upya kila ujenzi.\n \
         *\n \
         * Kila kazi hurudisha 0 ikifaulu (matokeo yamewekwa), au namba isiyo sifuri ikishindwa\n \
         * (matokeo hayajawekwa): hapo programu iingie katika hali yake salama. Namba na Ukweli\n \
         * hupitishwa kama double (Ukweli: 0 au 1). */\n\n\
         #ifndef ASILI_{guard}_H\n#define ASILI_{guard}_H\n\n#include <stdint.h>\n\n\
         #ifndef ASILI_ORODHA\n#define ASILI_ORODHA\n\
         typedef struct {{\n    double *data;\n    uint32_t len;\n}} asili_orodha;\n#endif\n"
    );
    for &i in functions {
        let f = &program.functions[i];
        // The parameters' own names where the program kept its source (C-safe ones only).
        let names: Vec<String> = program
            .ast
            .as_ref()
            .and_then(|m| m.functions.iter().find(|g| g.name == f.name))
            .map(|g| g.params.iter().map(|p| p.name.to_string()).collect())
            .unwrap_or_default();
        let field = |k: usize| match names.get(k) {
            Some(n) if n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') => n.clone(),
            _ => format!("hoja{k}"),
        };
        let hoja = if f.params.is_empty() {
            "const void *hoja".to_string()
        } else {
            h.push_str("\ntypedef struct {\n");
            for (k, p) in f.params.iter().enumerate() {
                let ty = if p.ty == Ty::List {
                    "asili_orodha"
                } else {
                    "double"
                };
                h.push_str(&format!("    {ty} {};\n", field(k)));
            }
            h.push_str(&format!("}} asili_{}_hoja;\n", f.name));
            format!("const asili_{}_hoja *hoja", f.name)
        };
        h.push_str(&format!(
            "int32_t asili_{}({hoja}, double *matokeo);\n",
            f.name
        ));
    }
    h.push_str(&format!("\n#endif /* ASILI_{guard}_H */\n"));
    h
}

/// An ELF32 relocatable object for ARM (EABI5, hard float) holding `text`, with a global
/// Thumb function symbol for each export and REL relocations against undefined symbols.
fn elf_object(
    text: &[u8],
    exports: &[(String, usize, usize)],
    relocs: &[(usize, String, Reloc)],
) -> Vec<u8> {
    fn u16le(v: &mut Vec<u8>, x: u16) {
        v.extend_from_slice(&x.to_le_bytes());
    }
    fn u32le(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_le_bytes());
    }
    let mut strtab = vec![0u8];
    let name = |s: &str, tab: &mut Vec<u8>| -> u32 {
        let at = tab.len() as u32;
        tab.extend_from_slice(s.as_bytes());
        tab.push(0);
        at
    };
    // Symbols: null, the section, the `$t` mapping symbol (Thumb code from 0), exports, then
    // the undefined symbols the relocations name.
    let mut symtab: Vec<u8> = vec![0; 16];
    let sym = |tab: &mut Vec<u8>, st_name: u32, value: u32, size: u32, info: u8, shndx: u16| {
        u32le(tab, st_name);
        u32le(tab, value);
        u32le(tab, size);
        tab.push(info);
        tab.push(0);
        u16le(tab, shndx);
    };
    sym(&mut symtab, 0, 0, 0, 3, 1); // STT_SECTION, local
    let t = name("$t", &mut strtab);
    sym(&mut symtab, t, 0, 0, 0, 1);
    let first_global = 3u32;
    for (export, at, size) in exports {
        let n = name(export, &mut strtab);
        // STB_GLOBAL | STT_FUNC; a Thumb function's address has bit 0 set.
        sym(&mut symtab, n, *at as u32 | 1, *size as u32, 0x12, 1);
    }
    let mut undefined: Vec<String> = relocs.iter().map(|(_, s, _)| s.clone()).collect();
    undefined.sort();
    undefined.dedup();
    let undefined_base = first_global + exports.len() as u32;
    for s in &undefined {
        let n = name(s, &mut strtab);
        sym(&mut symtab, n, 0, 0, 0x10, 0); // STB_GLOBAL, undefined
    }
    let mut rel = Vec::new();
    for (at, s, kind) in relocs {
        let index = undefined_base + undefined.iter().position(|u| u == s).unwrap_or(0) as u32;
        u32le(&mut rel, *at as u32);
        u32le(&mut rel, index << 8 | *kind as u32);
    }
    let mut shstrtab = vec![0u8];
    let sh_text = name(".text", &mut shstrtab);
    let sh_rel = name(".rel.text", &mut shstrtab);
    let sh_symtab = name(".symtab", &mut shstrtab);
    let sh_strtab = name(".strtab", &mut shstrtab);
    let sh_shstrtab = name(".shstrtab", &mut shstrtab);

    // Contents after the 52-byte header, each 4-aligned; section headers last.
    let mut body = Vec::new();
    let place = |data: &[u8], body: &mut Vec<u8>| -> u32 {
        while !body.len().is_multiple_of(4) {
            body.push(0);
        }
        let at = 52 + body.len() as u32;
        body.extend_from_slice(data);
        at
    };
    let text_at = place(text, &mut body);
    let rel_at = place(&rel, &mut body);
    let symtab_at = place(&symtab, &mut body);
    let strtab_at = place(&strtab, &mut body);
    let shstrtab_at = place(&shstrtab, &mut body);
    while !body.len().is_multiple_of(4) {
        body.push(0);
    }
    let shoff = 52 + body.len() as u32;

    let mut out = Vec::new();
    out.extend_from_slice(&[0x7F, b'E', b'L', b'F', 1, 1, 1, 0]);
    out.extend_from_slice(&[0; 8]);
    u16le(&mut out, 1); // ET_REL
    u16le(&mut out, 40); // EM_ARM
    u32le(&mut out, 1);
    u32le(&mut out, 0); // entry
    u32le(&mut out, 0); // phoff
    u32le(&mut out, shoff);
    u32le(&mut out, 0x0500_0400); // EABI version 5, hard-float ABI
    u16le(&mut out, 52);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, 40);
    u16le(&mut out, 6); // sections
    u16le(&mut out, 5); // .shstrtab
    out.extend_from_slice(&body);
    let section = |out: &mut Vec<u8>, f: [u32; 10]| {
        for x in f {
            u32le(out, x);
        }
    };
    section(&mut out, [0; 10]);
    section(
        &mut out,
        [sh_text, 1, 6, 0, text_at, text.len() as u32, 0, 0, 4, 0],
    );
    section(
        &mut out,
        [sh_rel, 9, 0x40, 0, rel_at, rel.len() as u32, 3, 1, 4, 8],
    );
    section(
        &mut out,
        [
            sh_symtab,
            2,
            0,
            0,
            symtab_at,
            symtab.len() as u32,
            4,
            first_global,
            4,
            16,
        ],
    );
    section(
        &mut out,
        [
            sh_strtab,
            3,
            0,
            0,
            strtab_at,
            strtab.len() as u32,
            0,
            0,
            1,
            0,
        ],
    );
    section(
        &mut out,
        [
            sh_shstrtab,
            3,
            0,
            0,
            shstrtab_at,
            shstrtab.len() as u32,
            0,
            0,
            1,
            0,
        ],
    );
    out
}
