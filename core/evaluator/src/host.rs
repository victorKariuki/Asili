//! The host side of native code: what the machine code `nguvu` generates calls back into.
//!
//! A [`Host`] owns one program's call frames (typed register files), its builtin table and
//! constants, and runs every bytecode function as native code — nothing interprets bytecode. Instructions native code does not compile (strings, maps, structs, builtins,
//! method calls, …) come back through [`native_exec`] one at a time and run here on the shared
//! semantics (`eval::ops`, `eval::methods`).

// Runtime code never panics on its own: an impossible state is an error the program sees
// (and its safe state handles), not a crash (see docs/design/safety-critical-roadmap.md §3).
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

use crate::builtins::BuiltinFn;
use crate::bytecode::{
    BytecodeFunc, BytecodeProgram, CmpOp, IndexMode, Opcode, Operand, StoredConstant, Ty, UnaryCode,
};
use crate::eval::{methods, ops};
use crate::numlist::NumList;
use crate::value::{self, EvalError, Value};
use asili_parser::UnaryOp;
use std::collections::HashMap;

type Reg = crate::bytecode::Reg;

/// One call's register files. Native code reads `nums` through a raw pointer and `lists`
/// through `native::list_ptr`/`list_len`, so neither may be resized while the call runs.
#[derive(Default)]
pub(crate) struct Frame {
    pub(crate) nums: Vec<f64>,
    pub(crate) lists: Vec<NumList>,
    pub(crate) vals: Vec<Value>,
}

pub(crate) enum Flow {
    Next,
    Finish(Ret),
    Fail(EvalError),
}

pub(crate) enum Ret {
    Num(f64),
    List(NumList),
    Val(Value),
}

// `repr(C)`, starting with what native code reads by offset (`native::DEPTH_OFFSET` and the
// next two): direct calls count themselves in `depth`, so the call-depth limit is the same on
// every tier, and direct entries find the runtime table and their stack limit here.
// Each is a 64-bit word on every target (a 32-bit target pads the runtime table's address).
#[repr(C)]
pub(crate) struct Host<'p> {
    depth: u64,
    runtime: &'static crate::native::Runtime,
    #[cfg(target_pointer_width = "32")]
    runtime_high: u32,
    /// Lowest stack address at which native code may make a direct call, for the stack
    /// segment the current native call runs on (set by `run_native`).
    stack_limit: u64,
    program: &'p BytecodeProgram,
    builtins: Vec<BuiltinFn>,
    builtin_index: HashMap<String, usize>,
    pool: Vec<Frame>,
    /// `program.constants` as values, built once: loading one (a shared `Neno` included) is a
    /// reference-count bump.
    consts: Vec<Value>,
    /// `consts` interned, for the `Neno` constants that name struct fields and types.
    names: Vec<crate::value::Name>,
    /// This program as other threads receive it (`tenda`, server workers); made on first use
    /// when the host was not started from a shared program.
    shared: Option<crate::spawn::Shared>,
    /// Builtin indices of `tenda`, `mkondo_tumikia`, `mkondo_tumikia_http` and `anzisha`,
    /// which need the program itself.
    spawners: [usize; 4],
    /// Argument buffer for builtin and method calls, reused so a call allocates nothing (see
    /// [`Host::take_args`]).
    args: Vec<Value>,
    /// Whether `init_constants` has run.
    constants_ready: bool,
    /// The source lines executed so far, when coverage is recorded (`Opcode::Line`).
    pub(crate) coverage: Option<std::collections::HashSet<usize>>,
    /// The debugger attached to this run, told of every `Opcode::Line`.
    pub(crate) debug: Option<std::sync::Arc<dyn crate::debug_hook::DebugHook>>,
    /// The program's machine code (`nguvu`), one entry per function.
    native: &'p crate::aot::NativeLibrary,
    /// Outcome of an instruction that native code handed to `exec_slow` and that ended the call.
    pending: Option<Flow>,
    /// The error now leaving calls was already traced (by the first call it left).
    error_traced: bool,
}

const _: () = {
    use crate::native::{DEPTH_OFFSET, LIMIT_OFFSET, RT_OFFSET};
    assert!(std::mem::offset_of!(Host<'static>, depth) == DEPTH_OFFSET as usize);
    assert!(std::mem::offset_of!(Host<'static>, runtime) == RT_OFFSET as usize);
    assert!(std::mem::offset_of!(Host<'static>, stack_limit) == LIMIT_OFFSET as usize);
};

pub(crate) static NATIVE_RUNTIME: crate::native::Runtime = crate::native::Runtime {
    exec: native_exec,
    list_ptr: crate::native::list_ptr,
    list_len: crate::native::list_len,
    list_head: crate::native::list_head,
    list_room: crate::native::list_room,
    list_push: crate::native::list_push,
    list_remove: crate::native::list_remove,
    fmod: crate::native::rt_fmod,
    pow: crate::native::rt_pow,
    floor: crate::native::rt_floor,
    ceil: crate::native::rt_ceil,
    float_to_int_sat: crate::native::rt_float_to_int_sat,
    shift_amount: crate::native::rt_shift_amount,
    call_host: native_call_host,
    depth_error: native_depth_error,
    box_num: crate::native::rt_box_num,
    box_bool: crate::native::rt_box_bool,
    val_mov: crate::native::rt_val_mov,
    const_val: native_const_val,
};

/// `ConstVal` from native code: `vals[dst] = constant k`.
extern "C" fn native_const_val(host: *mut std::ffi::c_void, frame: *mut Frame, dst: u32, k: u32) {
    // SAFETY: native code passes the `Host` and `Frame` that `run_native` handed it.
    let (host, frame) = unsafe { (&*(host as *const Host<'static>), &mut *frame) };
    frame.vals[dst as usize] = host.consts[k as usize].clone();
}

/// A direct native call that cannot run on the native stack (too deep, or too little room left):
/// make it through the host's own call path instead, with the arguments native code stored in
/// `nums` (the callee's register buffer), storing a numeric result at `nums[num_regs]`. Returns
/// `STATUS_RETURN`, or `STATUS_FAIL` with the error pending.
extern "C" fn native_call_host(host: *mut std::ffi::c_void, function: u32, nums: *mut f64) -> u64 {
    // SAFETY: called by native code with the `Host` it was handed, whose program is live, and a
    // buffer of `num_regs + 1` registers for `function` holding its arguments.
    let host = unsafe { &mut *(host as *mut Host<'static>) };
    let program = host.program;
    let index = function as usize;
    let f = &program.functions[index];
    let buf = unsafe { std::slice::from_raw_parts_mut(nums, f.num_regs as usize + 1) };
    let mut frame = host.frame_for(f);
    for p in &f.params {
        frame.nums[p.reg as usize] = buf[p.reg as usize];
    }
    // `invoke` counts the depth (reporting the limit), grows the stack, and runs the callee's
    // native code.
    match host.invoke(index, frame) {
        Ok(Ret::Num(v)) => {
            buf[f.num_regs as usize] = v;
            crate::native::STATUS_RETURN << 32
        }
        Ok(_) => {
            host.pending = Some(Flow::Fail(type_err("aina ya thamani ya kurudi si sahihi")));
            crate::native::STATUS_FAIL << 32
        }
        Err(e) => {
            host.pending = Some(Flow::Fail(e));
            crate::native::STATUS_FAIL << 32
        }
    }
}

/// `exec_slow` entry point for native code: `0` to continue, else a `native::STATUS_*`.
extern "C" fn native_exec(
    host: *mut std::ffi::c_void,
    frame: *mut Frame,
    function: u32,
    pc: u32,
) -> u32 {
    // SAFETY: native code only calls this with the `Host` and `Frame` that `run_native` passed
    // in, both of which outlive the call and are not otherwise borrowed while it runs.
    let host = unsafe { &mut *(host as *mut Host<'static>) };
    let frame = unsafe { &mut *frame };
    let program = host.program;
    let op = &program.functions[function as usize].code[pc as usize];
    match host.exec_slow(op, frame) {
        Flow::Next => 0,
        flow @ Flow::Finish(_) => {
            host.pending = Some(flow);
            crate::native::STATUS_FINISH as u32
        }
        flow @ Flow::Fail(_) => {
            host.pending = Some(flow);
            crate::native::STATUS_FAIL as u32
        }
    }
}

/// Deepest chain of `kazi` calls a program may make; one more fails with `undani mno`.
pub(crate) const MAX_CALL_DEPTH: usize = 10_000;

/// The error of a call past [`MAX_CALL_DEPTH`].
fn depth_error() -> EvalError {
    EvalError::Unknown("undani mno".into())
}

/// `CheckDepth` failed in native code: leave the error pending and return the failure status.
extern "C" fn native_depth_error(host: *mut std::ffi::c_void) -> u64 {
    // SAFETY: native code passes the `Host` it was handed.
    let host = unsafe { &mut *(host as *mut Host<'static>) };
    host.pending = Some(Flow::Fail(depth_error()));
    crate::native::STATUS_FAIL << 32
}

fn type_err(msg: &str) -> EvalError {
    EvalError::TypeErr(msg.to_string())
}

#[inline(always)]
fn to_index(n: f64) -> usize {
    (n as i64).max(0) as usize
}

#[inline(always)]
fn shift_amount(n: f64) -> u32 {
    let shift = n as i32;
    if (0..=63).contains(&shift) {
        shift as u32
    } else {
        0
    }
}

#[inline(always)]
fn compare(op: CmpOp, a: f64, b: f64) -> bool {
    match op {
        CmpOp::Lt => a < b,
        CmpOp::Le => a <= b,
        CmpOp::Gt => a > b,
        CmpOp::Ge => a >= b,
        CmpOp::Eq => a == b,
        CmpOp::Ne => a != b,
    }
}

#[inline(always)]
fn flag(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

impl<'p> Host<'p> {
    /// A host running `program` on `native` (built from exactly this program). `shared`
    /// describes the program to the threads it starts; without it the first thread started
    /// builds a shared copy.
    pub(crate) fn new(
        program: &'p BytecodeProgram,
        native: &'p crate::aot::NativeLibrary,
        shared: Option<crate::spawn::Shared>,
    ) -> Self {
        let table = crate::builtins::BuiltinTable::new();
        Host {
            depth: 0,
            runtime: &NATIVE_RUNTIME,
            #[cfg(target_pointer_width = "32")]
            runtime_high: 0,
            stack_limit: u64::MAX,
            program,
            args: Vec::new(),
            spawners: crate::builtins::MODULE_BUILTINS
                .map(|name| table.index.get(name).copied().unwrap_or(usize::MAX)),
            builtins: table.fns,
            builtin_index: table.index,
            shared,
            pool: Vec::new(),
            consts: program
                .constants
                .iter()
                .map(StoredConstant::to_value)
                .collect(),
            names: program
                .constants
                .iter()
                .map(|c| match c {
                    StoredConstant::Neno(text) => crate::value::Name::new(text),
                    _ => crate::value::Name::default(),
                })
                .collect(),
            native,
            pending: None,
            error_traced: false,
            constants_ready: false,
            coverage: None,
            debug: None,
        }
    }

    /// Call the program's `kazi` called `name`. A top-level call (not one made from inside the
    /// program) also finishes every `sawia` task it started before returning.
    pub(crate) fn call_by_name(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, EvalError> {
        self.init_constants()?;
        let index = self
            .program
            .functions
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| EvalError::UndefinedVar(name.to_string()))?;
        let result = self.call_values(index, args);
        if self.depth == 0 {
            crate::kazi_sawia::drain();
        }
        result
    }

    /// `anzisha(kazi_jina, hoja...)`: start the program's `kazi_jina` as a `sawia` task.
    fn anzisha(&mut self, args: &[Value]) -> Result<Value, EvalError> {
        let name = crate::builtins::arg_str(args, 0);
        let index = self
            .program
            .functions
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| EvalError::UndefinedVar(name.clone()))?;
        // The host outlives every task it starts (its top-level call drains them).
        let host =
            self as *mut Host<'p> as *mut Host<'static> as *mut dyn crate::kazi_sawia::Context;
        let task = crate::kazi_sawia::spawn(host, index, args.get(1..).unwrap_or(&[]).to_vec());
        Ok(Value::Ahadi(task))
    }

    /// Compute the program's non-literal module constants (once per host, before its first
    /// call) into the constant slots that stand for them.
    fn init_constants(&mut self) -> Result<(), EvalError> {
        let Some(init) = (!std::mem::replace(&mut self.constants_ready, true))
            .then_some(self.program.init)
            .flatten()
        else {
            return Ok(());
        };
        let values = self.call_values(init as usize, Vec::new())?;
        let Value::Orodha(items) = &values else {
            return Err(internal("thabiti zilizokokotolewa"));
        };
        for (k, c) in self.program.constants.iter().enumerate() {
            if let StoredConstant::Computed(i) = c {
                self.consts[k] = items[*i as usize].clone();
            }
        }
        Ok(())
    }

    /// This program as other threads receive it: the one the host was started from, or else a
    /// shared copy with its own native code (built once, on the first thread started).
    fn shared_program(&mut self) -> Result<crate::spawn::Shared, EvalError> {
        if let Some(shared) = &self.shared {
            return Ok(shared.clone());
        }
        let program = std::sync::Arc::new(self.program.clone());
        let native = std::sync::Arc::new(crate::bytecode::native_for(&program)?);
        let shared = crate::spawn::Shared::Code { program, native };
        self.shared = Some(shared.clone());
        Ok(shared)
    }

    /// `Neno` constant `k` interned (a struct or field name).
    fn name(&self, k: u32) -> crate::value::Name {
        self.names[k as usize]
    }

    fn frame_for(&mut self, f: &BytecodeFunc) -> Frame {
        let mut frame = self.pool.pop().unwrap_or_default();
        frame.nums.clear();
        frame.nums.resize(f.num_regs as usize, 0.0);
        // Lists keep their storage, emptied, so a frame reused from the pool allocates nothing
        // for lists of the sizes it held before.
        frame.lists.truncate(f.list_regs as usize);
        frame.lists.iter_mut().for_each(NumList::clear);
        frame
            .lists
            .resize_with(f.list_regs as usize, NumList::default);
        frame.vals.clear();
        frame.vals.resize_with(f.val_regs as usize, || Value::Hamna);
        for (reg, n) in &f.num_consts {
            frame.nums[*reg as usize] = *n;
        }
        frame
    }

    /// Return a finished call's frame to the pool. Its generic values are dropped now — a file
    /// or connection a `kazi` held is released when the call ends, not when the frame is next
    /// reused.
    fn release(&mut self, mut frame: Frame) {
        frame.vals.clear();
        self.pool.push(frame);
    }

    /// Call a function with generic arguments, converting to and from its typed registers.
    fn call_values(&mut self, index: usize, args: Vec<Value>) -> Result<Value, EvalError> {
        self.poll_signal()?;
        let program = self.program;
        let f = program
            .functions
            .get(index)
            .ok_or_else(|| EvalError::Unknown("faharisi ya kazi si halali".into()))?;
        let mut frame = self.frame_for(f);
        for (param, arg) in f.params.iter().zip(args) {
            store_value(&mut frame, *param, arg)?;
        }
        Ok(match self.invoke(index, frame)? {
            Ret::Num(n) if f.ret == Ty::Bool => Value::Ukweli(n != 0.0),
            Ret::Num(n) => Value::Namba(n),
            Ret::List(l) => Value::list(l.iter().map(Value::Namba).collect()),
            Ret::Val(v) => v,
        })
    }

    fn invoke(&mut self, index: usize, frame: Frame) -> Result<Ret, EvalError> {
        self.depth += 1;
        if self.depth > MAX_CALL_DEPTH as u64 {
            self.depth -= 1;
            return Err(depth_error());
        }
        // Native code makes direct calls only while `DIRECT_CALL_HEADROOM` is free below it, so
        // give it that much (on a fresh segment when the stack is short — or when its size is
        // unknown past the committed pages, as on musl's main thread).
        let native = self.native.funcs[index];
        let red_zone = 64 * 1024 + crate::native::DIRECT_CALL_HEADROOM;
        // Calls native code makes directly to native code bypass the host and are not traced.
        let function = &self.program.functions[index];
        let span = asili_trace::enter(&function.name, function.line);
        let result = crate::stack::maybe_grow(red_zone, 2 * 1024 * 1024, || {
            self.run_native(native, index, frame)
        });
        self.depth -= 1;
        match &result {
            Err(e) if asili_trace::on() && !self.error_traced => {
                asili_trace::emit(asili_trace::Tukio::Kosa, &e.to_string(), 0);
                self.error_traced = true;
            }
            Ok(_) => self.error_traced = false,
            Err(_) => {}
        }
        drop(span);
        result
    }

    fn run_native(
        &mut self,
        native: crate::native::NativeFn,
        index: usize,
        mut frame: Frame,
    ) -> Result<Ret, EvalError> {
        let nums = frame.nums.as_mut_ptr();
        let host = self as *mut Host<'p> as *mut std::ffi::c_void;
        // SAFETY: `native` was compiled from `program.functions[index]` for exactly this frame
        // layout (`frame_for` sized every register file), and the frame is not resized while
        // the call runs.
        // Direct calls check against the limit of the stack this call runs on (`invoke` may
        // have moved it to a new segment); the caller's limit is back in force after.
        let outer = std::mem::replace(&mut self.stack_limit, crate::native::stack_limit() as u64);
        let status = unsafe { native(self.runtime, host, &mut frame, nums) };
        self.stack_limit = outer;
        let pc = (status & 0xffff_ffff) as usize;
        let result = match status >> 32 {
            crate::native::STATUS_RETURN => Ok(match &self.program.functions[index].code[pc] {
                Opcode::Return { src } => match src.ty {
                    Ty::Num | Ty::Bool => Ret::Num(frame.nums[src.reg as usize]),
                    Ty::List => Ret::List(std::mem::take(&mut frame.lists[src.reg as usize])),
                    Ty::Val => Ret::Val(std::mem::replace(
                        &mut frame.vals[src.reg as usize],
                        Value::Hamna,
                    )),
                },
                _ => Ret::Val(Value::Tupu),
            }),
            _ => match self.pending.take() {
                Some(Flow::Finish(ret)) => Ok(ret),
                Some(Flow::Fail(err)) => Err(err),
                _ => Err(EvalError::Unknown("hali ya msimbo asilia si sahihi".into())),
            },
        };
        self.release(frame);
        result
    }

    /// Invoke a callback named by a string (for `ramani`, `chuja`, ...): builtins first, then
    /// module functions, matching the evaluator.
    fn callback(&mut self, name: &str, args: Vec<Value>) -> Result<Value, EvalError> {
        if let Some(i) = self.builtin_index.get(name) {
            return (self.builtins[*i])(&args);
        }
        let index = self
            .program
            .functions
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| EvalError::TypeErr(format!("kazi haijulikani: {name}")))?;
        self.call_values(index, args)
    }

    fn call_method(
        &mut self,
        recv: Value,
        method: &str,
        args: Vec<Value>,
    ) -> Result<Value, EvalError> {
        if methods::is_pure_method(&recv, method) {
            return methods::pure_method(&recv, method, &args);
        }
        if methods::is_mutating(&recv, method) {
            return methods::mutate_temporary(recv, method, &args);
        }
        if methods::is_callback_method(&recv, method) {
            return methods::callback_method(&recv, method, &args, &mut |name, a| {
                self.callback(name, a.to_vec())
            });
        }
        // A method the program defines on its own `umbo` or `jenum`.
        let (kind, target) = match &recv {
            Value::Struct(name, _) => ("umbo", name.to_string()),
            Value::Enum(name, _, _) => ("jenum", name.to_string()),
            _ => {
                return Err(EvalError::TypeErr(format!(
                    "mwito wa njia '{method}' unahitaji Neno, Orodha, jenum au umbo"
                )))
            }
        };
        let index = self.program.user_method(kind, &target, method)?;
        let mut call_args = Vec::with_capacity(args.len() + 1);
        call_args.push(recv);
        call_args.extend(args);
        self.call_values(index, call_args)
    }

    /// Execute one instruction native code hands over (everything off its numeric fast path).
    /// Copies of the registers `regs` of `vals`, in the reused argument buffer: hand it back with
    /// [`Host::give_args`]. A nested call meanwhile simply starts a buffer of its own.
    fn take_args(&mut self, vals: &[Value], regs: &[Reg]) -> Vec<Value> {
        let mut args = std::mem::take(&mut self.args);
        args.extend(regs.iter().map(|r| vals[*r as usize].clone()));
        args
    }

    fn give_args(&mut self, mut args: Vec<Value>) {
        args.clear();
        if args.capacity() >= self.args.capacity() {
            self.args = args;
        }
    }

    /// Run the `kazi` registered (`sikiliza_ishara`) for a signal that arrived, if any. Polled
    /// whenever native code calls into the host.
    fn poll_signal(&mut self) -> Result<(), EvalError> {
        if crate::alloc::over_limit() {
            return Err(EvalError::Unknown(format!(
                "kikomo cha kumbukumbu kimezidiwa (baiti {})",
                crate::alloc::system_bytes()
            )));
        }
        let signal = crate::signal::take_pending();
        if signal == 0 {
            return Ok(());
        }
        let Some(name) = crate::signal::get_handler(signal) else {
            return Ok(());
        };
        let Some(index) = self.program.functions.iter().position(|f| f.name == name) else {
            return Ok(());
        };
        self.call_values(index, Vec::new()).map(drop)
    }

    fn exec_slow(&mut self, op: &Opcode, frame: &mut Frame) -> Flow {
        if let Err(e) = self.poll_signal() {
            return Flow::Fail(e);
        }
        let program = self.program;
        macro_rules! finish {
            ($ret:expr) => {{
                return Flow::Finish($ret);
            }};
        }
        macro_rules! fail {
            ($err:expr) => {{
                return Flow::Fail($err);
            }};
        }
        if numeric_op(op, &mut frame.nums) {
            return Flow::Next;
        }
        match op {
            Opcode::MakeStruct { dst, name, fields } => {
                let flds = fields
                    .iter()
                    .map(|(f, r)| (self.name(*f), frame.vals[*r as usize].clone()))
                    .collect();
                frame.vals[*dst as usize] = Value::Struct(self.name(*name), flds);
                return Flow::Next;
            }
            Opcode::Field {
                dst,
                src,
                field,
                slot,
            } => {
                return match methods::field_at(&frame.vals[*src as usize], field, *slot) {
                    Ok(v) => {
                        frame.vals[*dst as usize] = v.clone();
                        Flow::Next
                    }
                    Err(e) => Flow::Fail(e),
                };
            }
            Opcode::IterItem { dst, items, idx } => {
                let item = match &frame.vals[*items as usize] {
                    Value::Orodha(items) => items.get(frame.nums[*idx as usize] as usize).cloned(),
                    _ => None,
                };
                let Some(item) = item else {
                    fail!(internal("kipengee cha kitanzi"));
                };
                frame.vals[*dst as usize] = item;
                return Flow::Next;
            }
            Opcode::FieldNum {
                dst,
                src,
                field,
                slot,
            } => {
                return match methods::field_at(&frame.vals[*src as usize], field, *slot) {
                    Ok(Value::Namba(v)) => {
                        frame.nums[*dst as usize] = *v;
                        Flow::Next
                    }
                    Ok(other) => Flow::Fail(EvalError::TypeErr(format!(
                        "operesheni inahitaji Namba, ilipata {other:?}"
                    ))),
                    Err(e) => Flow::Fail(e),
                };
            }
            Opcode::MakeEnum {
                dst,
                enum_name,
                variant,
                data,
            } => {
                let data = data.map(|r| Box::new(frame.vals[r as usize].clone()));
                frame.vals[*dst as usize] = Value::Enum(*enum_name, *variant, data);
                return Flow::Next;
            }
            Opcode::MatchPattern {
                dst,
                src,
                pattern,
                binds,
                binding,
            } => {
                let mut bound: Vec<(Reg, Value)> = Vec::new();
                let matched = crate::eval::pattern::match_pattern(
                    pattern,
                    &frame.vals[*src as usize],
                    &mut |name, v| {
                        if let Some((_, reg)) = binds.iter().find(|(n, _)| *n == name) {
                            bound.push((*reg, v.clone()));
                        }
                    },
                );
                if matched {
                    for (reg, v) in bound {
                        frame.vals[reg as usize] = v;
                    }
                } else if *binding {
                    return Flow::Fail(crate::eval::pattern::let_pattern_mismatch());
                }
                frame.nums[*dst as usize] = flag(matched);
                return Flow::Next;
            }
            Opcode::MakeMap { dst, entries } => {
                let mut m = crate::value::Kamusi::with_capacity_and_hasher(
                    entries.len(),
                    Default::default(),
                );
                for (k, v) in entries.iter() {
                    match value::MapKey::try_from_value(&frame.vals[*k as usize]) {
                        Ok(key) => {
                            m.insert(key, frame.vals[*v as usize].clone());
                        }
                        Err(e) => return Flow::Fail(e),
                    }
                }
                frame.vals[*dst as usize] = Value::Kamusi(std::rc::Rc::new(m));
                return Flow::Next;
            }
            _ => {}
        }
        let n = &mut frame.nums;
        match op {
            Opcode::CheckDepth => {
                if self.depth >= MAX_CALL_DEPTH as u64 {
                    fail!(depth_error());
                }
            }
            Opcode::Line { line, binds } => {
                if let Some(lines) = &mut self.coverage {
                    lines.insert(*line as usize);
                }
                if let Some(hook) = &self.debug {
                    // A fresh snapshot before `should_pause` may block, so a `variables` request
                    // made while paused sees this line's values: the locals visible here, then
                    // the predefined names.
                    let mut bindings: Vec<(String, String)> = binds
                        .iter()
                        .map(|(name, op)| {
                            (name.clone(), format!("{:?}", operand_value(frame, *op)))
                        })
                        .collect();
                    bindings.extend(
                        crate::env::global_constants()
                            .into_iter()
                            .filter(|(n, _)| !binds.iter().any(|(b, _)| b == n))
                            .map(|(n, v)| (n.to_string(), format!("{v:?}"))),
                    );
                    hook.record_bindings(bindings);
                    hook.should_pause(*line as usize);
                }
            }
            Opcode::ListRepeat { dst, value, count } => {
                let count = n[*count as usize];
                if !(count.is_finite() && count >= 0.0 && count.fract() == 0.0) {
                    fail!(type_err(
                        "orodha_rudia inahitaji idadi ya Namba kamili isiyo hasi"
                    ));
                }
                let v = n[*value as usize];
                frame.lists[*dst as usize] = NumList::repeat(v, count as usize);
            }
            Opcode::MakeNumList { dst, items } => {
                let list: NumList = items.iter().map(|r| n[*r as usize]).collect();
                frame.lists[*dst as usize] = list;
            }
            Opcode::ListGet {
                dst,
                list,
                idx,
                mode,
            } => {
                let i = to_index(n[*idx as usize]);
                let l = &frame.lists[*list as usize];
                match (l.get(i), mode) {
                    (Some(v), _) => frame.nums[*dst as usize] = v,
                    (None, IndexMode::Element) => fail!(methods::out_of_bounds_error(i, l.len())),
                    (None, IndexMode::Tokeo) => {
                        finish!(Ret::Val(methods::out_of_bounds(i, l.len())))
                    }
                }
            }
            Opcode::ListGetTokeo { dst, list, idx } => {
                let i = to_index(n[*idx as usize]);
                let l = &frame.lists[*list as usize];
                let v = match l.get(i) {
                    Some(v) => Value::sawa(Value::Namba(v)),
                    None => methods::out_of_bounds(i, l.len()),
                };
                frame.vals[*dst as usize] = v;
            }
            Opcode::ListSet { list, idx, src } => {
                let i = to_index(n[*idx as usize]);
                let v = n[*src as usize];
                if !frame.lists[*list as usize].set(i, v) {
                    fail!(type_err("ingiza: index nje ya mipaka"));
                }
            }
            Opcode::ListPush { list, src } => {
                let v = n[*src as usize];
                frame.lists[*list as usize].push(v);
            }
            Opcode::ListRemove { list, idx } => {
                let i = to_index(n[*idx as usize]);
                let l = &mut frame.lists[*list as usize];
                if i < l.len() {
                    l.remove(i);
                }
            }
            Opcode::ListRemoveVal { dst, list, idx } => {
                let i = to_index(n[*idx as usize]);
                let l = &mut frame.lists[*list as usize];
                frame.vals[*dst as usize] = if i < l.len() {
                    Value::Chaguo(Some(Box::new(Value::Namba(l.remove(i)))))
                } else {
                    Value::Chaguo(None)
                };
            }
            Opcode::ListLen { dst, list } => {
                n[*dst as usize] = frame.lists[*list as usize].len() as f64
            }
            Opcode::ListMov { dst, src } => {
                if dst != src {
                    let (d, s) = (*dst as usize, *src as usize);
                    // A copy into the register's own storage (no allocation when it fits).
                    let (a, b) = frame.lists.split_at_mut(d.max(s));
                    if d < s {
                        a[d].clone_from(&b[0]);
                    } else {
                        b[0].clone_from(&a[s]);
                    }
                }
            }
            Opcode::ListFromVal { dst, src } => match list_from_value(&frame.vals[*src as usize]) {
                Ok(list) => frame.lists[*dst as usize] = list,
                Err(e) => fail!(e),
            },
            Opcode::ListToVal { dst, src } => {
                let v = Value::list(
                    frame.lists[*src as usize]
                        .iter()
                        .map(Value::Namba)
                        .collect(),
                );
                frame.vals[*dst as usize] = v;
            }
            Opcode::ConstVal { dst, k } => {
                frame.vals[*dst as usize] = self.consts[*k as usize].clone();
            }
            Opcode::ValMov { dst, src } => crate::native::val_mov(frame, *dst, *src),
            Opcode::BoxNum { dst, src } => {
                let v = n[*src as usize];
                crate::native::box_num(frame, *dst, v)
            }
            Opcode::BoxBool { dst, src } => {
                let v = n[*src as usize];
                crate::native::box_bool(frame, *dst, v)
            }
            Opcode::UnboxNum { dst, src } => match &frame.vals[*src as usize] {
                Value::Namba(v) => frame.nums[*dst as usize] = *v,
                Value::Tokeo(Err(_)) => {
                    let err = frame.vals[*src as usize].clone();
                    finish!(Ret::Val(err));
                }
                other => fail!(EvalError::TypeErr(format!(
                    "operesheni inahitaji Namba, ilipata {other:?}"
                ))),
            },
            Opcode::UnboxBool { dst, src } => {
                frame.nums[*dst as usize] = flag(ops::truthy(&frame.vals[*src as usize]))
            }
            Opcode::ValBinary { op, dst, a, b } => {
                if dst == a && dst != b {
                    if let Ok([target, r]) =
                        frame.vals.get_disjoint_mut([*dst as usize, *b as usize])
                    {
                        if ops::assign_in_place(op, target, r) {
                            return Flow::Next;
                        }
                    }
                }
                match ops::binary_value(op, &frame.vals[*a as usize], &frame.vals[*b as usize]) {
                    Ok(v) => frame.vals[*dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::ValUnary { op, dst, src } => {
                let ast_op = match op {
                    UnaryCode::Neg => UnaryOp::Neg,
                    UnaryCode::BitNot => UnaryOp::BitNot,
                };
                match ops::unary_value(&ast_op, frame.vals[*src as usize].clone()) {
                    Ok(v) => frame.vals[*dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::ValIndex {
                dst,
                base,
                idx,
                mode,
            } => {
                let read = match mode {
                    IndexMode::Element => methods::index_element,
                    IndexMode::Tokeo => methods::index_value,
                };
                match read(&frame.vals[*base as usize], &frame.vals[*idx as usize]) {
                    Ok(v) => frame.vals[*dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::ValLen { dst, src } => {
                let len = match &frame.vals[*src as usize] {
                    Value::Orodha(items) => items.len(),
                    Value::Neno(s) => methods::grapheme_count(s),
                    _ => fail!(type_err("urefu inahitaji Orodha au Neno")),
                };
                frame.nums[*dst as usize] = len as f64;
            }
            Opcode::Unwrap { dst, src } => {
                match ops::propagate(frame.vals[*src as usize].clone()) {
                    Ok(v) => frame.vals[*dst as usize] = v,
                    // `?` on an error returns it from this function, like the evaluator.
                    Err(EvalError::Propagate(err)) => finish!(Ret::Val(err)),
                    Err(e) => fail!(e),
                }
            }
            Opcode::ListMutate(call) => {
                let args: Vec<Value> = call
                    .args
                    .iter()
                    .map(|r| frame.vals[*r as usize].clone())
                    .collect();
                // In place on the typed list where the method allows it.
                let list = &mut frame.lists[call.recv as usize];
                match (call.method.as_str(), args.first()) {
                    ("futa_zote", _) => {
                        list.clear();
                        frame.vals[call.dst as usize] = Value::Tupu;
                        return Flow::Next;
                    }
                    ("ongeza_zote", Some(Value::Orodha(more)))
                        if more.iter().all(|v| matches!(v, Value::Namba(_))) =>
                    {
                        for v in more.iter() {
                            if let Value::Namba(n) = v {
                                list.push(*n);
                            }
                        }
                        frame.vals[call.dst as usize] = Value::Tupu;
                        return Flow::Next;
                    }
                    _ => {}
                }
                let list = std::mem::take(&mut frame.lists[call.recv as usize]);
                let mut value = Value::list(list.iter().map(Value::Namba).collect());
                let result = methods::mutate(&mut value, call.method.as_str(), &args);
                match list_from_value(&value) {
                    Ok(list) => frame.lists[call.recv as usize] = list,
                    Err(e) => fail!(e),
                }
                match result {
                    Ok(v) => frame.vals[call.dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::ListMethod(call) => {
                let args: Vec<Value> = call
                    .args
                    .iter()
                    .map(|r| frame.vals[*r as usize].clone())
                    .collect();
                let list = &frame.lists[call.list as usize];
                let method = call.method.as_str();
                let out = match methods::numbers_method(list, method, &args) {
                    Some(out) => out,
                    // Arguments that are not numbers: the generic method, same answer.
                    None => {
                        let generic = Value::list(list.iter().map(Value::Namba).collect());
                        methods::pure_method(&generic, method, &args).map(methods::NumOut::Val)
                    }
                };
                let out = match out {
                    Ok(out) => out,
                    Err(e) => fail!(e),
                };
                let reg = call.dst.reg as usize;
                match (call.dst.ty, out) {
                    (Ty::List, methods::NumOut::List(l)) => frame.lists[reg] = l,
                    (Ty::List, other) => match list_from_value(&other.into_value()) {
                        Ok(l) => frame.lists[reg] = l,
                        Err(e) => fail!(e),
                    },
                    (Ty::Num | Ty::Bool, methods::NumOut::Num(n)) => frame.nums[reg] = n,
                    (Ty::Num | Ty::Bool, methods::NumOut::Bool(b)) => {
                        frame.nums[reg] = if b { 1.0 } else { 0.0 }
                    }
                    (Ty::Num | Ty::Bool, other) => match other.into_value() {
                        Value::Namba(n) => frame.nums[reg] = n,
                        Value::Ukweli(b) => frame.nums[reg] = if b { 1.0 } else { 0.0 },
                        _ => fail!(internal("ListMethod")),
                    },
                    (Ty::Val, out) => frame.vals[reg] = out.into_value(),
                }
            }
            Opcode::IterItems { dst, src } => {
                match methods::iter_items(frame.vals[*src as usize].clone()) {
                    Ok(items) => frame.vals[*dst as usize] = Value::Orodha(items),
                    Err(e) => fail!(e),
                }
            }
            Opcode::Jaribu { dst, src } => match ops::jaribu(&frame.vals[*src as usize]) {
                Ok(v) => frame.vals[*dst as usize] = v,
                Err(e) => fail!(e),
            },
            Opcode::Cast { dst, src, ty } => {
                let v = frame.vals[*src as usize].clone();
                match methods::cast_value(v, ty) {
                    Ok(v) => frame.vals[*dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::MakeList { dst, items } => {
                let list = items
                    .iter()
                    .map(|r| frame.vals[*r as usize].clone())
                    .collect();
                frame.vals[*dst as usize] = Value::list(list);
            }
            Opcode::Call(call) => {
                let callee = &program.functions[call.function as usize];
                let mut callee_frame = self.frame_for(callee);
                for (arg, param) in call.args.iter().zip(&callee.params) {
                    copy_operand(frame, *arg, &mut callee_frame, *param);
                }
                let ret = match self.invoke(call.function as usize, callee_frame) {
                    Ok(r) => r,
                    Err(e) => fail!(e),
                };
                let dst = call.dst;
                match (ret, dst.ty) {
                    (Ret::Num(v), Ty::Num | Ty::Bool) => frame.nums[dst.reg as usize] = v,
                    (Ret::List(l), Ty::List) => frame.lists[dst.reg as usize] = l,
                    (Ret::Val(v), Ty::Val) => frame.vals[dst.reg as usize] = v,
                    // A typed callee that exited through `?` hands back its `Tokeo` error;
                    // propagate it like the evaluator's early return.
                    (Ret::Val(err @ Value::Tokeo(Err(_))), _) => finish!(Ret::Val(err)),
                    (Ret::Val(v), ty) => {
                        if let Err(e) = store_value(frame, dst, v) {
                            fail!(e);
                        }
                        let _ = ty;
                    }
                    (Ret::Num(v), Ty::Val) => {
                        frame.vals[dst.reg as usize] = if callee.ret == Ty::Bool {
                            Value::Ukweli(v != 0.0)
                        } else {
                            Value::Namba(v)
                        }
                    }
                    (Ret::List(l), Ty::Val) => {
                        frame.vals[dst.reg as usize] =
                            Value::list(l.iter().map(Value::Namba).collect())
                    }
                    _ => fail!(type_err("aina ya thamani ya kurudi si sahihi")),
                }
            }
            Opcode::CallBuiltin(call) => {
                let args = self.take_args(&frame.vals, &call.args);
                let builtin = call.builtin as usize;
                if asili_trace::on() {
                    let name = self.builtin_index.iter().find(|(_, i)| **i == builtin);
                    asili_trace::emit(
                        asili_trace::Tukio::MwitoMfumo,
                        name.map_or("", |(n, _)| n.as_str()),
                        0,
                    );
                }
                let result = match self.spawners.iter().position(|s| *s == builtin) {
                    // Threads that run this program's `kazi` on this engine.
                    // `anzisha`: a `sawia` task on this host.
                    Some(3) => self.anzisha(&args),
                    Some(which) => self.shared_program().and_then(|shared| match which {
                        0 => crate::builtins::sambamba::tenda(&shared, &args),
                        1 => crate::builtins::mkondo::mkondo_tumikia(&shared, &args),
                        _ => crate::builtins::http::mkondo_tumikia_http(&shared, &args),
                    }),
                    None => (self.builtins[builtin])(&args),
                };
                self.give_args(args);
                match result {
                    Ok(v) => frame.vals[call.dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::CallMethod(call) => {
                let mut args = self.take_args(&frame.vals, &call.args);
                let recv = &frame.vals[call.recv as usize];
                // A state-free method reads the receiver in place (no copy of a string or list).
                let result = if methods::is_pure_name(recv, call.method) {
                    methods::pure_method(recv, call.method.as_str(), &args)
                } else {
                    let recv = recv.clone();
                    self.call_method(recv, call.method.as_str(), std::mem::take(&mut args))
                };
                self.give_args(args);
                match result {
                    Ok(v) => frame.vals[call.dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::MutMethod(call) => {
                let mut args = self.take_args(&frame.vals, &call.args);
                let target = &mut frame.vals[call.recv as usize];
                let result = if methods::is_mutating_name(target, call.method) {
                    methods::mutate(target, call.method.as_str(), &args)
                } else {
                    let recv = target.clone();
                    self.call_method(recv, call.method.as_str(), std::mem::take(&mut args))
                };
                self.give_args(args);
                match result {
                    Ok(v) => frame.vals[call.dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            // Handled above.
            Opcode::MakeStruct { .. }
            | Opcode::Field { .. }
            | Opcode::FieldNum { .. }
            | Opcode::IterItem { .. }
            | Opcode::MakeEnum { .. }
            | Opcode::MakeMap { .. }
            | Opcode::MatchPattern { .. }
            | Opcode::Mov { .. }
            | Opcode::Add { .. }
            | Opcode::Sub { .. }
            | Opcode::Mul { .. }
            | Opcode::Div { .. }
            | Opcode::Rem { .. }
            | Opcode::Pow { .. }
            | Opcode::BitAnd { .. }
            | Opcode::BitOr { .. }
            | Opcode::BitXor { .. }
            | Opcode::Shl { .. }
            | Opcode::Shr { .. }
            | Opcode::Neg { .. }
            | Opcode::BitNot { .. }
            | Opcode::Not { .. }
            | Opcode::Floor { .. }
            | Opcode::Ceil { .. }
            | Opcode::Trunc { .. }
            | Opcode::Cmp { .. } => fail!(internal("maagizo ya namba")),
            Opcode::Jump { .. }
            | Opcode::JumpIfFalse { .. }
            | Opcode::JumpIfTrue { .. }
            | Opcode::JumpIfNot { .. }
            | Opcode::ForStep { .. }
            | Opcode::Return { .. }
            | Opcode::ReturnTupu => {
                fail!(EvalError::Unknown(
                    "amri ya udhibiti nje ya mzunguko".into()
                ))
            }
        }
        Flow::Next
    }
}

/// Execute a pure `nums`-register instruction; `false` if `op` is not one (the reference numeric
/// semantics native code must match bit for bit).
#[inline(always)]
fn numeric_op(op: &Opcode, n: &mut [f64]) -> bool {
    let r = |x: &Reg| *x as usize;
    match op {
        Opcode::Mov { dst, src } => n[r(dst)] = n[r(src)],
        Opcode::Add { dst, a, b } => n[r(dst)] = n[r(a)] + n[r(b)],
        Opcode::Sub { dst, a, b } => n[r(dst)] = n[r(a)] - n[r(b)],
        Opcode::Mul { dst, a, b } => n[r(dst)] = n[r(a)] * n[r(b)],
        Opcode::Div { dst, a, b } => n[r(dst)] = n[r(a)] / n[r(b)],
        Opcode::Rem { dst, a, b } => n[r(dst)] = n[r(a)] % n[r(b)],
        Opcode::Pow { dst, a, b } => n[r(dst)] = n[r(a)].powf(n[r(b)]),
        Opcode::BitAnd { dst, a, b } => n[r(dst)] = ((n[r(a)] as i64) & (n[r(b)] as i64)) as f64,
        Opcode::BitOr { dst, a, b } => n[r(dst)] = ((n[r(a)] as i64) | (n[r(b)] as i64)) as f64,
        Opcode::BitXor { dst, a, b } => n[r(dst)] = ((n[r(a)] as i64) ^ (n[r(b)] as i64)) as f64,
        Opcode::Shl { dst, a, b } => {
            n[r(dst)] = (n[r(a)] as i64).wrapping_shl(shift_amount(n[r(b)])) as f64
        }
        Opcode::Shr { dst, a, b } => {
            n[r(dst)] = (n[r(a)] as i64).wrapping_shr(shift_amount(n[r(b)])) as f64
        }
        Opcode::Neg { dst, src } => n[r(dst)] = -n[r(src)],
        Opcode::BitNot { dst, src } => n[r(dst)] = !(n[r(src)] as i64) as f64,
        Opcode::Not { dst, src } => n[r(dst)] = flag(n[r(src)] == 0.0),
        Opcode::Floor { dst, src } => n[r(dst)] = n[r(src)].floor(),
        Opcode::Ceil { dst, src } => n[r(dst)] = n[r(src)].ceil(),
        Opcode::Trunc { dst, src } => n[r(dst)] = (n[r(src)] as i64) as f64,
        Opcode::Cmp { op, dst, a, b } => n[r(dst)] = flag(compare(*op, n[r(a)], n[r(b)])),
        _ => return false,
    }
    true
}

/// A register's value as a generic value.
fn operand_value(frame: &Frame, op: Operand) -> Value {
    match op.ty {
        Ty::Num => Value::Namba(frame.nums[op.reg as usize]),
        Ty::Bool => Value::Ukweli(frame.nums[op.reg as usize] != 0.0),
        Ty::List => Value::list(
            frame.lists[op.reg as usize]
                .iter()
                .map(Value::Namba)
                .collect(),
        ),
        Ty::Val => frame.vals[op.reg as usize].clone(),
    }
}

/// An instruction found the host in a state the compiler never produces: an error the program
/// sees (and its safe state handles), not a crash.
fn internal(what: &str) -> EvalError {
    EvalError::Unknown(format!("kosa la ndani la Asili ({what})"))
}

fn copy_operand(from: &Frame, src: Operand, to: &mut Frame, dst: Operand) {
    match dst.ty {
        Ty::Num | Ty::Bool => to.nums[dst.reg as usize] = from.nums[src.reg as usize],
        Ty::List => to.lists[dst.reg as usize].clone_from(&from.lists[src.reg as usize]),
        Ty::Val => to.vals[dst.reg as usize] = from.vals[src.reg as usize].clone(),
    }
}

fn list_from_value(v: &Value) -> Result<NumList, EvalError> {
    match v {
        Value::Orodha(items) => items
            .iter()
            .map(|item| match item {
                Value::Namba(n) => Ok(*n),
                _ => Err(type_err("Orodha<Namba> inahitaji Namba tu")),
            })
            .collect(),
        _ => Err(type_err("thamani si Orodha<Namba>")),
    }
}

/// Store a generic value into a typed register.
fn store_value(frame: &mut Frame, dst: Operand, v: Value) -> Result<(), EvalError> {
    match dst.ty {
        Ty::Num => match v {
            Value::Namba(n) => frame.nums[dst.reg as usize] = n,
            other => {
                return Err(EvalError::TypeErr(format!(
                    "operesheni inahitaji Namba, ilipata {other:?}"
                )))
            }
        },
        Ty::Bool => frame.nums[dst.reg as usize] = flag(matches!(v, Value::Ukweli(true))),
        Ty::List => frame.lists[dst.reg as usize] = list_from_value(&v)?,
        Ty::Val => frame.vals[dst.reg as usize] = v,
    }
    Ok(())
}

impl crate::kazi_sawia::Context for Host<'_> {
    fn swap_context(&mut self, saved: &mut [u64; 2]) {
        std::mem::swap(&mut self.depth, &mut saved[0]);
        std::mem::swap(&mut self.stack_limit, &mut saved[1]);
    }

    fn call_index(&mut self, index: usize, args: Vec<Value>) -> Result<Value, EvalError> {
        self.call_values(index, args)
    }
}
