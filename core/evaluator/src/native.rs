//! Support for ahead-of-time native code (`aot.rs`), kept separate from the LLVM emitter: the
//! calling convention between machine code and the VM, per-instruction register effects used to
//! spill/reload around interpreter callbacks, and the integer range analysis.

use crate::bytecode::{CmpOp, Frame, Opcode, Reg, Ty};
use std::ffi::c_void;

/// `fn(runtime, vm, frame, nums) -> status << 32 | pc`.
pub(crate) type NativeFn =
    unsafe extern "C" fn(*const Runtime, *mut c_void, *mut Frame, *mut f64) -> u64;

/// Status codes in the upper half of a native function's return value.
pub(crate) const STATUS_FINISH: u64 = 1;
pub(crate) const STATUS_FAIL: u64 = 2;
/// The function reached the `Return`/`ReturnTupu` instruction at `pc` (lower half).
pub(crate) const STATUS_RETURN: u64 = 3;
/// A speculation failed before instruction `pc`: every numeric register has been written back
/// to the frame, and the interpreter continues the call from `pc` (deoptimization).
pub(crate) const STATUS_DEOPT: u64 = 4;

/// Entry points native code calls, passed as the first argument. Field order is ABI: generated
/// code loads them by offset (`nguvu::ir::RtFn`).
#[repr(C)]
pub(crate) struct Runtime {
    /// `(vm, frame, function, pc) -> 0 | STATUS_FINISH | STATUS_FAIL`: run one instruction in
    /// the interpreter.
    pub exec: extern "C" fn(*mut c_void, *mut Frame, u32, u32) -> u32,
    pub list_ptr: extern "C" fn(*mut Frame, u32, u32) -> *mut u64,
    pub list_len: extern "C" fn(*mut Frame, u32) -> i64,
    /// Append; returns the (possibly moved) data pointer.
    pub list_push: extern "C" fn(*mut Frame, u32, f64) -> *mut u64,
    /// Remove at an in-range index; returns the new length.
    pub list_remove: extern "C" fn(*mut Frame, u32, i64) -> i64,
    // Math helpers.
    pub fmod: extern "C" fn(f64, f64) -> f64,
    pub pow: extern "C" fn(f64, f64) -> f64,
    pub floor: extern "C" fn(f64) -> f64,
    pub ceil: extern "C" fn(f64) -> f64,
    /// Rust's saturating `f64 as i64` (NaN → 0).
    pub float_to_int_sat: extern "C" fn(f64) -> i64,
    /// The interpreter's shift amount (`f64 as i32`, outside `0..=63` → 0).
    pub shift_amount: extern "C" fn(f64) -> i64,
    /// Lowest stack address direct native calls may run below (see [`rt_stack_limit`]).
    pub stack_limit: extern "C" fn() -> usize,
    /// `(vm, function, nums, pc) -> status`: finish a directly called function in the
    /// interpreter after it deoptimized.
    pub resume: extern "C" fn(*mut c_void, u32, *mut f64, u32) -> u64,
}

/// Byte offset of the VM's call-depth counter (`Vm` is `repr(C)` with `depth` first).
pub(crate) const VM_DEPTH_OFFSET: i32 = 0;

/// Stack headroom kept free below a direct native call: generated frames and the runtime
/// functions they call fit well inside it.
const DIRECT_CALL_HEADROOM: usize = 256 * 1024;

/// The lowest stack address at which native code may still make a direct call (a stack pointer
/// at or below it takes the interpreter's call path, which can grow the stack), or `usize::MAX`
/// when the stack's extent is unknown (every call then takes that path).
pub(crate) extern "C" fn rt_stack_limit() -> usize {
    let marker = 0u8;
    let sp = std::hint::black_box(&marker) as *const u8 as usize;
    match stacker::remaining_stack() {
        Some(left) if left > DIRECT_CALL_HEADROOM => sp - left + DIRECT_CALL_HEADROOM,
        _ => usize::MAX,
    }
}

pub(crate) extern "C" fn rt_fmod(a: f64, b: f64) -> f64 {
    a % b
}

pub(crate) extern "C" fn rt_pow(a: f64, b: f64) -> f64 {
    a.powf(b)
}

pub(crate) extern "C" fn rt_floor(a: f64) -> f64 {
    a.floor()
}

pub(crate) extern "C" fn rt_ceil(a: f64) -> f64 {
    a.ceil()
}

pub(crate) extern "C" fn rt_float_to_int_sat(a: f64) -> i64 {
    a as i64
}

pub(crate) extern "C" fn rt_shift_amount(a: f64) -> i64 {
    let shift = a as i32;
    if (0..=63).contains(&shift) {
        shift as i64
    } else {
        0
    }
}

/// Data pointer of list register `reg`, switched to integer words when `ints` is nonzero
/// (native code proved every element integral) or `f64` words otherwise; null if integers were
/// asked for and the list holds a non-integer (the caller deoptimizes).
pub(crate) extern "C" fn list_ptr(frame: *mut Frame, reg: u32, ints: u32) -> *mut u64 {
    // SAFETY: called by native code with the frame it was handed; `reg` was checked at compile
    // time to be below the function's `list_regs`.
    let frame = unsafe { &mut *frame };
    frame.lists[reg as usize].ensure(ints != 0)
}

pub(crate) extern "C" fn list_len(frame: *mut Frame, reg: u32) -> i64 {
    // SAFETY: as in `list_ptr`.
    let frame = unsafe { &*frame };
    frame.lists[reg as usize].len() as i64
}

/// Append `value` (keeping the list's representation: native code only pushes integers onto
/// integer lists) and return the possibly moved data pointer.
pub(crate) extern "C" fn list_push(frame: *mut Frame, reg: u32, value: f64) -> *mut u64 {
    // SAFETY: as in `list_ptr`.
    let frame = unsafe { &mut *frame };
    let list = &mut frame.lists[reg as usize];
    list.push(value);
    list.as_mut_ptr()
}

pub(crate) extern "C" fn list_remove(frame: *mut Frame, reg: u32, idx: i64) -> i64 {
    // SAFETY: as in `list_ptr`; native code checked `0 <= idx < len`.
    let frame = unsafe { &mut *frame };
    let list = &mut frame.lists[reg as usize];
    list.remove(idx as usize);
    list.len() as i64
}

/// Block leaders of a function: entry, jump targets, and successors of branches/returns.
/// `None` if a jump target is out of range.
/// Record native code handing a call back to the interpreter. `ASILI_NATIVE_TRACE=1` prints
/// each one (function index and bytecode pc), for finding guards that fail unexpectedly.
pub(crate) fn note_deopt(function: usize, pc: usize) {
    if std::env::var_os("ASILI_NATIVE_TRACE").is_some_and(|v| v == "1") {
        eprintln!("deopt: kazi #{function} pc {pc}");
    }
}

/// Where a jump instruction can transfer control besides falling through.
pub(crate) fn jump_target(op: &Opcode) -> Option<usize> {
    match op {
        Opcode::Jump { target }
        | Opcode::JumpIfFalse { target, .. }
        | Opcode::JumpIfTrue { target, .. }
        | Opcode::JumpIfNot { target, .. }
        | Opcode::ForStep { target, .. } => Some(*target as usize),
        _ => None,
    }
}

pub(crate) fn leaders(code: &[Opcode]) -> Option<std::collections::BTreeSet<usize>> {
    let mut leaders = std::collections::BTreeSet::new();
    leaders.insert(0usize);
    for (pc, op) in code.iter().enumerate() {
        if let Some(target) = jump_target(op) {
            if target >= code.len() {
                return None;
            }
            leaders.insert(target);
            leaders.insert(pc + 1);
        } else if matches!(op, Opcode::Return { .. } | Opcode::ReturnTupu) {
            leaders.insert(pc + 1);
        }
    }
    Some(leaders)
}

/// Numeric registers an instruction reads when executed by `exec_slow`.
pub(crate) fn num_reads(op: &Opcode) -> Vec<Reg> {
    match op {
        Opcode::Mov { src, .. }
        | Opcode::Neg { src, .. }
        | Opcode::BitNot { src, .. }
        | Opcode::Not { src, .. }
        | Opcode::Floor { src, .. }
        | Opcode::Ceil { src, .. }
        | Opcode::Trunc { src, .. }
        | Opcode::BoxNum { src, .. }
        | Opcode::BoxBool { src, .. } => vec![*src],
        Opcode::Add { a, b, .. }
        | Opcode::Sub { a, b, .. }
        | Opcode::Mul { a, b, .. }
        | Opcode::Div { a, b, .. }
        | Opcode::Rem { a, b, .. }
        | Opcode::Pow { a, b, .. }
        | Opcode::BitAnd { a, b, .. }
        | Opcode::BitOr { a, b, .. }
        | Opcode::BitXor { a, b, .. }
        | Opcode::Shl { a, b, .. }
        | Opcode::Shr { a, b, .. }
        | Opcode::Cmp { a, b, .. } => vec![*a, *b],
        Opcode::MakeNumList { items, .. } => items.clone(),
        Opcode::ListRepeat { value, count, .. } => vec![*value, *count],
        Opcode::ListGet { idx, .. }
        | Opcode::ListGetTokeo { idx, .. }
        | Opcode::ListRemove { idx, .. }
        | Opcode::ListRemoveVal { idx, .. } => vec![*idx],
        Opcode::ListSet { idx, src, .. } => vec![*idx, *src],
        Opcode::ListPush { src, .. } => vec![*src],
        Opcode::Call(call) => call
            .args
            .iter()
            .filter(|a| matches!(a.ty, Ty::Num | Ty::Bool))
            .map(|a| a.reg)
            .collect(),
        _ => Vec::new(),
    }
}

/// Numeric registers an instruction writes when executed by `exec_slow`.
pub(crate) fn num_writes(op: &Opcode) -> Vec<Reg> {
    match op {
        Opcode::Mov { dst, .. }
        | Opcode::Add { dst, .. }
        | Opcode::Sub { dst, .. }
        | Opcode::Mul { dst, .. }
        | Opcode::Div { dst, .. }
        | Opcode::Rem { dst, .. }
        | Opcode::Pow { dst, .. }
        | Opcode::BitAnd { dst, .. }
        | Opcode::BitOr { dst, .. }
        | Opcode::BitXor { dst, .. }
        | Opcode::Shl { dst, .. }
        | Opcode::Shr { dst, .. }
        | Opcode::Neg { dst, .. }
        | Opcode::BitNot { dst, .. }
        | Opcode::Not { dst, .. }
        | Opcode::Floor { dst, .. }
        | Opcode::Ceil { dst, .. }
        | Opcode::Trunc { dst, .. }
        | Opcode::Cmp { dst, .. }
        | Opcode::ListGet { dst, .. }
        | Opcode::ListLen { dst, .. }
        | Opcode::UnboxNum { dst, .. }
        | Opcode::UnboxBool { dst, .. }
        | Opcode::ValLen { dst, .. } => vec![*dst],
        Opcode::Call(call) if matches!(call.dst.ty, Ty::Num | Ty::Bool) => vec![call.dst.reg],
        _ => Vec::new(),
    }
}

/// List registers whose storage an instruction may reallocate or replace.
pub(crate) fn list_writes(op: &Opcode) -> Vec<Reg> {
    match op {
        Opcode::MakeNumList { dst, .. }
        | Opcode::ListRepeat { dst, .. }
        | Opcode::ListMov { dst, .. }
        | Opcode::ListFromVal { dst, .. } => vec![*dst],
        Opcode::ListPush { list, .. }
        | Opcode::ListRemove { list, .. }
        | Opcode::ListRemoveVal { list, .. } => vec![*list],
        Opcode::ListMutate(call) => vec![call.recv],
        Opcode::Call(call) if call.dst.ty == Ty::List => vec![call.dst.reg],
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------------------------
// Integer range analysis
// ---------------------------------------------------------------------------------------------

/// Largest magnitude below which every integer is exactly representable as an `f64`.
const EXACT: f64 = 9_007_199_254_740_992.0; // 2^53

/// What is known about every value a numeric register can hold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NumFact {
    pub lo: f64,
    pub hi: f64,
    /// Every value is a whole number.
    pub int: bool,
    /// Some value may be NaN.
    pub nan: bool,
    /// Some value may be `-0.0`.
    pub neg_zero: bool,
}

impl NumFact {
    const TOP: NumFact = NumFact {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
        int: false,
        nan: true,
        neg_zero: true,
    };

    /// No value at all (e.g. read from a list nothing was stored into yet); identity of `join`.
    const BOTTOM: NumFact = NumFact {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
        int: true,
        nan: false,
        neg_zero: false,
    };

    fn is_bottom(&self) -> bool {
        self.lo > self.hi && !self.nan
    }

    fn point(n: f64) -> NumFact {
        NumFact {
            lo: n,
            hi: n,
            int: n.is_finite() && n.fract() == 0.0,
            nan: n.is_nan(),
            neg_zero: n == 0.0 && n.is_sign_negative(),
        }
    }

    fn int_range(lo: f64, hi: f64) -> NumFact {
        NumFact {
            lo,
            hi,
            int: true,
            nan: false,
            neg_zero: false,
        }
    }

    /// Representable as an `i64` whose `as f64` is exactly the value, with no NaN or `-0.0`:
    /// integer arithmetic then gives bit-identical results to the `f64` semantics.
    pub fn exact_int(&self) -> bool {
        self.int && !self.nan && !self.neg_zero && self.lo > -EXACT && self.hi < EXACT
    }

    /// Whole numbers without NaN/-0.0 whose range is not bounded by ±2^53: native code may keep
    /// them as `i64` if every write checks the bound and deoptimizes when it fails.
    pub fn int_like(&self) -> bool {
        self.int && !self.nan && !self.neg_zero
    }

    fn join(self, other: NumFact) -> NumFact {
        NumFact {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            int: self.int && other.int,
            nan: self.nan || other.nan,
            neg_zero: self.neg_zero || other.neg_zero,
        }
    }

    fn finite(&self) -> bool {
        self.lo.is_finite() && self.hi.is_finite() && !self.nan
    }

    fn contains_zero(&self) -> bool {
        self.lo <= 0.0 && self.hi >= 0.0
    }

    /// Range of `x as i64` (saturating, truncating) over this fact.
    fn trunc_range(&self) -> (f64, f64) {
        let clamp = |x: f64| {
            x.trunc()
                .clamp(-9.223_372_036_854_776e18, 9.223_372_036_854_776e18)
        };
        let lo = if self.nan { self.lo.min(0.0) } else { self.lo };
        let hi = if self.nan { self.hi.max(0.0) } else { self.hi };
        (clamp(lo), clamp(hi))
    }
}

/// Result of an integer-valued operation (bitwise, comparisons): always a whole number and
/// never NaN/-0.0; exact only while inside ±2^53.
fn int_result(lo: f64, hi: f64) -> NumFact {
    NumFact::int_range(lo, hi)
}

fn next_pow2_minus_one(x: f64) -> f64 {
    if x < 1.0 {
        return 0.0;
    }
    let mut p = 1.0f64;
    while p <= x {
        p *= 2.0;
    }
    p - 1.0
}

/// Per-register facts for `f`'s numeric registers: the join of every value the register can
/// ever hold. Computed by forward dataflow over the basic blocks, narrowing ranges on each
/// branch edge (inside `wakati v <= 9`, `v` is at most 9), widening at loops so it terminates.
/// Result of [`analyze_numbers`].
pub(crate) struct NumAnalysis {
    /// Join of every value each numeric register can hold.
    pub regs: Vec<NumFact>,
    /// `ListGet`/`ListSet` instructions whose index is proven in bounds.
    pub safe_index: std::collections::HashSet<usize>,
    /// Per list register: every element it can ever hold is an exact integer (within ±2^53),
    /// so native code keeps it in integer words (see `NumList`).
    pub int_lists: Vec<bool>,
}

pub(crate) fn analyze_numbers(f: &crate::bytecode::BytecodeFunc) -> NumAnalysis {
    let nregs = f.num_regs as usize;
    // State slots: numeric registers, then one pseudo-register per list holding its length.
    let n = nregs + f.list_regs as usize;
    let give_up = || NumAnalysis {
        regs: vec![NumFact::TOP; nregs],
        safe_index: Default::default(),
        int_lists: vec![false; f.list_regs as usize],
    };
    let code = &f.code;
    let Some(leaders) = leaders(code) else {
        return give_up();
    };
    let starts: Vec<usize> = leaders.into_iter().filter(|&l| l < code.len()).collect();
    let block_of = |pc: usize| starts.partition_point(|&s| s <= pc) - 1;

    // Frame entry: constants, parameters, and zero for everything else.
    let consts: std::collections::HashMap<Reg, f64> = f.num_consts.iter().copied().collect();
    let params: std::collections::HashSet<Reg> = f
        .params
        .iter()
        .filter(|p| matches!(p.ty, Ty::Num | Ty::Bool))
        .map(|p| p.reg)
        .collect();
    let list_params: std::collections::HashSet<Reg> = f
        .params
        .iter()
        .filter(|p| p.ty == Ty::List)
        .map(|p| p.reg)
        .collect();
    let entry: Vec<NumFact> = (0..n as Reg)
        .map(|r| {
            if r as usize >= nregs {
                if list_params.contains(&(r - nregs as Reg)) {
                    NumFact::int_range(0.0, f64::INFINITY)
                } else {
                    NumFact::point(0.0)
                }
            } else if let Some(c) = consts.get(&r) {
                NumFact::point(*c)
            } else if params.contains(&r) {
                NumFact::TOP
            } else {
                NumFact::point(0.0)
            }
        })
        .collect();

    // Join of every value stored into each `Orodha<Namba>` register (`None`: nothing yet).
    let mut lists: Vec<Option<NumFact>> = vec![None; f.list_regs as usize];
    for p in &f.params {
        if p.ty == Ty::List {
            lists[p.reg as usize] = Some(NumFact::TOP);
        }
    }

    let mut list_changes = vec![0u32; lists.len()];
    // Widening thresholds: the program's own constants (and their neighbours), so a counter
    // bounded by `9` or `81` widens to that bound instead of straight to infinity.
    // Powers of two (and 2^k - 1) cover bit masks, which grow by doubling.
    let mut thresholds: Vec<f64> = (0..=53)
        .flat_map(|k| {
            let p = 2f64.powi(k);
            [p - 1.0, p, -p, 1.0 - p]
        })
        .collect();
    for (_, c) in &f.num_consts {
        if c.is_finite() {
            thresholds.extend([*c - 1.0, *c, *c + 1.0]);
        }
    }
    thresholds.sort_by(f64::total_cmp);
    thresholds.dedup();
    let widen = |m: &mut NumFact, o: &NumFact| {
        if m.lo < o.lo {
            m.lo = thresholds
                .iter()
                .rev()
                .find(|t| **t <= m.lo)
                .copied()
                .unwrap_or(f64::NEG_INFINITY);
        }
        if m.hi > o.hi {
            m.hi = thresholds
                .iter()
                .find(|t| **t >= m.hi)
                .copied()
                .unwrap_or(f64::INFINITY);
        }
    };
    for _outer in 0..32 {
        let mut defs: Vec<NumFact> = entry.clone();
        let mut lists_changed = false;
        let mut in_states: Vec<Option<Vec<NumFact>>> = vec![None; starts.len()];
        let mut visits = vec![vec![0u32; n]; starts.len()];
        in_states[0] = Some(entry.clone());
        let mut work = std::collections::BTreeSet::from([0usize]);
        let mut steps = 0usize;
        while let Some(b) = work.pop_first() {
            steps += 1;
            if steps > 100_000 {
                return give_up();
            }
            let mut state = in_states[b].clone().expect("queued blocks have a state");
            let end = starts.get(b + 1).copied().unwrap_or(code.len());
            let mut edges: Vec<(usize, Vec<NumFact>)> = Vec::new();
            let mut falls_through = true;
            for (pc, op) in code.iter().enumerate().take(end).skip(starts[b]) {
                for (list, fact) in list_transfer(op, &state, &lists) {
                    if fact.is_bottom() {
                        continue;
                    }
                    let l = list as usize;
                    let mut new = match lists[l] {
                        Some(old) => old.join(fact),
                        None => fact,
                    };
                    if lists[l] != Some(new) {
                        list_changes[l] += 1;
                        // Threshold widening: finitely many steps, then infinity.
                        if let (Some(old), true) = (lists[l], list_changes[l] > 4) {
                            widen(&mut new, &old);
                        }
                        lists[l] = Some(new);
                        lists_changed = true;
                    }
                }
                let bottom_input = num_reads(op).iter().any(|r| state[*r as usize].is_bottom())
                    || matches!(op, Opcode::ListGet { list, .. } if lists[*list as usize].is_none());
                for (dst, fact) in transfer(op, &state, &lists) {
                    state[dst as usize] = if bottom_input { NumFact::BOTTOM } else { fact };
                }
                for (slot, fact) in len_transfer(op, &state, nregs) {
                    state[slot] = fact;
                }
                let next = pc + 1;
                match op {
                    Opcode::Jump { target } => {
                        edges.push((*target as usize, state.clone()));
                        falls_through = false;
                    }
                    Opcode::JumpIfFalse { target, .. } | Opcode::JumpIfTrue { target, .. } => {
                        edges.push((*target as usize, state.clone()));
                        edges.push((next, state.clone()));
                        falls_through = false;
                    }
                    Opcode::JumpIfNot { op, a, b, target } => {
                        if let Some(s) = refine(&state, *op, *a, *b, true) {
                            edges.push((next, s));
                        }
                        if let Some(s) = refine(&state, *op, *a, *b, false) {
                            edges.push((*target as usize, s));
                        }
                        falls_through = false;
                    }
                    Opcode::ForStep { ctr, end, target } => {
                        if let Some(s) = refine(&state, CmpOp::Lt, *ctr, *end, true) {
                            edges.push((*target as usize, s));
                        }
                        if let Some(s) = refine(&state, CmpOp::Lt, *ctr, *end, false) {
                            edges.push((next, s));
                        }
                        falls_through = false;
                    }
                    Opcode::Return { .. } | Opcode::ReturnTupu => falls_through = false,
                    _ => {}
                }
            }
            if falls_through && end < code.len() {
                edges.push((end, state));
            }
            for (target_pc, out) in edges {
                if target_pc >= code.len() {
                    continue;
                }
                let t = block_of(target_pc);
                let merged = match &in_states[t] {
                    None => out,
                    Some(old) => {
                        let mut merged: Vec<NumFact> =
                            old.iter().zip(&out).map(|(a, b)| a.join(*b)).collect();
                        if merged == *old {
                            continue;
                        }
                        // Widen only registers that keep growing at this block.
                        for (r, (m, o)) in merged.iter_mut().zip(old).enumerate() {
                            if m != o {
                                visits[t][r] += 1;
                                if visits[t][r] > 4 {
                                    widen(m, o);
                                }
                            }
                        }
                        merged
                    }
                };
                in_states[t] = Some(merged);
                work.insert(t);
            }
        }
        if !lists_changed {
            // Summarize from the converged block states only.
            let mut safe_index = std::collections::HashSet::new();
            for (b, in_state) in in_states.iter().enumerate() {
                let Some(mut state) = in_state.clone() else {
                    continue;
                };
                let end = starts.get(b + 1).copied().unwrap_or(code.len());
                for (pc, op) in code.iter().enumerate().take(end).skip(starts[b]) {
                    if let Opcode::ListGet { list, idx, .. } | Opcode::ListSet { list, idx, .. } =
                        op
                    {
                        let (i, len) = (state[*idx as usize], state[nregs + *list as usize]);
                        if !i.nan && i.lo >= 0.0 && i.hi < len.lo {
                            safe_index.insert(pc);
                        }
                    }
                    let bottom_input = num_reads(op).iter().any(|r| state[*r as usize].is_bottom())
                        || matches!(op, Opcode::ListGet { list, .. } if lists[*list as usize].is_none());
                    for (dst, fact) in transfer(op, &state, &lists) {
                        let fact = if bottom_input { NumFact::BOTTOM } else { fact };
                        state[dst as usize] = fact;
                        defs[dst as usize] = defs[dst as usize].join(fact);
                    }
                    for (slot, fact) in len_transfer(op, &state, nregs) {
                        state[slot] = fact;
                    }
                }
            }
            defs.truncate(nregs);
            return NumAnalysis {
                regs: defs,
                safe_index,
                int_lists: lists
                    .iter()
                    .map(|l| l.is_none_or(|f| f.is_bottom() || f.exact_int()))
                    .collect(),
            };
        }
    }
    give_up()
}

/// Effect of an instruction on list lengths (pseudo-registers after the numeric ones).
fn len_transfer(op: &Opcode, state: &[NumFact], nregs: usize) -> Vec<(usize, NumFact)> {
    let slot = |list: &Reg| nregs + *list as usize;
    let unknown = NumFact::int_range(0.0, f64::INFINITY);
    match op {
        Opcode::MakeNumList { dst, items } => {
            vec![(slot(dst), NumFact::point(items.len() as f64))]
        }
        Opcode::ListRepeat { dst, count, .. } => {
            let c = state[*count as usize];
            let len = if c.exact_int() && c.lo >= 0.0 {
                NumFact::int_range(c.lo, c.hi)
            } else {
                unknown
            };
            vec![(slot(dst), len)]
        }
        Opcode::ListPush { list, .. } => {
            let l = state[slot(list)];
            vec![(slot(list), NumFact::int_range(l.lo + 1.0, l.hi + 1.0))]
        }
        Opcode::ListRemove { list, .. } | Opcode::ListRemoveVal { list, .. } => {
            // Out-of-range removal is a no-op, so the length drops by at most one.
            let l = state[slot(list)];
            vec![(slot(list), NumFact::int_range((l.lo - 1.0).max(0.0), l.hi))]
        }
        Opcode::ListMov { dst, src } => vec![(slot(dst), state[slot(src)])],
        Opcode::ListFromVal { dst, .. } => vec![(slot(dst), unknown)],
        Opcode::ListMutate(call) => vec![(slot(&call.recv), unknown)],
        Opcode::Call(call) if call.dst.ty == Ty::List => vec![(slot(&call.dst.reg), unknown)],
        _ => Vec::new(),
    }
}

/// Narrow `state` on the edge where `(a op b) == holds`; `None` if that edge is impossible.
fn refine(state: &[NumFact], op: CmpOp, a: Reg, b: Reg, holds: bool) -> Option<Vec<NumFact>> {
    // `!(a < b)` is `a >= b` only without NaN; a NaN operand makes every comparison false.
    let (x, y) = (state[a as usize], state[b as usize]);
    let op = if holds {
        op
    } else if x.nan || y.nan {
        return Some(state.to_vec());
    } else {
        match op {
            CmpOp::Lt => CmpOp::Ge,
            CmpOp::Le => CmpOp::Gt,
            CmpOp::Gt => CmpOp::Le,
            CmpOp::Ge => CmpOp::Lt,
            CmpOp::Eq => CmpOp::Ne,
            CmpOp::Ne => CmpOp::Eq,
        }
    };
    let step = if x.int && y.int { 1.0 } else { 0.0 };
    let (mut nx, mut ny) = (x, y);
    match op {
        CmpOp::Lt => {
            nx.hi = nx.hi.min(y.hi - step);
            ny.lo = ny.lo.max(x.lo + step);
        }
        CmpOp::Le => {
            nx.hi = nx.hi.min(y.hi);
            ny.lo = ny.lo.max(x.lo);
        }
        CmpOp::Gt => {
            nx.lo = nx.lo.max(y.lo + step);
            ny.hi = ny.hi.min(x.hi - step);
        }
        CmpOp::Ge => {
            nx.lo = nx.lo.max(y.lo);
            ny.hi = ny.hi.min(x.hi);
        }
        CmpOp::Eq => {
            nx.lo = x.lo.max(y.lo);
            nx.hi = x.hi.min(y.hi);
            ny.lo = nx.lo;
            ny.hi = nx.hi;
        }
        CmpOp::Ne => {
            // Only a point can be cut off an integer range's end.
            if y.lo == y.hi && x.int && y.int {
                if x.lo == y.lo {
                    nx.lo += 1.0;
                }
                if x.hi == y.lo {
                    nx.hi -= 1.0;
                }
            }
            if x.lo == x.hi && x.int && y.int {
                if y.lo == x.lo {
                    ny.lo += 1.0;
                }
                if y.hi == x.lo {
                    ny.hi -= 1.0;
                }
            }
        }
    }
    if op != CmpOp::Ne {
        // An ordered comparison that held means neither side was NaN.
        nx.nan = false;
        ny.nan = false;
    }
    if nx.lo > nx.hi || ny.lo > ny.hi {
        return None;
    }
    let mut out = state.to_vec();
    out[a as usize] = nx;
    out[b as usize] = ny;
    if a == b {
        out[a as usize] = NumFact {
            lo: nx.lo.max(ny.lo),
            hi: nx.hi.min(ny.hi),
            ..nx
        };
    }
    Some(out)
}

/// Facts about values flowing into list registers.
fn list_transfer(op: &Opcode, facts: &[NumFact], lists: &[Option<NumFact>]) -> Vec<(Reg, NumFact)> {
    let g = |r: &Reg| facts[*r as usize];
    match op {
        Opcode::MakeNumList { dst, items } => items.iter().map(|r| (*dst, g(r))).collect(),
        Opcode::ListRepeat { dst, value, .. } => vec![(*dst, g(value))],
        Opcode::ListSet { list, src, .. } | Opcode::ListPush { list, src } => {
            vec![(*list, g(src))]
        }
        Opcode::ListMov { dst, src } => lists[*src as usize]
            .map(|f| vec![(*dst, f)])
            .unwrap_or_default(),
        Opcode::ListFromVal { dst, .. } => vec![(*dst, NumFact::TOP)],
        Opcode::ListMutate(call) => vec![(call.recv, NumFact::TOP)],
        Opcode::Call(call) if call.dst.ty == Ty::List => vec![(call.dst.reg, NumFact::TOP)],
        _ => Vec::new(),
    }
}

fn transfer(op: &Opcode, facts: &[NumFact], lists: &[Option<NumFact>]) -> Vec<(Reg, NumFact)> {
    let g = |r: &Reg| facts[*r as usize];
    let fact = match op {
        Opcode::Mov { dst, src } => (*dst, g(src)),
        Opcode::Add { dst, a, b } | Opcode::Sub { dst, a, b } => {
            let (x, y) = (g(a), g(b));
            let add = matches!(op, Opcode::Add { .. });
            let (lo, hi) = if add {
                (x.lo + y.lo, x.hi + y.hi)
            } else {
                (x.lo - y.hi, x.hi - y.lo)
            };
            // Only inf - inf is NaN.
            let inf_minus_inf = if add {
                (x.hi == f64::INFINITY && y.lo == f64::NEG_INFINITY)
                    || (x.lo == f64::NEG_INFINITY && y.hi == f64::INFINITY)
            } else {
                (x.hi == f64::INFINITY && y.hi == f64::INFINITY)
                    || (x.lo == f64::NEG_INFINITY && y.lo == f64::NEG_INFINITY)
            };
            (
                *dst,
                NumFact {
                    lo: if lo.is_nan() { f64::NEG_INFINITY } else { lo },
                    hi: if hi.is_nan() { f64::INFINITY } else { hi },
                    int: x.int && y.int,
                    nan: x.nan || y.nan || inf_minus_inf,
                    // -0 + -0 = -0; -0 - +0 = -0.
                    neg_zero: if add {
                        x.neg_zero && y.neg_zero
                    } else {
                        x.neg_zero
                    },
                },
            )
        }
        Opcode::Mul { dst, a, b } => {
            let (x, y) = (g(a), g(b));
            if !(x.finite() && y.finite()) {
                (*dst, NumFact::TOP)
            } else {
                let c = [x.lo * y.lo, x.lo * y.hi, x.hi * y.lo, x.hi * y.hi];
                let neg_zero = x.neg_zero
                    || y.neg_zero
                    || (x.lo < 0.0 && y.contains_zero())
                    || (y.lo < 0.0 && x.contains_zero());
                (
                    *dst,
                    NumFact {
                        lo: c.iter().copied().fold(f64::INFINITY, f64::min),
                        hi: c.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                        int: x.int && y.int,
                        nan: false,
                        neg_zero,
                    },
                )
            }
        }
        Opcode::Div { dst, a, b } => {
            let (x, y) = (g(a), g(b));
            if x.finite() && y.finite() && (y.lo > 0.0 || y.hi < 0.0) {
                let c = [x.lo / y.lo, x.lo / y.hi, x.hi / y.lo, x.hi / y.hi];
                let nonneg = x.lo >= 0.0 && !x.neg_zero && y.lo > 0.0;
                (
                    *dst,
                    NumFact {
                        lo: c.iter().copied().fold(f64::INFINITY, f64::min),
                        hi: c.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                        int: false,
                        nan: false,
                        neg_zero: !nonneg,
                    },
                )
            } else {
                (*dst, NumFact::TOP)
            }
        }
        Opcode::Rem { dst, a, b } => {
            let (x, y) = (g(a), g(b));
            let divisor_nonzero = y.lo > 0.0 || y.hi < 0.0;
            if x.finite() && y.finite() && divisor_nonzero {
                // |x % y| < |y|; for integers that is at most |y| - 1.
                let m = y.lo.abs().max(y.hi.abs());
                let m = if x.int && y.int { m - 1.0 } else { m };
                let lo = if x.lo < 0.0 { -m } else { 0.0 };
                let hi = if x.hi > 0.0 {
                    m.min(x.hi.max(0.0))
                } else {
                    0.0
                };
                (
                    *dst,
                    NumFact {
                        lo,
                        hi,
                        int: x.int && y.int,
                        nan: false,
                        // fmod keeps the dividend's sign, including zero results.
                        neg_zero: x.lo < 0.0 || x.neg_zero,
                    },
                )
            } else {
                (*dst, NumFact::TOP)
            }
        }
        Opcode::Pow { dst, .. } => (*dst, NumFact::TOP),
        Opcode::BitAnd { dst, a, b } => {
            let ((xl, xh), (yl, yh)) = (g(a).trunc_range(), g(b).trunc_range());
            let hi = match (xl >= 0.0, yl >= 0.0) {
                (true, true) => xh.min(yh),
                (true, false) => xh,
                (false, true) => yh,
                (false, false) => return vec![(*dst, int_result(-EXACT * 1024.0, EXACT * 1024.0))],
            };
            (*dst, int_result(0.0, hi))
        }
        Opcode::BitOr { dst, a, b } | Opcode::BitXor { dst, a, b } => {
            let ((xl, xh), (yl, yh)) = (g(a).trunc_range(), g(b).trunc_range());
            if xl >= 0.0 && yl >= 0.0 {
                (*dst, int_result(0.0, next_pow2_minus_one(xh.max(yh))))
            } else {
                (*dst, int_result(-EXACT * 1024.0, EXACT * 1024.0))
            }
        }
        Opcode::Shl { dst, a, b } => {
            let ((xl, xh), (yl, yh)) = (g(a).trunc_range(), g(b).trunc_range());
            if xl >= 0.0 {
                // Amounts outside 0..=63 shift by 0, so only the in-range part matters.
                let _ = yl;
                let shift = yh.clamp(0.0, 63.0);
                (*dst, int_result(0.0, xh * 2f64.powf(shift)))
            } else {
                (*dst, int_result(-EXACT * 1024.0, EXACT * 1024.0))
            }
        }
        Opcode::Shr { dst, a, .. } => {
            let (xl, xh) = g(a).trunc_range();
            (*dst, int_result(xl.min(0.0), xh.max(0.0)))
        }
        Opcode::BitNot { dst, src } => {
            let (xl, xh) = g(src).trunc_range();
            (*dst, int_result(-xh - 1.0, -xl - 1.0))
        }
        Opcode::Neg { dst, src } => {
            let x = g(src);
            (
                *dst,
                NumFact {
                    lo: -x.hi,
                    hi: -x.lo,
                    int: x.int,
                    nan: x.nan,
                    // -(+0.0) is -0.0.
                    neg_zero: x.contains_zero() || x.nan,
                },
            )
        }
        Opcode::Not { dst, .. } | Opcode::Cmp { dst, .. } => (*dst, int_result(0.0, 1.0)),
        Opcode::Floor { dst, src } | Opcode::Ceil { dst, src } => {
            let x = g(src);
            let floor = matches!(op, Opcode::Floor { .. });
            let (lo, hi) = if floor {
                (x.lo.floor(), x.hi.floor())
            } else {
                (x.lo.ceil(), x.hi.ceil())
            };
            (
                *dst,
                NumFact {
                    lo,
                    hi,
                    int: x.finite(),
                    nan: x.nan,
                    // floor keeps -0.0; ceil also yields -0.0 on (-1, 0).
                    neg_zero: x.neg_zero || (!floor && x.lo < 0.0 && x.hi > -1.0),
                },
            )
        }
        Opcode::Trunc { dst, src } => {
            let (lo, hi) = g(src).trunc_range();
            (*dst, int_result(lo, hi))
        }
        Opcode::ForStep { ctr, end, .. } => {
            // The counter only advances while below `end`.
            let (c, e) = (g(ctr), g(end));
            let cap = if e.nan { f64::INFINITY } else { e.hi.ceil() };
            let lo = c.lo + 1.0;
            // Bounded by `end` directly, so the range converges without widening.
            let hi = if cap.is_finite() {
                cap.max(lo)
            } else {
                c.hi + 1.0
            };
            (
                *ctr,
                NumFact {
                    lo,
                    hi,
                    int: c.int,
                    nan: c.nan,
                    neg_zero: false,
                },
            )
        }
        Opcode::ListLen { dst, .. } | Opcode::ValLen { dst, .. } => {
            (*dst, int_result(0.0, EXACT - 1.0))
        }
        Opcode::UnboxBool { dst, .. } => (*dst, int_result(0.0, 1.0)),
        // Reads return a stored element (out-of-range reads leave the function).
        Opcode::ListGet { dst, list, .. } => (*dst, lists[*list as usize].unwrap_or(NumFact::TOP)),
        other => {
            return num_writes(other)
                .into_iter()
                .map(|r| (r, NumFact::TOP))
                .collect()
        }
    };
    vec![fact]
}
