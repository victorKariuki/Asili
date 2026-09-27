//! Ahead-of-time native code for ASB bytecode, through LLVM.
//!
//! `pata jenga` lowers the typed register bytecode to LLVM IR (text, no C source anywhere),
//! optimizes and links it with `clang -O2` into a shared library next to the `.asb`, and
//! `pata tenda` loads that library and runs its functions instead of interpreting them.
//!
//! The translation:
//!
//! * every `nums` register is an `alloca double` that LLVM's `mem2reg`/SROA promote to SSA
//!   registers, and numeric literals become constants;
//! * `Orodha<Namba>` registers cache their data pointer and length, so `b[i]?` and `b[i] = v`
//!   are a bounds check plus one load/store;
//! * arithmetic, bitwise operators, comparisons and control flow use native instructions with
//!   the interpreter's exact semantics (`llvm.fptosi.sat` for Rust's saturating `as`, the
//!   `0..=63` shift rule, `fmod` remainders with an integer fast path);
//! * anything touching generic `Value`s calls back into the interpreter's `exec_slow` through
//!   the [`Runtime`] table, spilling/reloading only the registers that instruction touches.
//!
//! The library exports a hash of the bytecode it was compiled from and is only used when that
//! hash matches the `.asb` being run, so a stale library can never execute. Set `ASILI_AOT=0`
//! to skip building it, or `ASILI_CLANG` to choose the compiler.

use crate::bytecode::{BytecodeFunc, BytecodeProgram, CmpOp, Opcode, Reg, Ty};
use crate::native::{
    analyze_numbers, leaders, list_writes, num_reads, num_writes, NativeFn, NumFact, STATUS_DEOPT,
    STATUS_FAIL, STATUS_RETURN,
};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Version of the calling convention between generated code and the VM.
const ABI_VERSION: u32 = 3;

/// FNV-1a over the serialized program: identifies the exact bytecode a library was built from.
pub fn program_hash(program: &BytecodeProgram) -> u64 {
    let bytes = bincode::serialize(program).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// File name of the native library for artifact `name` (e.g. `sudoku.so`).
pub fn library_file_name(name: &str) -> String {
    format!("{name}.{}", std::env::consts::DLL_EXTENSION)
}

/// Why no native library was produced.
#[derive(Debug)]
pub enum AotError {
    /// AOT disabled (`ASILI_AOT=0`) or no LLVM compiler available: run on the VM.
    Unavailable(String),
    /// The compiler ran and failed.
    Failed(String),
}

impl std::fmt::Display for AotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AotError::Unavailable(s) | AotError::Failed(s) => f.write_str(s),
        }
    }
}

fn clang() -> String {
    std::env::var("ASILI_CLANG").unwrap_or_else(|_| "clang".to_string())
}

/// Emit LLVM IR for `program`, compile it with clang, and write `dir/<name>.<dll ext>`
/// (the `.ll` is kept beside it for inspection).
pub fn build_library(
    program: &BytecodeProgram,
    dir: &Path,
    name: &str,
) -> Result<PathBuf, AotError> {
    if std::env::var("ASILI_AOT").is_ok_and(|v| v == "0") {
        return Err(AotError::Unavailable("ASILI_AOT=0".into()));
    }
    let compiler = clang();
    let probe = Command::new(&compiler).arg("--version").output();
    if !probe.is_ok_and(|o| o.status.success()) {
        return Err(AotError::Unavailable(format!("{compiler} haipatikani")));
    }
    let ir_path = dir.join(format!("{name}.ll"));
    let lib_path = dir.join(library_file_name(name));
    std::fs::write(&ir_path, emit_llvm_ir(program))
        .map_err(|e| AotError::Failed(format!("imeshindwa kuandika {}: {e}", ir_path.display())))?;
    let mut cmd = Command::new(&compiler);
    cmd.arg("-O2")
        .arg("-shared")
        .arg("-fPIC")
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(&lib_path)
        .arg(&ir_path);
    if cfg!(unix) {
        cmd.arg("-lm");
    }
    let out = cmd
        .output()
        .map_err(|e| AotError::Failed(format!("{compiler}: {e}")))?;
    if !out.status.success() {
        return Err(AotError::Failed(format!(
            "{compiler} imeshindwa: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(lib_path)
}

/// A loaded AOT library whose functions correspond 1:1 to a program's functions.
pub struct NativeLibrary {
    // Keeps the code mapped for as long as the function pointers are used.
    _lib: libloading::Library,
    pub(crate) funcs: Vec<NativeFn>,
}

impl NativeLibrary {
    /// Load `path` for `program`; fails if it was built from different bytecode.
    pub fn load(path: &Path, program: &BytecodeProgram) -> Result<Self, String> {
        // SAFETY: loading runs the library's initializers; generated libraries have none
        // beyond the platform defaults.
        let lib = unsafe { libloading::Library::new(path) }
            .map_err(|e| format!("imeshindwa kupakia {}: {e}", path.display()))?;
        let meta = |symbol: &[u8]| -> Result<u64, String> {
            // SAFETY: every generated library defines these as `i64 ()` functions.
            unsafe {
                let f: libloading::Symbol<unsafe extern "C" fn() -> u64> =
                    lib.get(symbol).map_err(|e| e.to_string())?;
                Ok(f())
            }
        };
        if meta(b"asili_aot_abi")? != ABI_VERSION as u64 {
            return Err("toleo la ABI si sahihi; jenga upya".into());
        }
        if meta(b"asili_aot_hash")? != program_hash(program) {
            return Err("maktaba asilia haiendani na kilele; jenga upya".into());
        }
        let mut funcs = Vec::with_capacity(program.functions.len());
        for i in 0..program.functions.len() {
            let symbol = format!("asili_fn_{i}");
            // SAFETY: `asili_fn_{i}` is generated with exactly `NativeFn`'s signature.
            let f: NativeFn = unsafe {
                *lib.get::<NativeFn>(symbol.as_bytes())
                    .map_err(|e| e.to_string())?
            };
            funcs.push(f);
        }
        Ok(NativeLibrary { _lib: lib, funcs })
    }
}

// ---------------------------------------------------------------------------------------------
// LLVM IR emission
// ---------------------------------------------------------------------------------------------

/// Lower `program` to textual LLVM IR.
pub fn emit_llvm_ir(program: &BytecodeProgram) -> String {
    let mut out = String::new();
    out.push_str("; Asili ASB -> LLVM IR (generated by asili-evaluator aot.rs)\n");
    out.push_str(
        "declare double @llvm.floor.f64(double)\n\
         declare double @llvm.ceil.f64(double)\n\
         declare double @llvm.copysign.f64(double, double)\n\
         declare double @llvm.pow.f64(double, double)\n\
         declare i64 @llvm.fptosi.sat.i64.f64(double)\n\
         declare i32 @llvm.fptosi.sat.i32.f64(double)\n\
         declare i64 @llvm.smax.i64(i64, i64)\n\
         declare double @llvm.fabs.f64(double)\n\
         declare {i64, i1} @llvm.smul.with.overflow.i64(i64, i64)\n\
         declare double @fmod(double, double)\n\n",
    );
    let _ = writeln!(
        out,
        "define i64 @asili_aot_abi() {{\n  ret i64 {ABI_VERSION}\n}}\n"
    );
    let _ = writeln!(
        out,
        "define i64 @asili_aot_hash() {{\n  ret i64 {}\n}}\n",
        program_hash(program) as i64
    );
    for (index, function) in program.functions.iter().enumerate() {
        match FnEmitter::emit(index, function) {
            Some(body) => out.push_str(&body),
            // Unreachable for well-formed bytecode; keep the symbol so loading still works,
            // and make it hand everything to the interpreter.
            None => {
                let _ = writeln!(
                    out,
                    "define i64 @asili_fn_{index}(ptr %rt, ptr %vm, ptr %frame, ptr %nums) {{\n  ret i64 {}\n}}\n",
                    (STATUS_FAIL << 32) as i64
                );
            }
        }
    }
    out
}

/// LLVM's hexadecimal double constant (always exact).
fn f64_lit(x: f64) -> String {
    format!("0x{:016X}", x.to_bits())
}

fn fcmp(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Lt => "olt",
        CmpOp::Le => "ole",
        CmpOp::Gt => "ogt",
        CmpOp::Ge => "oge",
        CmpOp::Eq => "oeq",
        // Rust's `!=` is true when either side is NaN.
        CmpOp::Ne => "une",
    }
}

struct FnEmitter<'a> {
    out: String,
    function: &'a BytecodeFunc,
    index: usize,
    tmp: usize,
    facts: Vec<NumFact>,
    /// Registers proven to hold exact integers: stored as `i64` instead of `double`.
    ints: Vec<bool>,
    /// List accesses proven in bounds (no check emitted).
    safe_index: std::collections::HashSet<usize>,
    /// `i64` registers whose ±2^53 bound is checked on every write (speculation).
    guarded: Vec<bool>,
    /// Instruction being emitted (the deoptimization resume point).
    cur_pc: usize,
    /// `(block, pc)` of every failed-speculation exit, feeding the shared `deopt` block.
    deopt_sites: Vec<(String, usize)>,
}

impl<'a> FnEmitter<'a> {
    fn emit(index: usize, function: &'a BytecodeFunc) -> Option<String> {
        let code = &function.code;
        let leaders = leaders(code)?;
        let analysis = analyze_numbers(function);
        let facts = analysis.regs;
        let ints: Vec<bool> = facts
            .iter()
            .map(|f| f.exact_int() || f.int_like())
            .collect();
        let guarded = facts
            .iter()
            .map(|f| f.int_like() && !f.exact_int())
            .collect();
        let mut e = FnEmitter {
            out: String::new(),
            function,
            index,
            tmp: 0,
            facts,
            ints,
            safe_index: analysis.safe_index,
            guarded,
            cur_pc: 0,
            deopt_sites: Vec::new(),
        };
        let _ = writeln!(
            e.out,
            "define i64 @asili_fn_{index}(ptr %rt, ptr %vm, ptr %frame, ptr %nums) {{\nentry:"
        );
        e.prologue();
        e.line("br label %L0".into());
        let mut terminated = true;
        for (pc, op) in code.iter().enumerate() {
            if leaders.contains(&pc) {
                if !terminated {
                    e.line(format!("br label %L{pc}"));
                }
                let _ = writeln!(e.out, "L{pc}:");
            } else if terminated {
                continue;
            }
            let _ = writeln!(e.out, "  ; {pc}: {op:?}");
            e.cur_pc = pc;
            terminated = e.instruction(pc, op);
        }
        if !terminated {
            let pc = code.len().saturating_sub(1);
            e.ret_status(STATUS_RETURN, pc);
        }
        e.deopt_block();
        e.out.push_str("}\n\n");
        Some(e.out)
    }

    fn line(&mut self, s: String) {
        self.out.push_str("  ");
        self.out.push_str(&s);
        self.out.push('\n');
    }

    fn fresh(&mut self) -> String {
        self.tmp += 1;
        format!("%t{}", self.tmp)
    }

    fn label(&mut self, prefix: &str) -> String {
        self.tmp += 1;
        format!("{prefix}{}", self.tmp)
    }

    fn int(&self, r: Reg) -> bool {
        self.ints[r as usize]
    }

    fn prologue(&mut self) {
        let f = self.function;
        let consts: std::collections::HashMap<Reg, f64> = f.num_consts.iter().copied().collect();
        let params: std::collections::HashSet<(bool, Reg)> = f
            .params
            .iter()
            .filter_map(|p| match p.ty {
                Ty::Num | Ty::Bool => Some((true, p.reg)),
                Ty::List => Some((false, p.reg)),
                Ty::Val => None,
            })
            .collect();
        // Runtime entry points (see `native::Runtime`).
        self.line("%exec = load ptr, ptr %rt".into());
        self.line("%rt.lp = getelementptr ptr, ptr %rt, i64 1".into());
        self.line("%lptr = load ptr, ptr %rt.lp".into());
        self.line("%rt.ll = getelementptr ptr, ptr %rt, i64 2".into());
        self.line("%llen = load ptr, ptr %rt.ll".into());
        self.line("%rt.push = getelementptr ptr, ptr %rt, i64 3".into());
        self.line("%lpush = load ptr, ptr %rt.push".into());
        self.line("%rt.rm = getelementptr ptr, ptr %rt, i64 4".into());
        self.line("%lremove = load ptr, ptr %rt.rm".into());
        for r in 0..f.num_regs {
            let ty = if self.int(r) { "i64" } else { "double" };
            let fact = self.facts[r as usize];
            self.line(format!(
                "%r{r} = alloca {ty} ; [{}, {}] int={} nan={} -0={}",
                fact.lo, fact.hi, fact.int, fact.nan, fact.neg_zero
            ));
        }
        for r in 0..f.list_regs {
            self.line(format!("%lp{r} = alloca ptr"));
            self.line(format!("%ll{r} = alloca i64"));
        }
        for r in 0..f.num_regs {
            if let Some(n) = consts.get(&r) {
                if self.int(r) {
                    self.line(format!("store i64 {}, ptr %r{r}", *n as i64));
                } else {
                    self.line(format!("store double {}, ptr %r{r}", f64_lit(*n)));
                }
            } else if params.contains(&(true, r)) {
                self.reload(r);
            } else if self.int(r) {
                self.line(format!("store i64 0, ptr %r{r}"));
            } else {
                self.line(format!("store double {}, ptr %r{r}", f64_lit(0.0)));
            }
        }
        for r in 0..f.list_regs {
            if params.contains(&(false, r)) {
                self.refresh_list(r);
            } else {
                self.line(format!("store ptr null, ptr %lp{r}"));
                self.line(format!("store i64 0, ptr %ll{r}"));
            }
        }
    }

    /// Register value as `double`.
    fn get_f(&mut self, r: Reg) -> String {
        let t = self.fresh();
        if self.int(r) {
            self.line(format!("{t} = load i64, ptr %r{r}"));
            return self.int_to_float(&t);
        }
        self.line(format!("{t} = load double, ptr %r{r}"));
        t
    }

    /// Register value as `i64`, with Rust's saturating `as i64` for `double` registers.
    fn get_i(&mut self, r: Reg) -> String {
        let t = self.fresh();
        if self.int(r) {
            self.line(format!("{t} = load i64, ptr %r{r}"));
            return t;
        }
        self.line(format!("{t} = load double, ptr %r{r}"));
        self.float_to_int(&t)
    }

    /// Leave to the interpreter (at the current instruction) unless `ok` holds.
    fn guard(&mut self, ok: &str) {
        let site = self.label("dsite");
        let cont = self.label("spec");
        self.line(format!("br i1 {ok}, label %{cont}, label %{site}"));
        let _ = writeln!(self.out, "{site}:");
        self.line("br label %deopt".into());
        let _ = writeln!(self.out, "{cont}:");
        self.deopt_sites.push((site, self.cur_pc));
    }

    /// Check `|v| <= 2^53` for a speculated `i64` register.
    fn guard_i(&mut self, v: &str) {
        let shifted = self.fresh();
        self.line(format!("{shifted} = add i64 {v}, 9007199254740992"));
        let ok = self.fresh();
        self.line(format!("{ok} = icmp ule i64 {shifted}, 18014398509481984"));
        self.guard(&ok);
    }

    /// Shared exit for failed speculation: write every register back and resume in the
    /// interpreter at the failing instruction (which has not written anything yet).
    fn deopt_block(&mut self) {
        if self.deopt_sites.is_empty() {
            return;
        }
        let _ = writeln!(self.out, "deopt:");
        let incoming: Vec<String> = self
            .deopt_sites
            .iter()
            .map(|(site, pc)| format!("[ {pc}, %{site} ]"))
            .collect();
        self.line(format!("%dpc = phi i64 {}", incoming.join(", ")));
        for r in 0..self.function.num_regs {
            self.spill(r);
        }
        self.line(format!(
            "%dret = or i64 %dpc, {}",
            (STATUS_DEOPT << 32) as i64
        ));
        self.line("ret i64 %dret".into());
    }

    fn set_f(&mut self, r: Reg, v: &str) {
        if self.int(r) && self.guarded[r as usize] {
            let a = self.fresh();
            self.line(format!("{a} = call double @llvm.fabs.f64(double {v})"));
            let ok = self.fresh();
            self.line(format!(
                "{ok} = fcmp ole double {a}, {}",
                f64_lit(9_007_199_254_740_992.0)
            ));
            self.guard(&ok);
        }
        if self.int(r) {
            // Every value this register receives is an exact integer, so the conversion
            // cannot saturate or see NaN: one plain `fptosi`.
            let i = self.fresh();
            self.line(format!("{i} = fptosi double {v} to i64"));
            self.line(format!("store i64 {i}, ptr %r{r}"));
        } else {
            self.line(format!("store double {v}, ptr %r{r}"));
        }
    }

    fn set_i(&mut self, r: Reg, v: &str) {
        if self.int(r) && self.guarded[r as usize] {
            self.guard_i(v);
        }
        if self.int(r) {
            self.line(format!("store i64 {v}, ptr %r{r}"));
        } else {
            let f = self.int_to_float(v);
            self.line(format!("store double {f}, ptr %r{r}"));
        }
    }

    fn mem_slot(&mut self, r: Reg) -> String {
        let p = self.fresh();
        self.line(format!("{p} = getelementptr double, ptr %nums, i64 {r}"));
        p
    }

    fn spill(&mut self, r: Reg) {
        let v = self.get_f(r);
        let p = self.mem_slot(r);
        self.line(format!("store double {v}, ptr {p}"));
    }

    fn reload(&mut self, r: Reg) {
        let p = self.mem_slot(r);
        let t = self.fresh();
        self.line(format!("{t} = load double, ptr {p}"));
        self.set_f(r, &t);
    }

    fn refresh_list(&mut self, r: Reg) {
        let p = self.fresh();
        self.line(format!("{p} = call ptr %lptr(ptr %frame, i32 {r})"));
        self.line(format!("store ptr {p}, ptr %lp{r}"));
        let l = self.fresh();
        self.line(format!("{l} = call i64 %llen(ptr %frame, i32 {r})"));
        self.line(format!("store i64 {l}, ptr %ll{r}"));
    }

    fn float_to_int(&mut self, v: &str) -> String {
        let t = self.fresh();
        self.line(format!(
            "{t} = call i64 @llvm.fptosi.sat.i64.f64(double {v})"
        ));
        t
    }

    fn int_to_float(&mut self, v: &str) -> String {
        let t = self.fresh();
        self.line(format!("{t} = sitofp i64 {v} to double"));
        t
    }

    /// Store an `i1` condition as 0/1 into `dst`.
    fn set_flag(&mut self, dst: Reg, cond: &str) {
        if self.int(dst) {
            let t = self.fresh();
            self.line(format!("{t} = zext i1 {cond} to i64"));
            self.line(format!("store i64 {t}, ptr %r{dst}"));
        } else {
            let t = self.fresh();
            self.line(format!(
                "{t} = select i1 {cond}, double {}, double {}",
                f64_lit(1.0),
                f64_lit(0.0)
            ));
            self.line(format!("store double {t}, ptr %r{dst}"));
        }
    }

    /// `a op b` as an `i1`, in integer form when both registers are exact integers.
    fn compare(&mut self, op: CmpOp, a: Reg, b: Reg) -> String {
        let c = self.fresh();
        if self.int(a) && self.int(b) {
            let x = self.get_i(a);
            let y = self.get_i(b);
            let cc = match op {
                CmpOp::Lt => "slt",
                CmpOp::Le => "sle",
                CmpOp::Gt => "sgt",
                CmpOp::Ge => "sge",
                CmpOp::Eq => "eq",
                CmpOp::Ne => "ne",
            };
            self.line(format!("{c} = icmp {cc} i64 {x}, {y}"));
        } else {
            let x = self.get_f(a);
            let y = self.get_f(b);
            self.line(format!("{c} = fcmp {} double {x}, {y}", fcmp(op)));
        }
        c
    }

    /// `r != 0` as an `i1`.
    fn truthy(&mut self, r: Reg) -> String {
        let c = self.fresh();
        if self.int(r) {
            let v = self.get_i(r);
            self.line(format!("{c} = icmp ne i64 {v}, 0"));
        } else {
            let v = self.get_f(r);
            self.line(format!("{c} = fcmp une double {v}, {}", f64_lit(0.0)));
        }
        c
    }

    fn ret_status(&mut self, status: u64, pc: usize) {
        self.line(format!("ret i64 {}", ((status << 32) | pc as u64) as i64));
    }

    /// Run instruction `pc` in the interpreter; return from the function if it ended the call.
    fn slow(&mut self, pc: usize, op: &Opcode) {
        for r in num_reads(op) {
            self.spill(r);
        }
        let s = self.fresh();
        self.line(format!(
            "{s} = call i32 %exec(ptr %vm, ptr %frame, i32 {}, i32 {pc})",
            self.index
        ));
        let c = self.fresh();
        self.line(format!("{c} = icmp ne i32 {s}, 0"));
        let done = self.label("done");
        let cont = self.label("cont");
        self.line(format!("br i1 {c}, label %{done}, label %{cont}"));
        let _ = writeln!(self.out, "{done}:");
        let w = self.fresh();
        self.line(format!("{w} = zext i32 {s} to i64"));
        let sh = self.fresh();
        self.line(format!("{sh} = shl i64 {w}, 32"));
        let rv = self.fresh();
        self.line(format!("{rv} = or i64 {sh}, {pc}"));
        self.line(format!("ret i64 {rv}"));
        let _ = writeln!(self.out, "{cont}:");
        for r in num_writes(op) {
            self.reload(r);
        }
        for r in list_writes(op) {
            self.refresh_list(r);
        }
    }

    /// Bounds-checked element pointer; the out-of-range path lets the interpreter raise the
    /// exact error and leaves the function.
    fn element(&mut self, list: Reg, idx: Reg, pc: usize, op: &Opcode) -> String {
        let i0 = self.get_i(idx);
        let i = if self.int(idx) && self.facts[idx as usize].lo >= 0.0 {
            i0
        } else {
            let i = self.fresh();
            self.line(format!("{i} = call i64 @llvm.smax.i64(i64 {i0}, i64 0)"));
            i
        };
        if self.safe_index.contains(&pc) {
            let base = self.fresh();
            self.line(format!("{base} = load ptr, ptr %lp{list}"));
            let addr = self.fresh();
            self.line(format!(
                "{addr} = getelementptr inbounds double, ptr {base}, i64 {i}"
            ));
            return addr;
        }
        let len = self.fresh();
        self.line(format!("{len} = load i64, ptr %ll{list}"));
        let ok = self.fresh();
        self.line(format!("{ok} = icmp ult i64 {i}, {len}"));
        let fast = self.label("inb");
        let oob = self.label("oob");
        self.line(format!("br i1 {ok}, label %{fast}, label %{oob}"));
        let _ = writeln!(self.out, "{oob}:");
        self.slow(pc, op);
        self.ret_status(STATUS_FAIL, pc);
        let _ = writeln!(self.out, "{fast}:");
        let base = self.fresh();
        self.line(format!("{base} = load ptr, ptr %lp{list}"));
        let addr = self.fresh();
        self.line(format!(
            "{addr} = getelementptr inbounds double, ptr {base}, i64 {i}"
        ));
        addr
    }

    fn remainder_f(&mut self, a: &str, b: &str) -> String {
        let ai = self.float_to_int(a);
        let bi = self.float_to_int(b);
        let af = self.int_to_float(&ai);
        let bf = self.int_to_float(&bi);
        let ea = self.fresh();
        self.line(format!("{ea} = fcmp oeq double {af}, {a}"));
        let eb = self.fresh();
        self.line(format!("{eb} = fcmp oeq double {bf}, {b}"));
        let nz = self.fresh();
        self.line(format!("{nz} = icmp ne i64 {bi}, 0"));
        let nm = self.fresh();
        self.line(format!("{nm} = icmp ne i64 {bi}, -1"));
        let ok1 = self.fresh();
        self.line(format!("{ok1} = and i1 {ea}, {eb}"));
        let ok2 = self.fresh();
        self.line(format!("{ok2} = and i1 {ok1}, {nz}"));
        let ok = self.fresh();
        self.line(format!("{ok} = and i1 {ok2}, {nm}"));
        let fast = self.label("remi");
        let slow = self.label("remf");
        let merge = self.label("remm");
        self.line(format!("br i1 {ok}, label %{fast}, label %{slow}"));
        let _ = writeln!(self.out, "{fast}:");
        let r = self.fresh();
        self.line(format!("{r} = srem i64 {ai}, {bi}"));
        let rf = self.int_to_float(&r);
        // fmod's result carries the dividend's sign, including -0.0.
        let rc = self.fresh();
        self.line(format!(
            "{rc} = call double @llvm.copysign.f64(double {rf}, double {a})"
        ));
        self.line(format!("br label %{merge}"));
        let _ = writeln!(self.out, "{slow}:");
        let rs = self.fresh();
        self.line(format!("{rs} = call double @fmod(double {a}, double {b})"));
        self.line(format!("br label %{merge}"));
        let _ = writeln!(self.out, "{merge}:");
        let res = self.fresh();
        self.line(format!(
            "{res} = phi double [ {rc}, %{fast} ], [ {rs}, %{slow} ]"
        ));
        res
    }

    /// `sakafu(a / b)` right after its division: an exact integer division when both operands
    /// are exact non-negative integers with a positive divisor (below 2^53 the rounded quotient
    /// never crosses the next integer, so the floors agree).
    fn floor_div(&self, pc: usize, src: Reg) -> Option<(Reg, Reg)> {
        let prev = self.function.code.get(pc.checked_sub(1)?)?;
        match prev {
            Opcode::Div { dst, a, b }
                if *dst == src
                    && self.int(*a)
                    && self.int(*b)
                    && self.facts[*a as usize].lo >= 0.0
                    && self.facts[*b as usize].lo > 0.0 =>
            {
                Some((*a, *b))
            }
            _ => None,
        }
    }

    /// Emit one instruction; returns whether it ended the current block.
    fn instruction(&mut self, pc: usize, op: &Opcode) -> bool {
        macro_rules! arith {
            ($dst:expr, $a:expr, $b:expr, $iop:literal, $fop:literal) => {{
                if self.int(*$dst) && self.int(*$a) && self.int(*$b) {
                    let x = self.get_i(*$a);
                    let y = self.get_i(*$b);
                    let t = self.fresh();
                    self.line(format!("{t} = {} i64 {x}, {y}", $iop));
                    self.set_i(*$dst, &t);
                } else {
                    let x = self.get_f(*$a);
                    let y = self.get_f(*$b);
                    let t = self.fresh();
                    self.line(format!("{t} = {} double {x}, {y}", $fop));
                    self.set_f(*$dst, &t);
                }
            }};
        }
        macro_rules! ibin {
            ($dst:expr, $a:expr, $b:expr, $inst:literal) => {{
                let x = self.get_i(*$a);
                let y = self.get_i(*$b);
                let t = self.fresh();
                self.line(format!("{t} = {} i64 {x}, {y}", $inst));
                self.set_i(*$dst, &t);
            }};
        }
        match op {
            Opcode::Mov { dst, src } => {
                if self.int(*dst) && self.int(*src) {
                    // The source already satisfies the destination's bound.
                    let v = self.get_i(*src);
                    self.line(format!("store i64 {v}, ptr %r{dst}"));
                } else {
                    let v = self.get_f(*src);
                    self.set_f(*dst, &v);
                }
            }
            Opcode::Add { dst, a, b } => arith!(dst, a, b, "add nsw", "fadd"),
            Opcode::Sub { dst, a, b } => arith!(dst, a, b, "sub nsw", "fsub"),
            Opcode::Mul { dst, a, b }
                if self.int(*dst)
                    && self.int(*a)
                    && self.int(*b)
                    && (self.guarded[*dst as usize]
                        || self.guarded[*a as usize]
                        || self.guarded[*b as usize]) =>
            {
                // Operands up to 2^53 can overflow i64: check, then bound the product.
                let x = self.get_i(*a);
                let y = self.get_i(*b);
                let pair = self.fresh();
                self.line(format!(
                    "{pair} = call {{i64, i1}} @llvm.smul.with.overflow.i64(i64 {x}, i64 {y})"
                ));
                let prod = self.fresh();
                self.line(format!("{prod} = extractvalue {{i64, i1}} {pair}, 0"));
                let ovf = self.fresh();
                self.line(format!("{ovf} = extractvalue {{i64, i1}} {pair}, 1"));
                let ok = self.fresh();
                self.line(format!("{ok} = xor i1 {ovf}, true"));
                self.guard(&ok);
                self.set_i(*dst, &prod);
            }
            Opcode::Mul { dst, a, b } => arith!(dst, a, b, "mul nsw", "fmul"),
            Opcode::Div { dst, a, b } => {
                let x = self.get_f(*a);
                let y = self.get_f(*b);
                let t = self.fresh();
                self.line(format!("{t} = fdiv double {x}, {y}"));
                self.set_f(*dst, &t);
            }
            Opcode::Rem { dst, a, b } => {
                if self.int(*dst) && self.int(*a) && self.int(*b) {
                    ibin!(dst, a, b, "srem");
                } else {
                    let x = self.get_f(*a);
                    let y = self.get_f(*b);
                    let r = self.remainder_f(&x, &y);
                    self.set_f(*dst, &r);
                }
            }
            Opcode::Pow { dst, a, b } => {
                let x = self.get_f(*a);
                let y = self.get_f(*b);
                let t = self.fresh();
                self.line(format!(
                    "{t} = call double @llvm.pow.f64(double {x}, double {y})"
                ));
                self.set_f(*dst, &t);
            }
            Opcode::BitAnd { dst, a, b } => ibin!(dst, a, b, "and"),
            Opcode::BitOr { dst, a, b } => ibin!(dst, a, b, "or"),
            Opcode::BitXor { dst, a, b } => ibin!(dst, a, b, "xor"),
            Opcode::Shl { dst, a, b } | Opcode::Shr { dst, a, b } => {
                let xi = self.get_i(*a);
                let fb = self.facts[*b as usize];
                // `shift_amount`: saturating i32, anything outside 0..=63 shifts by 0.
                let s64 = if self.int(*b) && fb.lo >= 0.0 && fb.hi <= 63.0 {
                    self.get_i(*b)
                } else {
                    let y = self.get_f(*b);
                    let s = self.fresh();
                    self.line(format!(
                        "{s} = call i32 @llvm.fptosi.sat.i32.f64(double {y})"
                    ));
                    let big = self.fresh();
                    self.line(format!("{big} = icmp ugt i32 {s}, 63"));
                    let s2 = self.fresh();
                    self.line(format!("{s2} = select i1 {big}, i32 0, i32 {s}"));
                    let s64 = self.fresh();
                    self.line(format!("{s64} = zext i32 {s2} to i64"));
                    s64
                };
                let t = self.fresh();
                let inst = if matches!(op, Opcode::Shl { .. }) {
                    "shl"
                } else {
                    "ashr"
                };
                self.line(format!("{t} = {inst} i64 {xi}, {s64}"));
                self.set_i(*dst, &t);
            }
            Opcode::Neg { dst, src } => {
                if self.int(*dst) && self.int(*src) {
                    let v = self.get_i(*src);
                    let t = self.fresh();
                    self.line(format!("{t} = sub nsw i64 0, {v}"));
                    self.set_i(*dst, &t);
                } else {
                    let v = self.get_f(*src);
                    let t = self.fresh();
                    self.line(format!("{t} = fneg double {v}"));
                    self.set_f(*dst, &t);
                }
            }
            Opcode::BitNot { dst, src } => {
                let i = self.get_i(*src);
                let t = self.fresh();
                self.line(format!("{t} = xor i64 {i}, -1"));
                self.set_i(*dst, &t);
            }
            Opcode::Not { dst, src } => {
                let c = self.truthy(*src);
                let n = self.fresh();
                self.line(format!("{n} = xor i1 {c}, true"));
                self.set_flag(*dst, &n);
            }
            Opcode::Floor { dst, src } | Opcode::Ceil { dst, src } => {
                let floor = matches!(op, Opcode::Floor { .. });
                if let (true, Some((a, b))) = (floor && self.int(*dst), self.floor_div(pc, *src)) {
                    ibin!(dst, &a, &b, "sdiv");
                } else if self.int(*src) {
                    let v = self.get_i(*src);
                    self.set_i(*dst, &v);
                } else {
                    let v = self.get_f(*src);
                    let t = self.fresh();
                    let name = if floor { "floor" } else { "ceil" };
                    self.line(format!("{t} = call double @llvm.{name}.f64(double {v})"));
                    self.set_f(*dst, &t);
                }
            }
            Opcode::Trunc { dst, src } => {
                let i = self.get_i(*src);
                self.set_i(*dst, &i);
            }
            Opcode::Cmp { op: cmp, dst, a, b } => {
                let c = self.compare(*cmp, *a, *b);
                self.set_flag(*dst, &c);
            }
            Opcode::Jump { target } => {
                self.line(format!("br label %L{target}"));
                return true;
            }
            Opcode::JumpIfFalse { cond, target } | Opcode::JumpIfTrue { cond, target } => {
                let c = self.truthy(*cond);
                let (t, f) = if matches!(op, Opcode::JumpIfTrue { .. }) {
                    (format!("L{target}"), format!("L{}", pc + 1))
                } else {
                    (format!("L{}", pc + 1), format!("L{target}"))
                };
                self.line(format!("br i1 {c}, label %{t}, label %{f}"));
                return true;
            }
            Opcode::JumpIfNot {
                op: cmp,
                a,
                b,
                target,
            } => {
                let c = self.compare(*cmp, *a, *b);
                self.line(format!("br i1 {c}, label %L{}, label %L{target}", pc + 1));
                return true;
            }
            Opcode::ForStep { ctr, end, target } => {
                if self.int(*ctr) {
                    let x = self.get_i(*ctr);
                    let n = self.fresh();
                    self.line(format!("{n} = add nsw i64 {x}, 1"));
                    self.set_i(*ctr, &n);
                } else {
                    let x = self.get_f(*ctr);
                    let n = self.fresh();
                    self.line(format!("{n} = fadd double {x}, {}", f64_lit(1.0)));
                    self.set_f(*ctr, &n);
                }
                let c = self.compare(CmpOp::Lt, *ctr, *end);
                self.line(format!("br i1 {c}, label %L{target}, label %L{}", pc + 1));
                return true;
            }
            Opcode::ListGet { dst, list, idx, .. } => {
                let addr = self.element(*list, *idx, pc, op);
                let v = self.fresh();
                self.line(format!("{v} = load double, ptr {addr}"));
                self.set_f(*dst, &v);
            }
            Opcode::ListSet { list, idx, src } => {
                let addr = self.element(*list, *idx, pc, op);
                let v = self.get_f(*src);
                self.line(format!("store double {v}, ptr {addr}"));
            }
            Opcode::ListPush { list, src } => {
                let v = self.get_f(*src);
                let p = self.fresh();
                self.line(format!(
                    "{p} = call ptr %lpush(ptr %frame, i32 {list}, double {v})"
                ));
                self.line(format!("store ptr {p}, ptr %lp{list}"));
                let l = self.fresh();
                self.line(format!("{l} = load i64, ptr %ll{list}"));
                let l2 = self.fresh();
                self.line(format!("{l2} = add i64 {l}, 1"));
                self.line(format!("store i64 {l2}, ptr %ll{list}"));
            }
            Opcode::ListRemove { list, idx } => {
                // Out-of-range removal is a no-op, like the interpreter.
                let i0 = self.get_i(*idx);
                let i = self.fresh();
                self.line(format!("{i} = call i64 @llvm.smax.i64(i64 {i0}, i64 0)"));
                let len = self.fresh();
                self.line(format!("{len} = load i64, ptr %ll{list}"));
                let ok = self.fresh();
                self.line(format!("{ok} = icmp ult i64 {i}, {len}"));
                let yes = self.label("rm");
                let done = self.label("rmd");
                self.line(format!("br i1 {ok}, label %{yes}, label %{done}"));
                let _ = writeln!(self.out, "{yes}:");
                let nl = self.fresh();
                self.line(format!(
                    "{nl} = call i64 %lremove(ptr %frame, i32 {list}, i64 {i})"
                ));
                self.line(format!("store i64 {nl}, ptr %ll{list}"));
                self.line(format!("br label %{done}"));
                let _ = writeln!(self.out, "{done}:");
            }
            Opcode::ListLen { dst, list } => {
                let l = self.fresh();
                self.line(format!("{l} = load i64, ptr %ll{list}"));
                self.set_i(*dst, &l);
            }
            Opcode::Return { src } => {
                if matches!(src.ty, Ty::Num | Ty::Bool) {
                    self.spill(src.reg);
                }
                self.ret_status(STATUS_RETURN, pc);
                return true;
            }
            Opcode::ReturnTupu => {
                self.ret_status(STATUS_RETURN, pc);
                return true;
            }
            other => self.slow(pc, other),
        }
        false
    }
}
