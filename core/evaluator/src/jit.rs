//! Native code generation for ASB bytecode with Cranelift.
//!
//! Every [`BytecodeFunc`] is translated into one machine-code function when a program is loaded.
//! The translation is deliberately direct, because the register VM already did the hard part
//! (static types and register allocation into typed files):
//!
//! * each `nums` register becomes a Cranelift SSA variable, so numeric locals live in machine
//!   registers and numeric literals are folded as constants;
//! * each `Orodha<Namba>` register caches its data pointer and length in two variables, so
//!   `b[i]?` and `b[i] = v` compile to a bounds check plus one load/store;
//! * arithmetic, bitwise operators, comparisons, branches and counted loops compile to native
//!   instructions with exactly the interpreter's semantics (saturating float/int conversions,
//!   the `0..=63` shift rule, `fmod` remainders);
//! * everything that touches generic `Value`s (strings, calls, builtins, methods, list growth)
//!   calls back into the interpreter's `exec_slow`, spilling only the numeric registers that
//!   instruction reads and reloading only those it writes.
//!
//! Native code therefore never changes behaviour, only speed, and any instruction can fall back
//! to the interpreter. Set `ASILI_JIT=0` to run the interpreter alone.

use crate::bytecode::{BytecodeFunc, BytecodeProgram, CmpOp, Frame, Opcode, Reg, Ty};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{types, AbiParam, Block, InstBuilder, MemFlagsData, Signature, Value};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::BTreeSet;
use std::ffi::c_void;

/// `fn(vm, frame, nums) -> status << 32 | pc`.
pub(crate) type NativeFn = unsafe extern "C" fn(*mut c_void, *mut Frame, *mut f64) -> u64;

/// Status codes in the upper half of a native function's return value.
pub(crate) const STATUS_FINISH: u64 = 1;
pub(crate) const STATUS_FAIL: u64 = 2;
/// The function reached the `Return`/`ReturnTupu` instruction at `pc` (lower half).
pub(crate) const STATUS_RETURN: u64 = 3;

/// Callbacks into the VM, provided by `bytecode.rs`.
pub(crate) struct Callbacks {
    /// `(vm, frame, function, pc) -> 0 | STATUS_FINISH | STATUS_FAIL`
    pub exec: extern "C" fn(*mut c_void, *mut Frame, u32, u32) -> u32,
}

pub(crate) struct Jit {
    // Owns the executable memory the function pointers point into.
    _module: JITModule,
    pub funcs: Vec<Option<NativeFn>>,
}

extern "C" fn list_ptr(frame: *mut Frame, reg: u32) -> *mut f64 {
    // SAFETY: called by native code with the frame it was handed; `reg` was checked at compile
    // time to be below the function's `list_regs`.
    let frame = unsafe { &mut *frame };
    frame.lists[reg as usize].as_mut_ptr()
}

extern "C" fn list_len(frame: *mut Frame, reg: u32) -> i64 {
    // SAFETY: as in `list_ptr`.
    let frame = unsafe { &*frame };
    frame.lists[reg as usize].len() as i64
}

extern "C" fn fmod(a: f64, b: f64) -> f64 {
    a % b
}

extern "C" fn powf(a: f64, b: f64) -> f64 {
    a.powf(b)
}

pub(crate) fn enabled() -> bool {
    std::env::var("ASILI_JIT").map(|v| v != "0").unwrap_or(true)
}

/// Compile every function of `program`. Functions that fail to compile stay interpreted.
pub(crate) fn compile_program(program: &BytecodeProgram, callbacks: &Callbacks) -> Option<Jit> {
    let mut flags = settings::builder();
    flags.set("opt_level", "speed").ok()?;
    let isa = cranelift_native::builder()
        .ok()?
        .finish(settings::Flags::new(flags))
        .ok()?;
    let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    builder.symbol("asili_exec", callbacks.exec as *const u8);
    builder.symbol("asili_list_ptr", list_ptr as *const u8);
    builder.symbol("asili_list_len", list_len as *const u8);
    builder.symbol("asili_fmod", fmod as *const u8);
    builder.symbol("asili_powf", powf as *const u8);
    let mut module = JITModule::new(builder);
    let helpers = Helpers::declare(&mut module)?;

    let mut ids: Vec<Option<FuncId>> = Vec::with_capacity(program.functions.len());
    let mut ctx = module.make_context();
    let mut fctx = FunctionBuilderContext::new();
    for (index, function) in program.functions.iter().enumerate() {
        ctx.func.signature = native_signature(&module);
        let id = module
            .declare_function(
                &format!("asili_fn_{index}"),
                Linkage::Local,
                &ctx.func.signature,
            )
            .ok()?;
        let ok = translate(
            &mut module,
            &helpers,
            &mut ctx.func,
            &mut fctx,
            index,
            function,
        );
        let defined = ok && module.define_function(id, &mut ctx).is_ok();
        module.clear_context(&mut ctx);
        ids.push(defined.then_some(id));
    }
    module.finalize_definitions().ok()?;
    let funcs = ids
        .into_iter()
        .map(|id| {
            id.map(|id| {
                let ptr = module.get_finalized_function(id);
                // SAFETY: `ptr` is a finalized function with `native_signature`.
                unsafe { std::mem::transmute::<*const u8, NativeFn>(ptr) }
            })
        })
        .collect();
    Some(Jit {
        _module: module,
        funcs,
    })
}

fn native_signature(module: &JITModule) -> Signature {
    let ptr = module.target_config().pointer_type();
    let mut sig = module.make_signature();
    sig.params.push(AbiParam::new(ptr));
    sig.params.push(AbiParam::new(ptr));
    sig.params.push(AbiParam::new(ptr));
    sig.returns.push(AbiParam::new(types::I64));
    sig
}

struct Helpers {
    exec: FuncId,
    list_ptr: FuncId,
    list_len: FuncId,
    fmod: FuncId,
    powf: FuncId,
}

impl Helpers {
    fn declare(module: &mut JITModule) -> Option<Self> {
        let ptr = module.target_config().pointer_type();
        let sig = |module: &JITModule, params: &[types::Type], ret: types::Type| {
            let mut s = module.make_signature();
            for p in params {
                s.params.push(AbiParam::new(*p));
            }
            s.returns.push(AbiParam::new(ret));
            s
        };
        let exec = sig(module, &[ptr, ptr, types::I32, types::I32], types::I32);
        let lptr = sig(module, &[ptr, types::I32], ptr);
        let llen = sig(module, &[ptr, types::I32], types::I64);
        let f2 = sig(module, &[types::F64, types::F64], types::F64);
        Some(Helpers {
            exec: module
                .declare_function("asili_exec", Linkage::Import, &exec)
                .ok()?,
            list_ptr: module
                .declare_function("asili_list_ptr", Linkage::Import, &lptr)
                .ok()?,
            list_len: module
                .declare_function("asili_list_len", Linkage::Import, &llen)
                .ok()?,
            fmod: module
                .declare_function("asili_fmod", Linkage::Import, &f2)
                .ok()?,
            powf: module
                .declare_function("asili_powf", Linkage::Import, &f2)
                .ok()?,
        })
    }
}

/// Numeric registers an instruction reads when executed by `exec_slow`.
fn num_reads(op: &Opcode) -> Vec<Reg> {
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
fn num_writes(op: &Opcode) -> Vec<Reg> {
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
fn list_writes(op: &Opcode) -> Vec<Reg> {
    match op {
        Opcode::MakeNumList { dst, .. }
        | Opcode::ListMov { dst, .. }
        | Opcode::ListFromVal { dst, .. } => vec![*dst],
        Opcode::ListPush { list, .. }
        | Opcode::ListRemove { list, .. }
        | Opcode::ListRemoveVal { list, .. } => vec![*list],
        Opcode::Call(call) if call.dst.ty == Ty::List => vec![call.dst.reg],
        _ => Vec::new(),
    }
}

fn float_cc(op: CmpOp) -> FloatCC {
    match op {
        CmpOp::Lt => FloatCC::LessThan,
        CmpOp::Le => FloatCC::LessThanOrEqual,
        CmpOp::Gt => FloatCC::GreaterThan,
        CmpOp::Ge => FloatCC::GreaterThanOrEqual,
        CmpOp::Eq => FloatCC::Equal,
        CmpOp::Ne => FloatCC::NotEqual,
    }
}

struct Translator<'a, 'b> {
    b: FunctionBuilder<'b>,
    module: &'a mut JITModule,
    helpers: &'a Helpers,
    function: &'a BytecodeFunc,
    index: u32,
    nums: Vec<Variable>,
    list_ptrs: Vec<Variable>,
    list_lens: Vec<Variable>,
    vm: Value,
    frame: Value,
    nums_ptr: Value,
    blocks: Vec<Option<Block>>,
    ptr_ty: types::Type,
}

fn translate(
    module: &mut JITModule,
    helpers: &Helpers,
    func: &mut cranelift_codegen::ir::Function,
    fctx: &mut FunctionBuilderContext,
    index: usize,
    function: &BytecodeFunc,
) -> bool {
    let code = &function.code;
    // Block leaders: entry, jump targets, and fall-through successors of branches.
    let mut leaders = BTreeSet::new();
    leaders.insert(0usize);
    for (pc, op) in code.iter().enumerate() {
        match op {
            Opcode::Jump { target }
            | Opcode::JumpIfFalse { target, .. }
            | Opcode::JumpIfTrue { target, .. }
            | Opcode::JumpIfNot { target, .. }
            | Opcode::ForStep { target, .. } => {
                if *target as usize >= code.len() {
                    return false;
                }
                leaders.insert(*target as usize);
                leaders.insert(pc + 1);
            }
            Opcode::Return { .. } | Opcode::ReturnTupu => {
                leaders.insert(pc + 1);
            }
            _ => {}
        }
    }
    let ptr_ty = module.target_config().pointer_type();
    let mut b = FunctionBuilder::new(func, fctx);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    let mut blocks = vec![None; code.len() + 1];
    for &l in &leaders {
        if l < code.len() {
            blocks[l] = Some(b.create_block());
        }
    }
    b.switch_to_block(entry);
    let params = b.block_params(entry).to_vec();
    let (vm, frame, nums_ptr) = (params[0], params[1], params[2]);

    let nums: Vec<Variable> = (0..function.num_regs)
        .map(|_| b.declare_var(types::F64))
        .collect();
    let list_ptrs: Vec<Variable> = (0..function.list_regs)
        .map(|_| b.declare_var(ptr_ty))
        .collect();
    let list_lens: Vec<Variable> = (0..function.list_regs)
        .map(|_| b.declare_var(types::I64))
        .collect();

    let mut t = Translator {
        b,
        module,
        helpers,
        function,
        index: index as u32,
        nums,
        list_ptrs,
        list_lens,
        vm,
        frame,
        nums_ptr,
        blocks,
        ptr_ty,
    };
    t.prologue();
    let first = t.blocks[0].expect("entry leader");
    t.b.ins().jump(first, &[]);

    let mut terminated = true;
    for (pc, op) in code.iter().enumerate() {
        if let Some(block) = t.blocks[pc] {
            if !terminated {
                t.b.ins().jump(block, &[]);
            }
            t.b.switch_to_block(block);
        } else if terminated {
            // Unreachable code after a terminator that is not a jump target.
            continue;
        }
        terminated = t.instruction(pc, op);
    }
    if !terminated {
        // Code always ends with `ReturnTupu`; keep the verifier happy regardless.
        let pc = code.len().saturating_sub(1);
        t.return_status(STATUS_RETURN, pc);
    }
    t.b.seal_all_blocks();
    let config = t.module.target_config();
    t.b.finalize(config);
    true
}

impl Translator<'_, '_> {
    fn prologue(&mut self) {
        let consts: std::collections::HashMap<Reg, f64> =
            self.function.num_consts.iter().copied().collect();
        let param_nums: BTreeSet<Reg> = self
            .function
            .params
            .iter()
            .filter(|p| matches!(p.ty, Ty::Num | Ty::Bool))
            .map(|p| p.reg)
            .collect();
        for reg in 0..self.function.num_regs {
            let value = if let Some(n) = consts.get(&reg) {
                self.b.ins().f64const(*n)
            } else if param_nums.contains(&reg) {
                self.load_num(reg)
            } else {
                self.b.ins().f64const(0.0)
            };
            self.b.def_var(self.nums[reg as usize], value);
        }
        let param_lists: BTreeSet<Reg> = self
            .function
            .params
            .iter()
            .filter(|p| p.ty == Ty::List)
            .map(|p| p.reg)
            .collect();
        for reg in 0..self.function.list_regs {
            if param_lists.contains(&reg) {
                self.refresh_list(reg);
            } else {
                let zero_ptr = self.b.ins().iconst(self.ptr_ty, 0);
                let zero = self.b.ins().iconst(types::I64, 0);
                self.b.def_var(self.list_ptrs[reg as usize], zero_ptr);
                self.b.def_var(self.list_lens[reg as usize], zero);
            }
        }
    }

    fn get(&mut self, reg: Reg) -> Value {
        self.b.use_var(self.nums[reg as usize])
    }

    fn set(&mut self, reg: Reg, value: Value) {
        self.b.def_var(self.nums[reg as usize], value);
    }

    fn load_num(&mut self, reg: Reg) -> Value {
        self.b.ins().load(
            types::F64,
            MemFlagsData::trusted(),
            self.nums_ptr,
            (reg as i32) * 8,
        )
    }

    fn spill(&mut self, reg: Reg) {
        let v = self.get(reg);
        self.b
            .ins()
            .store(MemFlagsData::trusted(), v, self.nums_ptr, (reg as i32) * 8);
    }

    fn reload(&mut self, reg: Reg) {
        let v = self.load_num(reg);
        self.set(reg, v);
    }

    fn call(&mut self, id: FuncId, args: &[Value]) -> Value {
        let func_ref = self.module.declare_func_in_func(id, self.b.func);
        let call = self.b.ins().call(func_ref, args);
        self.b.inst_results(call)[0]
    }

    fn refresh_list(&mut self, reg: Reg) {
        let r = self.b.ins().iconst(types::I32, reg as i64);
        let ptr = self.call(self.helpers.list_ptr, &[self.frame, r]);
        let r = self.b.ins().iconst(types::I32, reg as i64);
        let len = self.call(self.helpers.list_len, &[self.frame, r]);
        self.b.def_var(self.list_ptrs[reg as usize], ptr);
        self.b.def_var(self.list_lens[reg as usize], len);
    }

    fn to_int(&mut self, v: Value) -> Value {
        self.b.ins().fcvt_to_sint_sat(types::I64, v)
    }

    fn to_float(&mut self, v: Value) -> Value {
        self.b.ins().fcvt_from_sint(types::F64, v)
    }

    fn flag(&mut self, cond: Value) -> Value {
        let one = self.b.ins().f64const(1.0);
        let zero = self.b.ins().f64const(0.0);
        self.b.ins().select(cond, one, zero)
    }

    fn return_status(&mut self, status: u64, pc: usize) {
        let v = self
            .b
            .ins()
            .iconst(types::I64, ((status << 32) | pc as u64) as i64);
        self.b.ins().return_(&[v]);
    }

    fn block_at(&self, pc: u32) -> Block {
        self.blocks[pc as usize].expect("jump target is a leader")
    }

    fn next_block(&self, pc: usize) -> Block {
        self.blocks[pc + 1].expect("branch successor is a leader")
    }

    /// Execute instruction `pc` in the interpreter; returns from the native function when the
    /// instruction finished or failed the call.
    fn slow(&mut self, pc: usize, op: &Opcode) {
        for reg in num_reads(op) {
            self.spill(reg);
        }
        let f = self.b.ins().iconst(types::I32, self.index as i64);
        let p = self.b.ins().iconst(types::I32, pc as i64);
        let status = self.call(self.helpers.exec, &[self.vm, self.frame, f, p]);
        let done = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(status, done, &[], cont, &[]);
        self.b.switch_to_block(done);
        let wide = self.b.ins().uextend(types::I64, status);
        let shifted = self.b.ins().ishl_imm_u(wide, 32);
        let pc_v = self.b.ins().iconst(types::I64, pc as i64);
        let ret = self.b.ins().bor(shifted, pc_v);
        self.b.ins().return_(&[ret]);
        self.b.switch_to_block(cont);
        for reg in num_writes(op) {
            self.reload(reg);
        }
        for reg in list_writes(op) {
            self.refresh_list(reg);
        }
    }

    /// Bounds-checked element address; branches to `slow_path` when out of range.
    fn element(&mut self, list: Reg, idx: Reg, pc: usize, op: &Opcode) -> Value {
        let n = self.get(idx);
        let i = self.to_int(n);
        let zero = self.b.ins().iconst(types::I64, 0);
        let i = self.b.ins().smax(i, zero);
        let len = self.b.use_var(self.list_lens[list as usize]);
        let in_bounds = self.b.ins().icmp(IntCC::UnsignedLessThan, i, len);
        let fast = self.b.create_block();
        let oob = self.b.create_block();
        self.b.ins().brif(in_bounds, fast, &[], oob, &[]);
        self.b.switch_to_block(oob);
        // The interpreter produces the exact `KosaMipaka`/`ingiza` error and exits.
        self.slow(pc, op);
        self.return_status(STATUS_FAIL, pc);
        self.b.switch_to_block(fast);
        let base = self.b.use_var(self.list_ptrs[list as usize]);
        let offset = self.b.ins().ishl_imm_u(i, 3);
        self.b.ins().iadd(base, offset)
    }

    /// Translate one instruction; returns whether it ended the current block.
    fn instruction(&mut self, pc: usize, op: &Opcode) -> bool {
        macro_rules! bin {
            ($dst:expr, $a:expr, $b:expr, $f:ident) => {{
                let x = self.get(*$a);
                let y = self.get(*$b);
                let r = self.b.ins().$f(x, y);
                self.set(*$dst, r);
            }};
        }
        macro_rules! int_bin {
            ($dst:expr, $a:expr, $b:expr, $f:ident) => {{
                let x = self.get(*$a);
                let y = self.get(*$b);
                let xi = self.to_int(x);
                let yi = self.to_int(y);
                let r = self.b.ins().$f(xi, yi);
                let r = self.to_float(r);
                self.set(*$dst, r);
            }};
        }
        match op {
            Opcode::Mov { dst, src } => {
                let v = self.get(*src);
                self.set(*dst, v);
            }
            Opcode::Add { dst, a, b } => bin!(dst, a, b, fadd),
            Opcode::Sub { dst, a, b } => bin!(dst, a, b, fsub),
            Opcode::Mul { dst, a, b } => bin!(dst, a, b, fmul),
            Opcode::Div { dst, a, b } => bin!(dst, a, b, fdiv),
            Opcode::Rem { dst, a, b } => {
                let x = self.get(*a);
                let y = self.get(*b);
                let r = self.remainder(x, y);
                self.set(*dst, r);
            }
            Opcode::Pow { dst, a, b } => {
                let x = self.get(*a);
                let y = self.get(*b);
                let r = self.call(self.helpers.powf, &[x, y]);
                self.set(*dst, r);
            }
            Opcode::BitAnd { dst, a, b } => int_bin!(dst, a, b, band),
            Opcode::BitOr { dst, a, b } => int_bin!(dst, a, b, bor),
            Opcode::BitXor { dst, a, b } => int_bin!(dst, a, b, bxor),
            Opcode::Shl { dst, a, b } | Opcode::Shr { dst, a, b } => {
                let x = self.get(*a);
                let y = self.get(*b);
                let xi = self.to_int(x);
                // `shift_amount`: saturating i32, anything outside 0..=63 shifts by 0.
                let s = self.b.ins().fcvt_to_sint_sat(types::I32, y);
                let too_big = self.b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, s, 63);
                let zero = self.b.ins().iconst(types::I32, 0);
                let s = self.b.ins().select(too_big, zero, s);
                let r = if matches!(op, Opcode::Shl { .. }) {
                    self.b.ins().ishl(xi, s)
                } else {
                    self.b.ins().sshr(xi, s)
                };
                let r = self.to_float(r);
                self.set(*dst, r);
            }
            Opcode::Neg { dst, src } => {
                let v = self.get(*src);
                let r = self.b.ins().fneg(v);
                self.set(*dst, r);
            }
            Opcode::BitNot { dst, src } => {
                let v = self.get(*src);
                let i = self.to_int(v);
                let r = self.b.ins().bnot(i);
                let r = self.to_float(r);
                self.set(*dst, r);
            }
            Opcode::Not { dst, src } => {
                let v = self.get(*src);
                let zero = self.b.ins().f64const(0.0);
                let c = self.b.ins().fcmp(FloatCC::Equal, v, zero);
                let r = self.flag(c);
                self.set(*dst, r);
            }
            Opcode::Floor { dst, src } => {
                let v = self.get(*src);
                let r = self.b.ins().floor(v);
                self.set(*dst, r);
            }
            Opcode::Ceil { dst, src } => {
                let v = self.get(*src);
                let r = self.b.ins().ceil(v);
                self.set(*dst, r);
            }
            Opcode::Trunc { dst, src } => {
                let v = self.get(*src);
                let i = self.to_int(v);
                let r = self.to_float(i);
                self.set(*dst, r);
            }
            Opcode::Cmp { op, dst, a, b } => {
                let x = self.get(*a);
                let y = self.get(*b);
                let c = self.b.ins().fcmp(float_cc(*op), x, y);
                let r = self.flag(c);
                self.set(*dst, r);
            }
            Opcode::Jump { target } => {
                let t = self.block_at(*target);
                self.b.ins().jump(t, &[]);
                return true;
            }
            Opcode::JumpIfFalse { cond, target } | Opcode::JumpIfTrue { cond, target } => {
                let v = self.get(*cond);
                let zero = self.b.ins().f64const(0.0);
                let cc = if matches!(op, Opcode::JumpIfFalse { .. }) {
                    FloatCC::Equal
                } else {
                    FloatCC::NotEqual
                };
                let c = self.b.ins().fcmp(cc, v, zero);
                let (t, next) = (self.block_at(*target), self.next_block(pc));
                self.b.ins().brif(c, t, &[], next, &[]);
                return true;
            }
            Opcode::JumpIfNot { op, a, b, target } => {
                let x = self.get(*a);
                let y = self.get(*b);
                let c = self.b.ins().fcmp(float_cc(*op), x, y);
                let (t, next) = (self.block_at(*target), self.next_block(pc));
                self.b.ins().brif(c, next, &[], t, &[]);
                return true;
            }
            Opcode::ForStep { ctr, end, target } => {
                let x = self.get(*ctr);
                let one = self.b.ins().f64const(1.0);
                let next_v = self.b.ins().fadd(x, one);
                self.set(*ctr, next_v);
                let e = self.get(*end);
                let c = self.b.ins().fcmp(FloatCC::LessThan, next_v, e);
                let (t, next) = (self.block_at(*target), self.next_block(pc));
                self.b.ins().brif(c, t, &[], next, &[]);
                return true;
            }
            Opcode::ListGet { dst, list, idx } => {
                let addr = self.element(*list, *idx, pc, op);
                let v = self
                    .b
                    .ins()
                    .load(types::F64, MemFlagsData::trusted(), addr, 0);
                self.set(*dst, v);
            }
            Opcode::ListSet { list, idx, src } => {
                let addr = self.element(*list, *idx, pc, op);
                let v = self.get(*src);
                self.b.ins().store(MemFlagsData::trusted(), v, addr, 0);
            }
            Opcode::ListLen { dst, list } => {
                let len = self.b.use_var(self.list_lens[*list as usize]);
                let r = self.to_float(len);
                self.set(*dst, r);
            }
            Opcode::Return { src } => {
                if matches!(src.ty, Ty::Num | Ty::Bool) {
                    self.spill(src.reg);
                }
                self.return_status(STATUS_RETURN, pc);
                return true;
            }
            Opcode::ReturnTupu => {
                self.return_status(STATUS_RETURN, pc);
                return true;
            }
            other => self.slow(pc, other),
        }
        false
    }

    /// `a % b` with Rust/`fmod` semantics. Integral operands (the common case: indices,
    /// counters, bit masks) use one integer division; everything else calls `fmod`.
    fn remainder(&mut self, a: Value, b: Value) -> Value {
        let ai = self.to_int(a);
        let bi = self.to_int(b);
        let af = self.to_float(ai);
        let bf = self.to_float(bi);
        let a_exact = self.b.ins().fcmp(FloatCC::Equal, af, a);
        let b_exact = self.b.ins().fcmp(FloatCC::Equal, bf, b);
        let nonzero = self.b.ins().icmp_imm_s(IntCC::NotEqual, bi, 0);
        let not_minus_one = self.b.ins().icmp_imm_s(IntCC::NotEqual, bi, -1);
        let ok = self.b.ins().band(a_exact, b_exact);
        let ok = self.b.ins().band(ok, nonzero);
        let ok = self.b.ins().band(ok, not_minus_one);
        let fast = self.b.create_block();
        let slow = self.b.create_block();
        let merge = self.b.create_block();
        self.b.append_block_param(merge, types::F64);
        self.b.ins().brif(ok, fast, &[], slow, &[]);
        self.b.switch_to_block(fast);
        let r = self.b.ins().srem(ai, bi);
        let rf = self.to_float(r);
        // fmod's result carries the dividend's sign, including -0.0.
        let rf = self.b.ins().fcopysign(rf, a);
        self.b.ins().jump(merge, &[rf.into()]);
        self.b.switch_to_block(slow);
        let r = self.call(self.helpers.fmod, &[a, b]);
        self.b.ins().jump(merge, &[r.into()]);
        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }
}
