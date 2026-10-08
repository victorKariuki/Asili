//! The host side of native code: what the machine code `nguvu` generates calls back into.
//!
//! A [`Host`] owns one program's call frames (typed register files), its builtin table and
//! constants, and runs every bytecode function as native code — nothing interprets bytecode. Instructions native code does not compile (strings, maps, structs, builtins,
//! method calls, …) come back through [`native_exec`] one at a time and run here on the shared
//! semantics (`eval::ops`, `eval::methods`); `kazi` the compiler left to the tree-walker run on a
//! [`crate::TreeContext`].

use crate::builtins::BuiltinFn;
use crate::bytecode::{
    ast_function, BytecodeFunc, BytecodeProgram, CmpOp, IndexMode, Opcode, Operand, StoredConstant,
    Ty, UnaryCode,
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

// `repr(C)` with `depth` first: native code's direct calls count themselves in it at offset 0
// (`native::DEPTH_OFFSET`), so the call-depth limit is the same on every tier.
#[repr(C)]
pub(crate) struct Host<'p> {
    depth: usize,
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
    /// Builtin indices of `tenda`, `mkondo_tumikia` and `mkondo_tumikia_http`, which need the
    /// program itself.
    spawners: [usize; 3],
    /// Argument buffer for builtin and method calls, reused so a call allocates nothing (see
    /// [`Host::take_args`]).
    args: Vec<Value>,
    /// Tree-walkers for mixed mode, reused across calls (a nested tree → native → tree call takes
    /// a second one).
    trees: Vec<crate::TreeContext>,
    /// The program's machine code (`nguvu`), one entry per function.
    native: &'p crate::aot::NativeLibrary,
    /// Outcome of an instruction that native code handed to `exec_slow` and that ended the call.
    pending: Option<Flow>,
    /// The error now leaving calls was already traced (by the first call it left).
    error_traced: bool,
}

pub(crate) static NATIVE_RUNTIME: crate::native::Runtime = crate::native::Runtime {
    exec: native_exec,
    list_ptr: crate::native::list_ptr,
    list_len: crate::native::list_len,
    list_push: crate::native::list_push,
    list_remove: crate::native::list_remove,
    fmod: crate::native::rt_fmod,
    pow: crate::native::rt_pow,
    floor: crate::native::rt_floor,
    ceil: crate::native::rt_ceil,
    float_to_int_sat: crate::native::rt_float_to_int_sat,
    shift_amount: crate::native::rt_shift_amount,
    stack_limit: crate::native::rt_stack_limit,
    call_host: native_call_host,
};

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

pub(crate) use crate::runtime::MAX_CALL_DEPTH;

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
    /// describes the program to the threads it starts; without it they run on the tree-walker.
    pub(crate) fn new(
        program: &'p BytecodeProgram,
        native: &'p crate::aot::NativeLibrary,
        shared: Option<crate::spawn::Shared>,
    ) -> Self {
        let table = crate::builtins::BuiltinTable::new();
        Host {
            depth: 0,
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
            trees: Vec::new(),
            native,
            pending: None,
            error_traced: false,
        }
    }

    /// Call the program's `kazi` called `name`.
    pub(crate) fn call_by_name(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, EvalError> {
        let index = self
            .program
            .functions
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| EvalError::UndefinedVar(name.to_string()))?;
        self.call_values(index, args)
    }

    /// This program as other threads receive it: the one the host was started from, or else
    /// its syntax tree for the tree-walker.
    fn shared_program(&mut self) -> crate::spawn::Shared {
        let program = self.program;
        self.shared
            .get_or_insert_with(|| {
                crate::spawn::Shared::Tree(std::sync::Arc::new(
                    program
                        .ast
                        .clone()
                        .expect("bytecode carries its syntax tree"),
                ))
            })
            .clone()
    }

    /// `Neno` constant `k` interned (a struct or field name).
    fn name(&self, k: u32) -> crate::value::Name {
        self.names[k as usize]
    }

    fn frame_for(&mut self, f: &BytecodeFunc) -> Frame {
        let mut frame = self.pool.pop().unwrap_or_default();
        frame.nums.clear();
        frame.nums.resize(f.num_regs as usize, 0.0);
        frame.lists.clear();
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

    fn release(&mut self, frame: Frame) {
        self.pool.push(frame);
    }

    /// Run function `index` — one the compiler left to the tree-walker — on it.
    fn tree_call(&mut self, index: usize, args: Vec<Value>) -> Result<Value, EvalError> {
        let program = self.program;
        let module = program
            .ast
            .as_ref()
            .ok_or_else(|| EvalError::Unknown("kilele hakina mti wa programu".into()))?;
        let name = &program.functions[index].name;
        let f = ast_function(module, name).ok_or_else(|| EvalError::UndefinedVar(name.clone()))?;
        let mut tree = match self.trees.pop() {
            Some(tree) => tree,
            None => crate::TreeContext::new(module)?,
        };
        let hook = crate::runtime::NativeHook {
            host: self as *mut Host<'p> as *mut std::ffi::c_void,
            call: tree_to_native,
            shared: native_shared,
        };
        let result = tree.call(module, f, args, Some(hook));
        self.trees.push(tree);
        result
    }

    /// Call a function with generic arguments, converting to and from its typed registers.
    fn call_values(&mut self, index: usize, args: Vec<Value>) -> Result<Value, EvalError> {
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
        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(EvalError::Unknown("undani mno".into()));
        }
        // Native code makes direct calls only while `DIRECT_CALL_HEADROOM` is free below it, so
        // give it that much (on a fresh segment when the stack is short — or when its size is
        // unknown past the committed pages, as on musl's main thread).
        let native = self.native.funcs[index];
        let red_zone = 64 * 1024 + crate::native::DIRECT_CALL_HEADROOM;
        // Calls native code makes directly to native code bypass the host and are not traced.
        let span = asili_trace::enter(&self.program.functions[index].name, 0);
        let result = stacker::maybe_grow(red_zone, 2 * 1024 * 1024, || {
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
        let status = unsafe { native(&NATIVE_RUNTIME, host, &mut frame, nums) };
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
        Err(EvalError::Unknown(format!(
            "bytecode method haijaungwa mkono: {method}"
        )))
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

    fn exec_slow(&mut self, op: &Opcode, frame: &mut Frame) -> Flow {
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
                let Value::Orodha(items) = &frame.vals[*items as usize] else {
                    unreachable!("IterItems leaves an Orodha")
                };
                frame.vals[*dst as usize] = items[frame.nums[*idx as usize] as usize].clone();
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
                frame.vals[*dst as usize] = Value::Enum(enum_name.clone(), variant.clone(), data);
                return Flow::Next;
            }
            Opcode::MatchPattern {
                dst,
                src,
                pattern,
                binds,
            } => {
                let mut bound: Vec<(Reg, Value)> = Vec::new();
                let matched = crate::eval::expr::match_pattern(
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
        if let Opcode::Interpreted { function } = op {
            let f = &program.functions[*function as usize];
            let args = f.params.iter().map(|p| operand_value(frame, *p)).collect();
            return match self.tree_call(*function as usize, args) {
                Ok(v) => Flow::Finish(ret_of(f.ret, v)),
                Err(e) => Flow::Fail(e),
            };
        }
        let n = &mut frame.nums;
        match op {
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
                let copy = frame.lists[*src as usize].clone();
                frame.lists[*dst as usize] = copy;
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
            Opcode::ValMov { dst, src } => {
                frame.vals[*dst as usize] = frame.vals[*src as usize].clone()
            }
            Opcode::BoxNum { dst, src } => {
                frame.vals[*dst as usize] = Value::Namba(n[*src as usize])
            }
            Opcode::BoxBool { dst, src } => {
                frame.vals[*dst as usize] = Value::Ukweli(n[*src as usize] != 0.0)
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
                let list = std::mem::take(&mut frame.lists[call.recv as usize]);
                let mut value = Value::list(list.iter().map(Value::Namba).collect());
                let result = methods::mutate(&mut value, &call.method, &args);
                match list_from_value(&value) {
                    Ok(list) => frame.lists[call.recv as usize] = list,
                    Err(e) => fail!(e),
                }
                match result {
                    Ok(v) => frame.vals[call.dst as usize] = v,
                    Err(e) => fail!(e),
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
                    Some(which) => {
                        let shared = self.shared_program();
                        match which {
                            0 => crate::builtins::sambamba::tenda(&shared, &args),
                            1 => crate::builtins::mkondo::mkondo_tumikia(&shared, &args),
                            _ => crate::builtins::http::mkondo_tumikia_http(&shared, &args),
                        }
                    }
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
                let result = if methods::is_pure_method(recv, &call.method) {
                    methods::pure_method(recv, &call.method, &args)
                } else {
                    let recv = recv.clone();
                    self.call_method(recv, &call.method, std::mem::take(&mut args))
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
                let result = if methods::is_mutating(target, &call.method) {
                    methods::mutate(target, &call.method, &args)
                } else {
                    let recv = target.clone();
                    self.call_method(recv, &call.method, std::mem::take(&mut args))
                };
                self.give_args(args);
                match result {
                    Ok(v) => frame.vals[call.dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            // Handled above.
            Opcode::Interpreted { .. }
            | Opcode::MakeStruct { .. }
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
            | Opcode::Cmp { .. } => unreachable!("numeric_op handles numeric instructions"),
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

/// A generic result as a function declared to return `ty` returns it.
fn ret_of(ty: Ty, v: Value) -> Ret {
    match (ty, v) {
        (Ty::Num, Value::Namba(n)) => Ret::Num(n),
        (Ty::Bool, Value::Ukweli(b)) => Ret::Num(flag(b)),
        (Ty::List, v @ Value::Orodha(_)) => match list_from_value(&v) {
            Ok(l) => Ret::List(l),
            Err(_) => Ret::Val(v),
        },
        (_, v) => Ret::Val(v),
    }
}

/// The tree-walker calling `name`: native code runs it unless it is one of the
/// tree-walker's own.
/// [`crate::runtime::NativeHook::shared`].
fn native_shared(host: *mut std::ffi::c_void) -> crate::spawn::Shared {
    // SAFETY: as for `tree_to_native`.
    let host = unsafe { &mut *(host as *mut Host<'static>) };
    host.shared_program()
}

fn tree_to_native(
    host: *mut std::ffi::c_void,
    name: &str,
    args: &[Value],
) -> Option<Result<Value, EvalError>> {
    // SAFETY: the pointer is the `Host` whose `tree_call` is running this tree-walker, alive and
    // between instructions for the whole call (the same re-entry native code makes through
    // `native_exec`).
    let host = unsafe { &mut *(host as *mut Host<'static>) };
    let index = host.program.functions.iter().position(|f| f.name == name)?;
    if matches!(
        host.program.functions[index].code.first(),
        Some(Opcode::Interpreted { .. })
    ) {
        return None;
    }
    Some(host.call_values(index, args.to_vec()))
}

fn copy_operand(from: &Frame, src: Operand, to: &mut Frame, dst: Operand) {
    match dst.ty {
        Ty::Num | Ty::Bool => to.nums[dst.reg as usize] = from.nums[src.reg as usize],
        Ty::List => to.lists[dst.reg as usize] = from.lists[src.reg as usize].clone(),
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
