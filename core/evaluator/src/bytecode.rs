//! The ASB bytecode compiler: the input to the native backend (`nguvu`).
//!
//! The compiler lowers a conservative, performance-sensitive subset of Asili to a register
//! machine with three register files per call frame:
//!
//! * `nums`: unboxed `f64`s, holding every `Namba` and `Ukweli` (as `0.0`/`1.0`) local and
//!   temporary whose type is statically known;
//! * `lists`: unboxed [`NumList`]s (integer or `f64` words) for `Orodha<Namba>` locals;
//! * `vals`: generic [`Value`]s for everything else.
//!
//! Numeric code therefore never touches the `Value` enum: `a + b` is one `Add` instruction on two
//! `f64` registers, `ikiwa x < y` is one fused compare-and-branch, `b[i]?` on an
//! `Orodha<Namba>` is one bounds-checked load, and every numeric literal lives in a register that
//! is filled once when the frame is entered. Generic operations reuse the tree-walking
//! evaluator's shared helpers (`eval::methods`) so both execution paths agree on behaviour and
//! error text.
//!
//! A program containing syntax that is not represented here does not compile:
//! [`compile_module_explained`] names the `kazi` and line.

use crate::builtins::builtin_names;
use crate::eval::methods;
use crate::value::{self, EvalError, Value};
use asili_parser::{
    AssignOp, BinaryOp, Block, Expr, ExprId, Exprs, ForMode, Function, Module, Pattern, Stmt,
    UnaryOp,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Register index within one register file of a frame.
pub type Reg = u32;

/// Static type of a register, which also selects its register file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ty {
    /// `Namba`, in the `nums` file.
    Num,
    /// `Ukweli`, in the `nums` file as `0.0`/`1.0`.
    Bool,
    /// `Orodha<Namba>`, in the `lists` file.
    List,
    /// Any other value, in the `vals` file.
    Val,
}

impl Ty {
    fn from_type_name(name: &str) -> Ty {
        match name.replace(' ', "").as_str() {
            "Namba" => Ty::Num,
            "Ukweli" => Ty::Bool,
            "Orodha<Namba>" => Ty::List,
            _ => Ty::Val,
        }
    }

    fn in_nums(self) -> bool {
        matches!(self, Ty::Num | Ty::Bool)
    }
}

/// A typed register reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operand {
    pub ty: Ty,
    pub reg: Reg,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BytecodeFunc {
    pub name: String,
    /// Source line of the `kazi` (its trace span).
    pub line: u32,
    pub params: Vec<Operand>,
    pub ret: Ty,
    pub num_regs: u32,
    pub list_regs: u32,
    pub val_regs: u32,
    /// Numeric constants copied into their registers on frame entry.
    pub num_consts: Vec<(Reg, f64)>,
    pub code: Vec<Opcode>,
}

/// How an indexing instruction treats an out-of-range `Orodha` index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexMode {
    /// Plain `b[i]`: the element; out of range is a runtime error.
    Element,
    /// `b[i]?`: out of range returns the `KosaMipaka` `Tokeo` error from the function
    /// (`ListGet`), or yields the `Tokeo` value (`ValIndex`, for `?`/`jaribu` to unwrap).
    Tokeo,
}

/// Comparison selector for the generic compare-and-branch opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmpOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryCode {
    Neg,
    BitNot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CallOp {
    pub function: u32,
    pub args: Vec<Operand>,
    pub dst: Operand,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuiltinOp {
    pub builtin: u32,
    pub args: Vec<Reg>,
    pub dst: Reg,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MethodOp {
    pub method: asili_parser::Name,
    pub recv: Reg,
    pub args: Vec<Reg>,
    pub dst: Reg,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// An in-place method (`ongeza`, `ingiza`, ...) on a generic local.
pub struct MutMethodOp {
    pub method: asili_parser::Name,
    pub recv: Reg,
    pub args: Vec<Reg>,
    pub dst: Reg,
}

/// Register-machine instructions. Unless noted, `dst`/`a`/`b`/`src` are `nums` registers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Opcode {
    Mov {
        dst: Reg,
        src: Reg,
    },
    Add {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Sub {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Mul {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Div {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Rem {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Pow {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    BitAnd {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    BitOr {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    BitXor {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Shl {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Shr {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Neg {
        dst: Reg,
        src: Reg,
    },
    BitNot {
        dst: Reg,
        src: Reg,
    },
    Not {
        dst: Reg,
        src: Reg,
    },
    Floor {
        dst: Reg,
        src: Reg,
    },
    Ceil {
        dst: Reg,
        src: Reg,
    },
    /// `dst = (a op b) as 0.0/1.0`.
    Cmp {
        op: CmpOp,
        dst: Reg,
        a: Reg,
        b: Reg,
    },

    Jump {
        target: u32,
    },
    JumpIfFalse {
        cond: Reg,
        target: u32,
    },
    JumpIfTrue {
        cond: Reg,
        target: u32,
    },
    /// Jump when `a op b` does **not** hold (the fall-through is the "then" path).
    JumpIfNot {
        op: CmpOp,
        a: Reg,
        b: Reg,
        target: u32,
    },
    /// Counted-loop back edge: `ctr += 1; if ctr < end { goto target }`.
    ForStep {
        ctr: Reg,
        end: Reg,
        target: u32,
    },
    /// `dst = trunc(src)`, used for range-loop bounds.
    Trunc {
        dst: Reg,
        src: Reg,
    },

    /// `lists[dst] = [nums[items]...]`.
    MakeNumList {
        dst: Reg,
        items: Vec<Reg>,
    },
    /// `orodha_rudia(value, count)` into an `Orodha<Namba>` register.
    ListRepeat {
        dst: Reg,
        value: Reg,
        count: Reg,
    },
    /// `b[i]` / `b[i]?` on an `Orodha<Namba>`; `mode` decides what out of range does.
    ListGet {
        dst: Reg,
        list: Reg,
        idx: Reg,
        mode: IndexMode,
    },
    /// `b[i]` without `?`: `vals[dst]` receives the `Tokeo`.
    ListGetTokeo {
        dst: Reg,
        list: Reg,
        idx: Reg,
    },
    ListSet {
        list: Reg,
        idx: Reg,
        src: Reg,
    },
    ListPush {
        list: Reg,
        src: Reg,
    },
    ListRemove {
        list: Reg,
        idx: Reg,
    },
    /// `vals[dst] = Chaguo(removed)`.
    ListRemoveVal {
        dst: Reg,
        list: Reg,
        idx: Reg,
    },
    ListLen {
        dst: Reg,
        list: Reg,
    },
    /// `lists[dst] = lists[src].clone()`.
    ListMov {
        dst: Reg,
        src: Reg,
    },
    /// `lists[dst]` from `vals[src]`, which must be an `Orodha` of `Namba`s.
    ListFromVal {
        dst: Reg,
        src: Reg,
    },
    /// `vals[dst] = Orodha(lists[src])`.
    ListToVal {
        dst: Reg,
        src: Reg,
    },

    /// `vals[dst] = constants[k]`.
    ConstVal {
        dst: Reg,
        k: u32,
    },
    ValMov {
        dst: Reg,
        src: Reg,
    },
    /// `vals[dst] = Namba(nums[src])`.
    BoxNum {
        dst: Reg,
        src: Reg,
    },
    /// `vals[dst] = Ukweli(nums[src] != 0)`.
    BoxBool {
        dst: Reg,
        src: Reg,
    },
    /// `nums[dst]` from `vals[src]`, which must be a `Namba`.
    UnboxNum {
        dst: Reg,
        src: Reg,
    },
    /// `nums[dst] = vals[src] == Ukweli(kweli)`.
    UnboxBool {
        dst: Reg,
        src: Reg,
    },
    /// Generic binary operator on `vals`.
    ValBinary {
        op: BinaryOp,
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    ValUnary {
        op: UnaryCode,
        dst: Reg,
        src: Reg,
    },
    /// `vals[dst] = vals[base][vals[idx]]`.
    ValIndex {
        dst: Reg,
        base: Reg,
        idx: Reg,
        mode: IndexMode,
    },
    /// `nums[dst] = vals[src].urefu()`.
    ValLen {
        dst: Reg,
        src: Reg,
    },
    /// `?` on `vals[src]`: `Tokeo` errors return from the function.
    Unwrap {
        dst: Reg,
        src: Reg,
    },
    /// Any other mutating method on an `Orodha<Namba>` local (e.g. `badilisha`), through the
    /// shared implementation; `vals[dst]` receives its result.
    ListMutate(Box<MutMethodOp>),
    /// `vals[dst]` = the `Orodha` snapshot `kwa ... katika vals[src]` iterates.
    IterItems {
        dst: Reg,
        src: Reg,
    },
    /// `jaribu` on `vals[src]`: errors abort execution.
    Jaribu {
        dst: Reg,
        src: Reg,
    },
    Cast {
        dst: Reg,
        src: Reg,
        ty: Box<str>,
    },
    MakeList {
        dst: Reg,
        items: Box<[Reg]>,
    },
    Call(Box<CallOp>),
    CallBuiltin(Box<BuiltinOp>),
    CallMethod(Box<MethodOp>),
    MutMethod(Box<MutMethodOp>),
    Return {
        src: Operand,
    },
    ReturnTupu,
    /// `vals[dst] = Name { field: vals[reg], … }`, fields in declaration order. The struct and
    /// field names are `Neno` constants (indices into `constants`), which the host holds as shared
    /// text: building the value only bumps reference counts, and the program itself stays plain
    /// data that threads can share.
    MakeStruct {
        dst: Reg,
        name: u32,
        fields: Box<[(u32, Reg)]>,
    },
    /// `vals[dst] = vals[src].field`; `slot` is where the field sits when the receiver's
    /// `umbo` is statically known (`u32::MAX` otherwise) — a hint, checked against the name.
    Field {
        dst: Reg,
        src: Reg,
        field: String,
        slot: u32,
    },
    /// `vals[dst] = vals[items][nums[idx]]`: the next item of a `kwa … katika` snapshot (an
    /// `Orodha` from `IterItems`; the loop keeps the index in range).
    IterItem {
        dst: Reg,
        items: Reg,
        idx: Reg,
    },
    /// `nums[dst] = vals[src].field`, a field declared `Namba`.
    FieldNum {
        dst: Reg,
        src: Reg,
        field: String,
        slot: u32,
    },
    /// `vals[dst] = Enum::Variant(vals[data])` (no data when `None`).
    MakeEnum {
        dst: Reg,
        enum_name: asili_parser::Name,
        variant: asili_parser::Name,
        data: Option<Reg>,
    },
    /// `nums[dst] = vals[src]` matches `pattern` (a `linganisha` arm); on a match the names it
    /// binds are stored in their `vals` registers. `binding` (`acha (a, b) = e`): no match is
    /// an error instead of a false flag.
    MatchPattern {
        dst: Reg,
        src: Reg,
        pattern: Box<Pattern>,
        binds: Box<[(String, Reg)]>,
        binding: bool,
    },
    /// `vals[dst] = { vals[k]: vals[v], … }`.
    MakeMap {
        dst: Reg,
        entries: Box<[(Reg, Reg)]>,
    },
    /// Where a call was inlined: fail as the call would have if it went one level past the
    /// call-depth limit (`MAX_CALL_DEPTH`), so inlining never changes where that error happens.
    CheckDepth,
    /// The statement at source line `line` starts (only in programs compiled with
    /// `CompileOptions::lines`): coverage and the debugger observe it in the host. `binds`
    /// (with `CompileOptions::bindings`): the locals visible there, innermost first, for the
    /// debugger's variables view.
    Line {
        line: u32,
        binds: Box<[(String, Operand)]>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BytecodeProgram {
    pub constants: Vec<StoredConstant>,
    pub functions: Vec<BytecodeFunc>,
    pub entry: String,
    /// The module's syntax tree (build caches read the module back from it; on wasm, until it
    /// has a backend, the tree-walker runs it).
    pub ast: Option<asili_parser::Module>,
    /// Every type a `shughuli ya` block is written for, even an empty one (for the error a
    /// method call on that type gives when no such method exists).
    pub impl_targets: Vec<String>,
    /// The function computing the module constants that are not literals, in order, as one
    /// `Orodha` (`StoredConstant::Computed(i)` is its `i`th item).
    pub init: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StoredConstant {
    Neno(String),
    Namba(f64),
    Tupu,
    Hamna,
    Ukweli(bool),
    Herufi(char),
    /// The value of the program's `n`th computed module constant (`thabiti TAU = 2.0 * PI`),
    /// filled in from `BytecodeProgram::init` before the program's first call.
    Computed(u32),
}

impl StoredConstant {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))] // only the native-code host loads constants
    pub(crate) fn to_value(&self) -> Value {
        match self {
            StoredConstant::Neno(s) => Value::neno(s.clone()),
            StoredConstant::Namba(n) => Value::Namba(*n),
            StoredConstant::Ukweli(b) => Value::Ukweli(*b),
            StoredConstant::Herufi(c) => Value::Herufi(*c),
            StoredConstant::Computed(_) => Value::Tupu,
            StoredConstant::Tupu => Value::Tupu,
            StoredConstant::Hamna => Value::Hamna,
        }
    }
}

impl BytecodeProgram {
    pub fn find_function(&self, name: &str) -> Option<&BytecodeFunc> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// `recv.method(…)` on a value of the program's own type `target` (a `umbo` or `jenum`):
    /// the index of the method's function, an inherent method before a trait's (methods are
    /// laid out that way, see `impl_methods`), or the error such a call gives.
    pub(crate) fn user_method(
        &self,
        kind: &str,
        target: &str,
        method: &str,
    ) -> Result<usize, EvalError> {
        let inherent = impl_function_name(target, None, method);
        let trait_suffix = format!(">::{method}");
        self.functions
            .iter()
            .position(|f| {
                f.name == inherent
                    || (f.name.starts_with(target)
                        && f.name[target.len()..].starts_with('<')
                        && f.name.ends_with(&trait_suffix))
            })
            .ok_or_else(|| {
                if self.impl_targets.iter().any(|t| t == target) {
                    EvalError::TypeErr(format!("njia '{method}' haijulikani kwa {kind} '{target}'"))
                } else {
                    EvalError::TypeErr(format!(
                        "{kind} '{target}' hauna shughuli yoyote iliyofafanuliwa"
                    ))
                }
            })
    }
}

// ---------------------------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------------------------

/// How a program is compiled beyond its meaning.
#[derive(Clone, Copy, Debug, Default)]
pub struct CompileOptions {
    /// Mark the start of every statement (`Opcode::Line`), for coverage and the debugger; no
    /// call is inlined, so every statement of every `kazi` is marked.
    pub lines: bool,
    /// With `lines`: each mark also names the locals visible there (the debugger).
    pub bindings: bool,
}

/// Lower the whole module to bytecode; `None` when some `kazi` cannot be lowered
/// ([`compile_module_explained`] names it).
pub fn compile_module(module: &Module) -> Option<BytecodeProgram> {
    compile_module_inner(module, CompileOptions::default(), &mut None)
}

/// Every `kazi` lowered to bytecode, or the first `kazi` and source line that could not be.
pub fn compile_module_explained(module: &Module) -> Result<BytecodeProgram, String> {
    compile_module_with(module, CompileOptions::default())
}

/// [`compile_module_explained`] with `options`.
pub fn compile_module_with(
    module: &Module,
    options: CompileOptions,
) -> Result<BytecodeProgram, String> {
    let mut failed_line = None;
    let program = compile_module_inner(module, options, &mut failed_line);
    match (program, failed_line) {
        (Some(program), None) => Ok(program),
        (_, Some((function, line))) => Err(format!("kazi '{function}', mstari {line}")),
        (None, None) => unreachable!("a failed lowering records where"),
    }
}

fn compile_module_inner(
    module: &Module,
    options: CompileOptions,
    failed_line: &mut Option<(String, usize)>,
) -> Option<BytecodeProgram> {
    let mut program = ProgramCompiler {
        options,
        failed_line: None,
        constants: Vec::new(),
        functions: HashMap::new(),
        module_consts: HashMap::new(),
        builtins: builtin_names()
            .into_iter()
            .enumerate()
            .map(|(i, name)| (name, i as u32))
            .collect(),
        structs: module
            .structs
            .iter()
            .map(|s| {
                (
                    s.name.clone(),
                    s.fields
                        .iter()
                        .map(|(f, t)| (f.clone(), t.as_ref().map(|t| t.name.replace(' ', ""))))
                        .collect(),
                )
            })
            .collect(),
        enums: module.enums.iter().map(|e| e.name.clone()).collect(),
        impl_methods: module
            .impls
            .iter()
            .flat_map(|i| i.body.iter().map(|f| f.name.to_string()))
            .collect(),
        impl_index: HashMap::new(),
    };
    for (i, function) in module.functions.iter().enumerate() {
        program.functions.insert(
            function.name.to_string(),
            FunctionSig {
                index: i as u32,
                params: function
                    .params
                    .iter()
                    .map(|p| Ty::from_type_name(&p.ty.name))
                    .collect(),
                ret: Ty::from_type_name(&function.return_type.name),
                ret_name: function.return_type.name.clone(),
                inline: inlinable(function, &module.exprs),
            },
        );
    }
    // `shughuli ya` methods compile as functions after the module's own, named
    // `Umbo::njia` (see `impl_function_name`).
    let methods = impl_methods(module);
    for (k, (imp, f)) in methods.iter().enumerate() {
        let index = (module.functions.len() + k) as u32;
        let name = impl_function_name(&imp.target, imp.trait_name.as_deref(), &f.name);
        program.functions.insert(
            name,
            FunctionSig {
                index,
                params: f
                    .params
                    .iter()
                    .map(|p| Ty::from_type_name(&p.ty.name))
                    .collect(),
                ret: Ty::from_type_name(&f.return_type.name),
                ret_name: f.return_type.name.clone(),
                inline: None,
            },
        );
        program
            .impl_index
            .entry((imp.target.clone(), f.name.to_string()))
            .or_insert(index);
    }
    let mut computed = Vec::new();
    for constant in &module.constants {
        let Some(stored) = literal(&module.exprs, &module[constant.value]) else {
            // Computed once, before the program's first call (see `init_function`).
            let stored = StoredConstant::Computed(computed.len() as u32);
            computed.push(constant);
            program.module_consts.insert(
                constant.name.clone(),
                (Ty::Val, stored, constant.ty.name.clone()),
            );
            continue;
        };
        let ty = match (&stored, Ty::from_type_name(&constant.ty.name)) {
            (StoredConstant::Namba(_), _) => Ty::Num,
            (StoredConstant::Ukweli(_), _) => Ty::Bool,
            _ => Ty::Val,
        };
        program.module_consts.insert(
            constant.name.clone(),
            (ty, stored, constant.ty.name.clone()),
        );
    }
    let mut functions = Vec::with_capacity(module.functions.len() + methods.len());
    let all = module
        .functions
        .iter()
        .map(|f| (f.name.to_string(), f))
        .chain(methods.iter().map(|(imp, f)| {
            (
                impl_function_name(&imp.target, imp.trait_name.as_deref(), &f.name),
                *f,
            )
        }));
    for (qualified, function) in all {
        match FunctionCompiler::compile(&mut program, &module.exprs, function).map(|mut f| {
            f.name = qualified.clone();
            f
        }) {
            Some(mut f) => {
                #[cfg(not(target_arch = "wasm32"))]
                crate::scalars::split_structs(&mut f, &program.constants);
                functions.push(f)
            }
            None => {
                *failed_line = Some((
                    function.name.to_string(),
                    program.failed_line.unwrap_or(function.line),
                ));
                return None;
            }
        }
    }
    let init = if computed.is_empty() {
        None
    } else {
        match FunctionCompiler::init_function(&mut program, &module.exprs, &computed) {
            Some(f) => {
                functions.push(f);
                Some((functions.len() - 1) as u32)
            }
            None => {
                *failed_line = Some(("thabiti".to_string(), program.failed_line.unwrap_or(0)));
                return None;
            }
        }
    };
    if std::env::var_os("ASILI_BYTECODE_DUMP").is_some() {
        // Debugging aid: the instructions of every compiled `kazi`.
        for f in &functions {
            eprintln!("kazi {}:", f.name);
            for (pc, op) in f.code.iter().enumerate() {
                eprintln!("  {pc:4} {op:?}");
            }
        }
    }
    Some(BytecodeProgram {
        constants: program.constants,
        functions,
        entry: "kuu".to_string(),
        ast: Some(module.clone()),
        impl_targets: module.impls.iter().map(|i| i.target.clone()).collect(),
        init,
    })
}

fn literal(exprs: &Exprs, expr: &Expr) -> Option<StoredConstant> {
    Some(match expr {
        Expr::Number(s) => StoredConstant::Namba(value::parse_number(s)),
        Expr::String(s) => StoredConstant::Neno(s.clone()),
        Expr::Bool(b) => StoredConstant::Ukweli(*b),
        Expr::Char(c) => StoredConstant::Herufi(*c),
        Expr::Hamna => StoredConstant::Hamna,
        Expr::Group(e) => return literal(exprs, &exprs[*e]),
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
            ..
        } => match literal(exprs, &exprs[*expr])? {
            StoredConstant::Namba(n) => StoredConstant::Namba(-n),
            _ => return None,
        },
        _ => return None,
    })
}

struct FunctionSig {
    index: u32,
    params: Vec<Ty>,
    ret: Ty,
    ret_name: String,
    /// Set when calls compile to the body itself (see [`inlinable`]).
    inline: Option<Inline>,
}

/// A `kazi` whose body is one `rejesha` of a small expression that calls nothing, mutates
/// nothing and cannot return early: a call to it compiles to that expression with the
/// parameters bound to the arguments' registers — no frame, no call.
#[derive(Clone)]
struct Inline {
    /// Parameter names and declared type names, in order.
    params: Vec<(String, String)>,
    body: ExprId,
}

/// Most expression nodes an inlined body may have.
const INLINE_MAX_NODES: usize = 40;

/// The [`Inline`] form of `f`, when it has one.
fn inlinable(f: &Function, exprs: &Exprs) -> Option<Inline> {
    let [Stmt::Return {
        value: Some(body), ..
    }] = f.body.statements.as_slice()
    else {
        return None;
    };
    if f.is_test {
        return None;
    }
    let nodes = exprs.descendants(*body);
    let pure = nodes.len() <= INLINE_MAX_NODES
        && nodes.iter().all(|id| match &exprs[*id] {
            Expr::Number(_)
            | Expr::String(_)
            | Expr::Bool(_)
            | Expr::Char(_)
            | Expr::Hamna
            | Expr::Ident { .. }
            | Expr::Group(_)
            | Expr::Binary { .. }
            | Expr::Cast { .. }
            | Expr::FieldAccess { .. }
            | Expr::StructLiteral { .. }
            | Expr::EnumConstruct { .. }
            | Expr::If { .. }
            | Expr::List { .. } => true,
            Expr::Unary { op, .. } => matches!(op, UnaryOp::Neg | UnaryOp::Not | UnaryOp::BitNot),
            // Calls (depth, callbacks), methods (mutation), `?` (early return), indexing (its
            // error names the callee's line): compiled as calls.
            Expr::Call { .. }
            | Expr::MethodCall { .. }
            | Expr::Propagate { .. }
            | Expr::Index { .. }
            | Expr::Map { .. } => false,
        });
    pure.then(|| Inline {
        params: f
            .params
            .iter()
            .map(|p| (p.name.to_string(), p.ty.name.replace(' ', "")))
            .collect(),
        body: *body,
    })
}

struct ProgramCompiler {
    options: CompileOptions,
    /// Innermost statement line that could not be lowered.
    failed_line: Option<usize>,
    constants: Vec<StoredConstant>,
    functions: HashMap<String, FunctionSig>,
    /// name -> (type, value, declared type name)
    module_consts: HashMap<String, (Ty, StoredConstant, String)>,
    builtins: HashMap<String, u32>,
    /// `umbo` name -> field names in declaration order.
    structs: HashMap<String, Vec<(String, Option<String>)>>,
    /// Declared `jenum` names.
    enums: std::collections::HashSet<String>,
    /// Method names some `impl` block defines (a call may reach a user method even where a
    /// builtin method of the same name exists).
    impl_methods: std::collections::HashSet<String>,
    /// (`umbo`, method) -> the compiled method's function index; inherent `shughuli ya` blocks
    /// first, then trait impls, as the tree-walker searches them.
    impl_index: HashMap<(String, String), u32>,
}

impl ProgramCompiler {
    fn constant(&mut self, constant: StoredConstant) -> u32 {
        if let Some(i) = self.constants.iter().position(|c| *c == constant) {
            return i as u32;
        }
        self.constants.push(constant);
        (self.constants.len() - 1) as u32
    }
}

#[derive(Clone)]
struct Local {
    op: Operand,
    /// Static type name for generic locals (e.g. `Neno`, `Orodha<Neno>`), used to decide
    /// whether a method call on it is supported.
    type_name: Option<String>,
}

struct LoopState {
    label: Option<String>,
    /// `scopes.len()` where the loop starts: bindings of scopes below it outlive its iterations.
    depth: usize,
    breaks: Vec<usize>,
    continues: Vec<usize>,
}

struct FunctionCompiler<'a> {
    program: &'a mut ProgramCompiler,
    /// The module's expression arena.
    exprs: &'a Exprs,
    scopes: Vec<HashMap<String, Local>>,
    code: Vec<Opcode>,
    num_regs: u32,
    list_regs: u32,
    val_regs: u32,
    num_consts: Vec<(Reg, f64)>,
    const_regs: HashMap<u64, Reg>,
    loops: Vec<LoopState>,
    ret: Ty,
}

fn is_cmp(op: &BinaryOp) -> Option<CmpOp> {
    Some(match op {
        BinaryOp::Lt => CmpOp::Lt,
        BinaryOp::Le => CmpOp::Le,
        BinaryOp::Gt => CmpOp::Gt,
        BinaryOp::Ge => CmpOp::Ge,
        BinaryOp::Eq => CmpOp::Eq,
        BinaryOp::Ne => CmpOp::Ne,
        _ => return None,
    })
}

/// The AST operator a generic `Binary` instruction carries; `na`/`au` short-circuit and are
/// lowered to jumps instead.
fn binary_code(op: &BinaryOp) -> Option<BinaryOp> {
    match op {
        BinaryOp::And | BinaryOp::Or => None,
        other => Some(other.clone()),
    }
}

/// Static result type name of a supported method, for chaining (`b.vipande(9).ramani(..)`).
fn method_result_type(receiver: &str, method: &str) -> Option<&'static str> {
    if receiver.starts_with("Orodha") {
        match method {
            "clona" | "vipande" | "kwa_neno" | "ramani" | "chuja" => Some("Orodha"),
            "jiunge" | "unganisha" => Some("Neno"),
            _ => None,
        }
    } else if receiver.starts_with("Kamusi") {
        (method == "funguo").then_some("Orodha")
    } else if receiver.starts_with("Seti") {
        (method == "orodha").then_some("Orodha")
    } else if receiver == "Neno" {
        match method {
            "clona" | "kata" | "kwa_herufi_ndogo" | "kwa_herufi_kubwa" | "unganisha" | "rudia"
            | "badilisha" => Some("Neno"),
            "gawanya" => Some("Orodha<Neno>"),
            _ => None,
        }
    } else {
        None
    }
}

/// A value of the statically known receiver type, to ask `eval::methods` which methods exist.
fn probe_value(type_name: &str) -> Option<Value> {
    let base = type_name.split('<').next().unwrap_or(type_name).trim();
    Some(match base {
        "Neno" => Value::neno(String::new()),
        "Orodha" => Value::list(Vec::new()),
        "Kamusi" => Value::Kamusi(Default::default()),
        "Seti" => Value::Seti(Default::default()),
        "Chaguo" => Value::Chaguo(None),
        "Tokeo" => Value::sawa(Value::Tupu),
        "Jozi" => Value::Jozi(Box::new(Value::Tupu), Box::new(Value::Tupu)),
        "Wakati" => Value::Wakati(0.0),
        _ => return None,
    })
}

/// Whether `stmts` (recursively) assign to `name`.
fn block_assigns(block: &Block, name: &str) -> bool {
    block.statements.iter().any(|stmt| match stmt {
        Stmt::Assign { name: n, .. } => n == name,
        Stmt::If {
            then_block,
            else_if,
            else_block,
            ..
        } => {
            block_assigns(then_block, name)
                || else_if.iter().any(|(_, b)| block_assigns(b, name))
                || else_block.as_ref().is_some_and(|b| block_assigns(b, name))
        }
        Stmt::While { body, .. } | Stmt::For { body, .. } => block_assigns(body, name),
        Stmt::Match { arms, .. } => arms.iter().any(|a| block_assigns(&a.body, name)),
        _ => false,
    })
}

impl<'a> FunctionCompiler<'a> {
    fn compile(
        program: &'a mut ProgramCompiler,
        exprs: &'a Exprs,
        function: &Function,
    ) -> Option<BytecodeFunc> {
        let ret = Ty::from_type_name(&function.return_type.name);
        let mut f = FunctionCompiler {
            program,
            exprs,
            scopes: vec![HashMap::new()],
            code: Vec::new(),
            num_regs: 0,
            list_regs: 0,
            val_regs: 0,
            num_consts: Vec::new(),
            const_regs: HashMap::new(),
            loops: Vec::new(),
            ret,
        };
        let mut params = Vec::with_capacity(function.params.len());
        for param in &function.params {
            let ty = Ty::from_type_name(&param.ty.name);
            let op = f.declare(&param.name, ty, Some(param.ty.name.clone()));
            params.push(op);
        }
        f.block(&function.body)?;
        f.emit(Opcode::ReturnTupu);
        Some(BytecodeFunc {
            name: function.name.to_string(),
            line: function.line as u32,
            params,
            ret,
            num_regs: f.num_regs,
            list_regs: f.list_regs,
            val_regs: f.val_regs,
            num_consts: f.num_consts,
            code: f.code,
        })
    }

    /// The module constants that are not literals, computed in order (each sees the ones
    /// before it) and returned as one `Orodha`.
    fn init_function(
        program: &'a mut ProgramCompiler,
        exprs: &'a Exprs,
        constants: &[&asili_parser::Constant],
    ) -> Option<BytecodeFunc> {
        let mut f = FunctionCompiler {
            program,
            exprs,
            scopes: vec![HashMap::new()],
            code: Vec::new(),
            num_regs: 0,
            list_regs: 0,
            val_regs: 0,
            num_consts: Vec::new(),
            const_regs: HashMap::new(),
            loops: Vec::new(),
            ret: Ty::Val,
        };
        let mut items = Vec::with_capacity(constants.len());
        for constant in constants {
            let dst = f.declare(&constant.name, Ty::Val, Some(constant.ty.name.clone()));
            if f.expr_into(f.node(constant.value), dst).is_none() {
                f.program.failed_line.get_or_insert(constant.line);
                return None;
            }
            items.push(dst.reg);
        }
        let out = f.temp(Ty::Val);
        f.emit(Opcode::MakeList {
            dst: out.reg,
            items: items.into_boxed_slice(),
        });
        f.emit(Opcode::Return { src: out });
        Some(BytecodeFunc {
            name: "<thabiti>".to_string(),
            line: constants.first().map_or(0, |c| c.line as u32),
            params: Vec::new(),
            ret: Ty::Val,
            num_regs: f.num_regs,
            list_regs: f.list_regs,
            val_regs: f.val_regs,
            num_consts: f.num_consts,
            code: f.code,
        })
    }

    /// The expression `id` (borrowed from the module, not from `self`).
    fn node(&self, id: ExprId) -> &'a Expr {
        &self.exprs[id]
    }

    // -- registers and scopes -----------------------------------------------------------------

    fn temp(&mut self, ty: Ty) -> Operand {
        let reg = match ty {
            Ty::Num | Ty::Bool => {
                self.num_regs += 1;
                self.num_regs - 1
            }
            Ty::List => {
                self.list_regs += 1;
                self.list_regs - 1
            }
            Ty::Val => {
                self.val_regs += 1;
                self.val_regs - 1
            }
        };
        Operand { ty, reg }
    }

    fn declare(&mut self, name: &str, ty: Ty, type_name: Option<String>) -> Operand {
        // Written type names may carry spaces (`Orodha < Neno >`); keep one spelling.
        let type_name = type_name.map(|t| {
            if t.contains(' ') {
                t.replace(' ', "")
            } else {
                t
            }
        });
        let op = self.temp(ty);
        self.scopes
            .last_mut()
            .expect("scope")
            .insert(name.to_string(), Local { op, type_name });
        op
    }

    fn lookup(&self, name: &str) -> Option<&Local> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    fn num_const(&mut self, n: f64) -> Reg {
        if let Some(reg) = self.const_regs.get(&n.to_bits()) {
            return *reg;
        }
        let reg = self.temp(Ty::Num).reg;
        self.num_consts.push((reg, n));
        self.const_regs.insert(n.to_bits(), reg);
        reg
    }

    fn emit(&mut self, op: Opcode) -> usize {
        self.code.push(op);
        self.code.len() - 1
    }

    fn here(&self) -> u32 {
        self.code.len() as u32
    }

    fn patch(&mut self, at: usize, to: u32) {
        match &mut self.code[at] {
            Opcode::Jump { target }
            | Opcode::JumpIfFalse { target, .. }
            | Opcode::JumpIfTrue { target, .. }
            | Opcode::JumpIfNot { target, .. }
            | Opcode::ForStep { target, .. } => *target = to,
            _ => {}
        }
    }

    // -- static types -------------------------------------------------------------------------

    fn infer(&self, expr: &Expr) -> Ty {
        match expr {
            Expr::Number(_) => Ty::Num,
            Expr::Bool(_) => Ty::Bool,
            Expr::Ident { name, .. } => match self.lookup(name) {
                Some(local) => local.op.ty,
                None => match self.program.module_consts.get(name.as_str()) {
                    Some((ty, _, _)) => *ty,
                    None => match crate::env::global_constants()
                        .into_iter()
                        .find(|(n, _)| n == name)
                    {
                        Some((_, Value::Namba(_))) => Ty::Num,
                        Some((_, Value::Ukweli(_))) => Ty::Bool,
                        _ => Ty::Val,
                    },
                },
            },
            Expr::Group(e) => self.infer(self.node(*e)),
            Expr::Unary { op, expr, .. } => match op {
                UnaryOp::Neg | UnaryOp::BitNot if self.infer(self.node(*expr)) == Ty::Num => {
                    Ty::Num
                }
                UnaryOp::Not => Ty::Bool,
                UnaryOp::BorrowImm | UnaryOp::BorrowMut => self.infer(self.node(*expr)),
                _ => Ty::Val,
            },
            Expr::Binary {
                left, op, right, ..
            } => {
                if is_cmp(op).is_some() || matches!(op, BinaryOp::And | BinaryOp::Or) {
                    Ty::Bool
                } else if self.infer(self.node(*left)) == Ty::Num
                    && self.infer(self.node(*right)) == Ty::Num
                {
                    Ty::Num
                } else {
                    Ty::Val
                }
            }
            Expr::Cast { expr, ty, .. } => {
                if ty.name.replace(' ', "") == "Namba" && self.infer(self.node(*expr)).in_nums() {
                    Ty::Num
                } else {
                    Ty::Val
                }
            }
            Expr::Call { callee, args, .. } => match self.node(*callee) {
                Expr::Ident { name, .. } => {
                    if matches!(name.as_str(), "sakafu" | "dari")
                        && args.len() == 1
                        && self.infer(self.node(args[0])) == Ty::Num
                        && self.lookup(name).is_none()
                        && !self.program.functions.contains_key(name.as_str())
                    {
                        Ty::Num
                    } else {
                        // The program's own `kazi` first: it shadows a builtin.
                        self.program
                            .functions
                            .get(name.as_str())
                            .map(|f| f.ret)
                            .unwrap_or(Ty::Val)
                    }
                }
                _ => Ty::Val,
            },
            Expr::MethodCall {
                method_name, args, ..
            } if method_name == "urefu" && args.is_empty() => Ty::Num,
            Expr::Index { base, .. } if self.infer(self.node(*base)) == Ty::List => Ty::Num,
            Expr::FieldAccess {
                receiver, field, ..
            } if self.field_type(self.node(*receiver), field).as_deref() == Some("Namba") => {
                Ty::Num
            }
            Expr::Propagate { expr, .. } => match self.node(*expr) {
                Expr::Index { base, .. } if self.infer(self.node(*base)) == Ty::List => Ty::Num,
                _ => Ty::Val,
            },
            Expr::If {
                then_expr,
                else_if,
                else_expr,
                ..
            } => {
                let mut ty = self.infer(self.node(*then_expr));
                for (_, e) in else_if {
                    if self.infer(self.node(*e)) != ty {
                        ty = Ty::Val;
                    }
                }
                match else_expr {
                    Some(e) if self.infer(self.node(*e)) == ty => ty,
                    _ => Ty::Val,
                }
            }
            _ => Ty::Val,
        }
    }

    /// Position and declared type of `receiver.field`, when the receiver's `umbo` is
    /// statically known.
    fn field_decl(&self, receiver: &Expr, field: &str) -> Option<(u32, Option<String>)> {
        let umbo = self.type_name(receiver)?;
        let fields = self.program.structs.get(&umbo)?;
        let slot = fields.iter().position(|(f, _)| f == field)?;
        Some((slot as u32, fields[slot].1.clone()))
    }

    fn field_type(&self, receiver: &Expr, field: &str) -> Option<String> {
        self.field_decl(receiver, field)?.1
    }

    /// Static type name of a generic expression, where known.
    fn type_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::String(_) => Some("Neno".into()),
            // Items of one known type: `["a", "b"]` is an `Orodha<Neno>`.
            Expr::List {
                elements: items, ..
            } => {
                let first = items.first().and_then(|e| self.type_name(self.node(*e)));
                Some(match first {
                    Some(t)
                        if items[1..]
                            .iter()
                            .all(|e| self.type_name(self.node(*e)).as_ref() == Some(&t)) =>
                    {
                        format!("Orodha<{t}>")
                    }
                    _ => "Orodha".into(),
                })
            }
            // An element of an `Orodha<T>` (not `?`, which keeps the `Tokeo`).
            Expr::Index { base, .. } => self
                .type_name(self.node(*base))?
                .strip_prefix("Orodha<")?
                .strip_suffix('>')
                .map(str::to_string),
            Expr::StructLiteral { struct_name, .. } => Some(struct_name.clone()),
            Expr::Group(e) => self.type_name(self.node(*e)),
            Expr::Ident { name, .. } => match self.lookup(name) {
                Some(local) if local.op.ty == Ty::List => Some("Orodha<Namba>".into()),
                Some(local) => local.type_name.clone(),
                None => self
                    .program
                    .module_consts
                    .get(name.as_str())
                    .map(|(_, _, t)| t.clone()),
            },
            Expr::Cast { ty, .. } if ty.name == "Neno" => Some("Neno".into()),
            Expr::FieldAccess {
                receiver, field, ..
            } => self.field_type(self.node(*receiver), field),
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } if self.type_name(self.node(*left)).as_deref() == Some("Neno")
                || self.type_name(self.node(*right)).as_deref() == Some("Neno") =>
            {
                Some("Neno".into())
            }
            Expr::MethodCall {
                receiver,
                method_name,
                ..
            } => {
                let recv = self.type_name(self.node(*receiver))?;
                method_result_type(&recv, method_name).map(str::to_string)
            }
            // The program's own `kazi` (which shadows a builtin of the same name).
            Expr::Call { callee, .. } => match self.node(*callee) {
                Expr::Ident { name, .. } => self
                    .program
                    .functions
                    .get(name.as_str())
                    .map(|f| f.ret_name.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Every local visible here, innermost first, each name once (a shadowed one is hidden).
    fn visible_locals(&self) -> Box<[(String, Operand)]> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for scope in self.scopes.iter().rev() {
            let mut names: Vec<_> = scope.iter().collect();
            names.sort_by_key(|(_, l)| std::cmp::Reverse((l.op.ty as u8, l.op.reg)));
            for (name, local) in names {
                if seen.insert(name.as_str()) {
                    out.push((name.clone(), local.op));
                }
            }
        }
        out.into_boxed_slice()
    }

    // -- statements ---------------------------------------------------------------------------

    fn block(&mut self, block: &Block) -> Option<()> {
        self.scopes.push(HashMap::new());
        for stmt in &block.statements {
            if self.program.options.lines {
                let binds = if self.program.options.bindings {
                    self.visible_locals()
                } else {
                    Box::new([])
                };
                self.emit(Opcode::Line {
                    line: stmt.line() as u32,
                    binds,
                });
            }
            if self.stmt(stmt).is_none() {
                self.program.failed_line.get_or_insert(stmt.line());
                return None;
            }
        }
        self.scopes.pop();
        Some(())
    }

    fn stmt(&mut self, stmt: &Stmt) -> Option<()> {
        match stmt {
            Stmt::Let {
                name, ty, value, ..
            } => {
                let (declared, type_name) = match ty {
                    Some(t) => (Ty::from_type_name(&t.name), Some(t.name.clone())),
                    None => {
                        let inferred = self.infer(self.node(*value));
                        let inferred = if inferred == Ty::Val
                            && self.numeric_list_literal(self.node(*value))
                        {
                            Ty::List
                        } else {
                            inferred
                        };
                        (inferred, self.type_name(self.node(*value)))
                    }
                };
                // The initializer is evaluated before the new binding is visible.
                let dst = self.temp(declared);
                self.expr_into(self.node(*value), dst)?;
                self.scopes
                    .last_mut()?
                    .insert(name.to_string(), Local { op: dst, type_name });
            }
            Stmt::Assign {
                name, op, value, ..
            } => {
                let dst = self.lookup(name)?.op;
                match op {
                    AssignOp::Assign => self.expr_into(self.node(*value), dst)?,
                    compound => {
                        let bin = match compound {
                            AssignOp::AddAssign => BinaryOp::Add,
                            AssignOp::SubAssign => BinaryOp::Sub,
                            AssignOp::MulAssign => BinaryOp::Mul,
                            AssignOp::DivAssign => BinaryOp::Div,
                            AssignOp::Assign => unreachable!(),
                        };
                        let current = Expr::Ident {
                            name: *name,
                            line: 0,
                            column: 0,
                        };
                        // The result may land in a temporary of another type (e.g. a generic
                        // `+`); make sure it reaches the variable.
                        let result = self.binary(&current, &bin, self.node(*value), Some(dst))?;
                        if result != dst {
                            self.convert(result, dst)?;
                        }
                    }
                }
            }
            Stmt::Expr { expr, .. } => self.expr_stmt(self.node(*expr))?,
            Stmt::Return { value, .. } => match value {
                Some(value) => {
                    let src = self.expr_as(self.node(*value), self.ret)?;
                    self.emit(Opcode::Return { src });
                }
                None => {
                    self.emit(Opcode::ReturnTupu);
                }
            },
            Stmt::If {
                cond,
                then_block,
                else_if,
                else_block,
                ..
            } => {
                let mut ends = Vec::new();
                let skip = self.cond_false_jumps(self.node(*cond))?;
                self.block(then_block)?;
                ends.push(self.emit(Opcode::Jump { target: 0 }));
                let mut pending = skip;
                for (cond, block) in else_if {
                    let here = self.here();
                    for j in pending {
                        self.patch(j, here);
                    }
                    pending = self.cond_false_jumps(self.node(*cond))?;
                    self.block(block)?;
                    ends.push(self.emit(Opcode::Jump { target: 0 }));
                }
                let here = self.here();
                for j in pending {
                    self.patch(j, here);
                }
                if let Some(block) = else_block {
                    self.block(block)?;
                }
                let end = self.here();
                for j in ends {
                    self.patch(j, end);
                }
            }
            Stmt::While {
                label, cond, body, ..
            } => {
                let top = self.here();
                let exits = self.cond_false_jumps(self.node(*cond))?;
                self.loops.push(LoopState {
                    label: label.clone(),
                    depth: self.scopes.len(),
                    breaks: Vec::new(),
                    continues: Vec::new(),
                });
                self.block(body)?;
                self.emit(Opcode::Jump { target: top });
                let end = self.here();
                for j in exits {
                    self.patch(j, end);
                }
                let state = self.loops.pop()?;
                for j in state.breaks {
                    self.patch(j, end);
                }
                for j in state.continues {
                    self.patch(j, top);
                }
            }
            Stmt::For {
                label,
                var,
                mode,
                body,
                ..
            } => self.for_loop(label.clone(), var, mode, body)?,
            Stmt::Break { label, .. } => {
                let jump = self.emit(Opcode::Jump { target: 0 });
                self.find_loop(label)?.breaks.push(jump);
            }
            Stmt::Continue { label, .. } => {
                let jump = self.emit(Opcode::Jump { target: 0 });
                self.find_loop(label)?.continues.push(jump);
            }
            Stmt::Match { expr, arms, .. } => {
                // The scrutinee is evaluated once; arms are tried in order and the first match
                // runs with the pattern's names bound (no arm matching does nothing).
                let src = self.expr_as(self.node(*expr), Ty::Val)?;
                let mut ends = Vec::new();
                for arm in arms {
                    self.scopes.push(HashMap::new());
                    let mut names = Vec::new();
                    pattern_names(&arm.pattern, &mut names);
                    let binds: Vec<(String, Reg)> = names
                        .into_iter()
                        .map(|n| {
                            let reg = self.declare(&n, Ty::Val, None).reg;
                            (n, reg)
                        })
                        .collect();
                    let flag = self.temp(Ty::Bool);
                    self.emit(Opcode::MatchPattern {
                        dst: flag.reg,
                        src: src.reg,
                        pattern: Box::new(arm.pattern.clone()),
                        binds: binds.into_boxed_slice(),
                        binding: false,
                    });
                    let skip = self.emit(Opcode::JumpIfFalse {
                        cond: flag.reg,
                        target: 0,
                    });
                    self.block(&arm.body)?;
                    ends.push(self.emit(Opcode::Jump { target: 0 }));
                    self.scopes.pop();
                    let next = self.here();
                    self.patch(skip, next);
                }
                let end = self.here();
                for j in ends {
                    self.patch(j, end);
                }
            }
            Stmt::Drop { name, .. } => {
                // The innermost binding of `name`. Not one declared outside an enclosing loop:
                // there a second iteration's drop is the tree-walker's "already dropped" error,
                // which a static drop cannot give.
                let scope = self
                    .scopes
                    .iter()
                    .rposition(|s| s.contains_key(name.as_str()))?;
                if self.loops.iter().any(|l| l.depth > scope) {
                    return None;
                }
                let local = self.scopes[scope].remove(name.as_str())?;
                let reg = local.op.reg;
                match local.op.ty {
                    Ty::Num | Ty::Bool => {}
                    Ty::List => {
                        self.emit(Opcode::MakeNumList {
                            dst: reg,
                            items: Vec::new(),
                        });
                    }
                    Ty::Val => {
                        let k = self.program.constant(StoredConstant::Tupu);
                        self.emit(Opcode::ConstVal { dst: reg, k });
                    }
                }
            }
            Stmt::LetPattern { pattern, value, .. } => {
                // The value is evaluated before the pattern's names are visible; a value the
                // pattern does not match is an error.
                let src = self.expr_as(self.node(*value), Ty::Val)?;
                let mut names = Vec::new();
                pattern_names(pattern, &mut names);
                let binds: Vec<(String, Reg)> = names
                    .into_iter()
                    .map(|n| {
                        let reg = self.declare(&n, Ty::Val, None).reg;
                        (n, reg)
                    })
                    .collect();
                let flag = self.temp(Ty::Bool);
                self.emit(Opcode::MatchPattern {
                    dst: flag.reg,
                    src: src.reg,
                    pattern: Box::new(pattern.clone()),
                    binds: binds.into_boxed_slice(),
                    binding: true,
                });
            }
        }
        Some(())
    }

    fn find_loop(&mut self, label: &Option<String>) -> Option<&mut LoopState> {
        match label {
            None => self.loops.last_mut(),
            Some(label) => self
                .loops
                .iter_mut()
                .rev()
                .find(|l| l.label.as_deref() == Some(label.as_str())),
        }
    }

    fn numeric_list_literal(&self, expr: &Expr) -> bool {
        match expr {
            Expr::List { elements, .. } => {
                !elements.is_empty()
                    && elements
                        .iter()
                        .all(|e| self.infer(self.node(*e)) == Ty::Num)
            }
            _ => false,
        }
    }

    fn for_loop(
        &mut self,
        label: Option<String>,
        var: &str,
        mode: &ForMode,
        body: &Block,
    ) -> Option<()> {
        self.scopes.push(HashMap::new());
        match mode {
            ForMode::Range { start, end } => {
                let start = self.expr_as(self.node(*start), Ty::Num)?;
                let end = self.expr_as(self.node(*end), Ty::Num)?;
                let ctr = self.temp(Ty::Num).reg;
                let end_reg = self.temp(Ty::Num).reg;
                self.emit(Opcode::Trunc {
                    dst: ctr,
                    src: start.reg,
                });
                self.emit(Opcode::Trunc {
                    dst: end_reg,
                    src: end.reg,
                });
                let skip = self.emit(Opcode::JumpIfNot {
                    op: CmpOp::Lt,
                    a: ctr,
                    b: end_reg,
                    target: 0,
                });
                let body_top = self.here();
                // The loop variable is a fresh binding per iteration; reuse the counter
                // register directly unless the body assigns to it.
                if block_assigns(body, var) {
                    let v = self.declare(var, Ty::Num, Some("Namba".into()));
                    self.emit(Opcode::Mov {
                        dst: v.reg,
                        src: ctr,
                    });
                } else {
                    self.scopes.last_mut()?.insert(
                        var.to_string(),
                        Local {
                            op: Operand {
                                ty: Ty::Num,
                                reg: ctr,
                            },
                            type_name: Some("Namba".into()),
                        },
                    );
                }
                self.loops.push(LoopState {
                    label,
                    depth: self.scopes.len(),
                    breaks: Vec::new(),
                    continues: Vec::new(),
                });
                self.block(body)?;
                let step = self.here();
                self.emit(Opcode::ForStep {
                    ctr,
                    end: end_reg,
                    target: body_top,
                });
                let end_pos = self.here();
                self.patch(skip, end_pos);
                let state = self.loops.pop()?;
                for j in state.breaks {
                    self.patch(j, end_pos);
                }
                for j in state.continues {
                    self.patch(j, step);
                }
            }
            ForMode::InExpr(collection) => {
                // Iterate over a snapshot the body cannot reach, like the evaluator.
                let idx = self.temp(Ty::Num).reg;
                let len = self.temp(Ty::Num).reg;
                let (source, item) = if self.infer(self.node(*collection)) == Ty::List {
                    let snapshot = self.temp(Ty::List);
                    self.expr_into(self.node(*collection), snapshot)?;
                    self.emit(Opcode::ListLen {
                        dst: len,
                        list: snapshot.reg,
                    });
                    let item = self.declare(var, Ty::Num, Some("Namba".into()));
                    (snapshot, item)
                } else {
                    // `Orodha` items, or `Kamusi` entries as `Jozi` pairs.
                    let type_name = self.type_name(self.node(*collection));
                    let elem = match type_name.as_deref() {
                        Some(t) if t.starts_with("Kamusi") => Some("Jozi".to_string()),
                        Some(t) => t
                            .strip_prefix("Orodha<")
                            .and_then(|rest| rest.strip_suffix('>'))
                            .map(str::to_string),
                        None => None,
                    };
                    let value = self.expr_as(self.node(*collection), Ty::Val)?;
                    let snapshot = self.temp(Ty::Val);
                    self.emit(Opcode::IterItems {
                        dst: snapshot.reg,
                        src: value.reg,
                    });
                    self.emit(Opcode::ValLen {
                        dst: len,
                        src: snapshot.reg,
                    });
                    let item_ty = match elem.as_deref().map(Ty::from_type_name) {
                        Some(Ty::List) | None => Ty::Val,
                        Some(ty) => ty,
                    };
                    let item = self.declare(var, item_ty, elem);
                    (snapshot, item)
                };
                let zero = self.num_const(0.0);
                self.emit(Opcode::Mov {
                    dst: idx,
                    src: zero,
                });
                let skip = self.emit(Opcode::JumpIfNot {
                    op: CmpOp::Lt,
                    a: idx,
                    b: len,
                    target: 0,
                });
                let body_top = self.here();
                // The index is always in range here.
                if source.ty == Ty::List {
                    self.emit(Opcode::ListGet {
                        dst: item.reg,
                        list: source.reg,
                        idx,
                        mode: IndexMode::Element,
                    });
                } else {
                    let next = self.dst_or_temp(Some(item), Ty::Val);
                    self.emit(Opcode::IterItem {
                        dst: next.reg,
                        items: source.reg,
                        idx,
                    });
                    self.convert(next, item)?;
                }
                self.loops.push(LoopState {
                    label,
                    depth: self.scopes.len(),
                    breaks: Vec::new(),
                    continues: Vec::new(),
                });
                self.block(body)?;
                let step = self.here();
                self.emit(Opcode::ForStep {
                    ctr: idx,
                    end: len,
                    target: body_top,
                });
                let end_pos = self.here();
                self.patch(skip, end_pos);
                let state = self.loops.pop()?;
                for j in state.breaks {
                    self.patch(j, end_pos);
                }
                for j in state.continues {
                    self.patch(j, step);
                }
            }
        }
        self.scopes.pop();
        Some(())
    }

    /// An expression evaluated for its side effects only.
    fn expr_stmt(&mut self, expr: &Expr) -> Option<()> {
        if let Expr::MethodCall {
            receiver,
            method_name,
            args,
            ..
        } = expr
        {
            // (A user method name skips the numeric-list fast paths: `method_call` handles it.)
            if let (false, Expr::Ident { name, .. }) = (
                self.program.impl_methods.contains(method_name),
                self.node(*receiver),
            ) {
                if let Some(local) = self.lookup(name).cloned() {
                    // Statement-level numeric-list mutations: no `Tupu`/`Chaguo` result.
                    let list = local.op.reg;
                    match (local.op.ty, method_name.as_str(), args.len()) {
                        (Ty::List, "ondoa", 1) => {
                            let idx = self.expr_as(self.node(args[0]), Ty::Num)?.reg;
                            self.emit(Opcode::ListRemove { list, idx });
                            return Some(());
                        }
                        (Ty::List, "ongeza", 1) => {
                            let src = self.expr_as(self.node(args[0]), Ty::Num)?.reg;
                            self.emit(Opcode::ListPush { list, src });
                            return Some(());
                        }
                        (Ty::List, "ingiza", 2) => {
                            let idx = self.expr_as(self.node(args[0]), Ty::Num)?.reg;
                            let src = self.expr_as(self.node(args[1]), Ty::Num)?.reg;
                            self.emit(Opcode::ListSet { list, idx, src });
                            return Some(());
                        }
                        _ => {}
                    }
                }
            }
        }
        self.expr(expr)?;
        Some(())
    }

    /// Emit jumps taken when `cond` is false; returns the jump sites to patch.
    fn cond_false_jumps(&mut self, cond: &Expr) -> Option<Vec<usize>> {
        match cond {
            Expr::Group(inner) => self.cond_false_jumps(self.node(*inner)),
            Expr::Binary {
                left,
                op: BinaryOp::And,
                right,
                ..
            } => {
                let mut jumps = self.cond_false_jumps(self.node(*left))?;
                jumps.extend(self.cond_false_jumps(self.node(*right))?);
                Some(jumps)
            }
            Expr::Binary {
                left, op, right, ..
            } if is_cmp(op).is_some()
                && self.infer(self.node(*left)) == Ty::Num
                && self.infer(self.node(*right)) == Ty::Num =>
            {
                let a = self.expr_as(self.node(*left), Ty::Num)?;
                let b = self.expr_as(self.node(*right), Ty::Num)?;
                Some(vec![self.emit(Opcode::JumpIfNot {
                    op: is_cmp(op)?,
                    a: a.reg,
                    b: b.reg,
                    target: 0,
                })])
            }
            _ => {
                let c = self.expr_as(cond, Ty::Bool)?;
                Some(vec![self.emit(Opcode::JumpIfFalse {
                    cond: c.reg,
                    target: 0,
                })])
            }
        }
    }

    // -- expressions --------------------------------------------------------------------------

    /// Compile `expr` into a register of type `ty`.
    fn expr_as(&mut self, expr: &Expr, ty: Ty) -> Option<Operand> {
        let op = self.expr(expr)?;
        if op.ty == ty || (op.ty.in_nums() && ty.in_nums()) {
            return Some(Operand { ty, reg: op.reg });
        }
        let dst = self.temp(ty);
        self.convert(op, dst)?;
        Some(dst)
    }

    /// Compile `expr` so that its value ends up in `dst`.
    fn expr_into(&mut self, expr: &Expr, dst: Operand) -> Option<()> {
        let result = self.expr_to(expr, Some(dst))?;
        if result != dst {
            self.convert(result, dst)?;
        }
        Some(())
    }

    /// Compile `expr` wherever is natural (possibly an existing local's register).
    fn expr(&mut self, expr: &Expr) -> Option<Operand> {
        self.expr_to(expr, None)
    }

    /// Move/convert between registers.
    fn convert(&mut self, src: Operand, dst: Operand) -> Option<()> {
        if src == dst {
            return Some(());
        }
        let op = match (src.ty, dst.ty) {
            (a, b) if a.in_nums() && b.in_nums() => Opcode::Mov {
                dst: dst.reg,
                src: src.reg,
            },
            (Ty::List, Ty::List) => Opcode::ListMov {
                dst: dst.reg,
                src: src.reg,
            },
            (Ty::Val, Ty::Val) => Opcode::ValMov {
                dst: dst.reg,
                src: src.reg,
            },
            (Ty::Num, Ty::Val) => Opcode::BoxNum {
                dst: dst.reg,
                src: src.reg,
            },
            (Ty::Bool, Ty::Val) => Opcode::BoxBool {
                dst: dst.reg,
                src: src.reg,
            },
            (Ty::List, Ty::Val) => Opcode::ListToVal {
                dst: dst.reg,
                src: src.reg,
            },
            (Ty::Val, Ty::Num) => Opcode::UnboxNum {
                dst: dst.reg,
                src: src.reg,
            },
            (Ty::Val, Ty::Bool) => Opcode::UnboxBool {
                dst: dst.reg,
                src: src.reg,
            },
            (Ty::Val, Ty::List) => Opcode::ListFromVal {
                dst: dst.reg,
                src: src.reg,
            },
            _ => return None,
        };
        self.emit(op);
        Some(())
    }

    fn dst_or_temp(&mut self, dst: Option<Operand>, ty: Ty) -> Operand {
        match dst {
            Some(d) if d.ty == ty || (d.ty.in_nums() && ty.in_nums()) => d,
            _ => self.temp(ty),
        }
    }

    fn expr_to(&mut self, expr: &Expr, dst: Option<Operand>) -> Option<Operand> {
        match expr {
            Expr::Number(s) => {
                let reg = self.num_const(value::parse_number(s));
                Some(Operand { ty: Ty::Num, reg })
            }
            Expr::Bool(b) => {
                let reg = self.num_const(if *b { 1.0 } else { 0.0 });
                Some(Operand { ty: Ty::Bool, reg })
            }
            Expr::String(_) | Expr::Char(_) | Expr::Hamna => {
                let k = literal(self.exprs, expr)?;
                let k = self.program.constant(k);
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::ConstVal { dst: out.reg, k });
                Some(out)
            }
            Expr::Ident { name, .. } => {
                if let Some(local) = self.lookup(name) {
                    return Some(local.op);
                }
                let (ty, constant) = match self.program.module_consts.get(name.as_str()) {
                    Some((ty, constant, _)) => (*ty, constant.clone()),
                    // The predefined names (`Ukomo`, `PI`, …), as the tree-walker's outermost
                    // scope holds them.
                    None => crate::env::global_constants()
                        .into_iter()
                        .find(|(n, _)| n == name)
                        .and_then(|(_, v)| match v {
                            Value::Namba(n) => Some((Ty::Num, StoredConstant::Namba(n))),
                            Value::Ukweli(b) => Some((Ty::Bool, StoredConstant::Ukweli(b))),
                            Value::Neno(s) => Some((Ty::Val, StoredConstant::Neno(s.to_string()))),
                            Value::Tupu => Some((Ty::Val, StoredConstant::Tupu)),
                            _ => None,
                        })?,
                };
                match (ty, constant) {
                    (Ty::Num, StoredConstant::Namba(n)) => Some(Operand {
                        ty,
                        reg: self.num_const(n),
                    }),
                    (Ty::Bool, StoredConstant::Ukweli(b)) => Some(Operand {
                        ty,
                        reg: self.num_const(if b { 1.0 } else { 0.0 }),
                    }),
                    (_, constant) => {
                        let k = self.program.constant(constant);
                        let out = self.dst_or_temp(dst, Ty::Val);
                        self.emit(Opcode::ConstVal { dst: out.reg, k });
                        Some(out)
                    }
                }
            }
            Expr::Group(e) => self.expr_to(self.node(*e), dst),
            Expr::StructLiteral {
                struct_name,
                fields,
                ..
            } => {
                // Unknown `umbo` or a missing field: the tree-walker reports it when (if) this
                // runs, so leave the `kazi` to it.
                let declared = self.program.structs.get(struct_name)?.clone();
                let mut regs = Vec::with_capacity(declared.len());
                for (fname, _) in declared {
                    let (_, fexpr) = fields.iter().find(|(n, _)| *n == fname)?;
                    let reg = self.expr_as(self.node(*fexpr), Ty::Val)?.reg;
                    regs.push((self.program.constant(StoredConstant::Neno(fname)), reg));
                }
                let out = self.dst_or_temp(dst, Ty::Val);
                let name = self
                    .program
                    .constant(StoredConstant::Neno(struct_name.clone()));
                self.emit(Opcode::MakeStruct {
                    dst: out.reg,
                    name,
                    fields: regs.into_boxed_slice(),
                });
                Some(out)
            }
            Expr::FieldAccess {
                receiver, field, ..
            } => {
                let slot = self
                    .field_decl(self.node(*receiver), field)
                    .map_or(u32::MAX, |d| d.0);
                let src = self.expr_as(self.node(*receiver), Ty::Val)?;
                if self.infer(expr) == Ty::Num {
                    let out = self.dst_or_temp(dst, Ty::Num);
                    self.emit(Opcode::FieldNum {
                        dst: out.reg,
                        src: src.reg,
                        field: field.clone(),
                        slot,
                    });
                    return Some(out);
                }
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::Field {
                    dst: out.reg,
                    src: src.reg,
                    field: field.clone(),
                    slot,
                });
                Some(out)
            }
            Expr::EnumConstruct {
                enum_name,
                variant_name,
                data,
                ..
            } => {
                if !self.program.enums.contains(enum_name.as_str()) {
                    return None;
                }
                let data = match data {
                    Some(d) => Some(self.expr_as(self.node(*d), Ty::Val)?.reg),
                    None => None,
                };
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::MakeEnum {
                    dst: out.reg,
                    enum_name: *enum_name,
                    variant: *variant_name,
                    data,
                });
                Some(out)
            }
            // Literal keys only: they cannot fail to be keys, so building the map after every
            // entry is evaluated is indistinguishable from the tree-walker's pair-by-pair order.
            Expr::Map { entries, .. }
                if entries.iter().all(|(k, _)| {
                    matches!(
                        self.node(*k),
                        Expr::String(_) | Expr::Number(_) | Expr::Char(_) | Expr::Bool(_)
                    )
                }) =>
            {
                let mut regs = Vec::with_capacity(entries.len());
                for (k, v) in entries {
                    let k = self.expr_as(self.node(*k), Ty::Val)?.reg;
                    let v = self.expr_as(self.node(*v), Ty::Val)?.reg;
                    regs.push((k, v));
                }
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::MakeMap {
                    dst: out.reg,
                    entries: regs.into_boxed_slice(),
                });
                Some(out)
            }
            Expr::List { elements, .. } => {
                if dst.is_some_and(|d| d.ty == Ty::List)
                    && elements
                        .iter()
                        .all(|e| self.infer(self.node(*e)) == Ty::Num)
                {
                    let mut items = Vec::with_capacity(elements.len());
                    for e in elements {
                        items.push(self.expr_as(self.node(*e), Ty::Num)?.reg);
                    }
                    let out = dst?;
                    self.emit(Opcode::MakeNumList {
                        dst: out.reg,
                        items,
                    });
                    return Some(out);
                }
                let mut items = Vec::with_capacity(elements.len());
                for e in elements {
                    items.push(self.expr_as(self.node(*e), Ty::Val)?.reg);
                }
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::MakeList {
                    dst: out.reg,
                    items: items.into_boxed_slice(),
                });
                Some(out)
            }
            Expr::Index { base, index, .. } => {
                let base_op = self.expr(self.node(*base))?;
                if base_op.ty == Ty::List {
                    let idx = self.expr_as(self.node(*index), Ty::Num)?;
                    let out = self.dst_or_temp(dst, Ty::Num);
                    self.emit(Opcode::ListGet {
                        dst: out.reg,
                        list: base_op.reg,
                        idx: idx.reg,
                        mode: IndexMode::Element,
                    });
                    return Some(Operand {
                        ty: Ty::Num,
                        reg: out.reg,
                    });
                }
                let base_val = self.as_val(base_op)?;
                let idx = self.expr_as(self.node(*index), Ty::Val)?;
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::ValIndex {
                    dst: out.reg,
                    base: base_val.reg,
                    idx: idx.reg,
                    mode: IndexMode::Element,
                });
                Some(out)
            }
            Expr::Propagate { expr: inner, .. } => {
                if let Expr::Index { base, index, .. } = self.node(*inner) {
                    if self.infer(self.node(*base)) == Ty::List {
                        let list = self.expr_as(self.node(*base), Ty::List)?;
                        let idx = self.expr_as(self.node(*index), Ty::Num)?;
                        let out = self.dst_or_temp(dst, Ty::Num);
                        self.emit(Opcode::ListGet {
                            dst: out.reg,
                            list: list.reg,
                            idx: idx.reg,
                            mode: IndexMode::Tokeo,
                        });
                        return Some(Operand {
                            ty: Ty::Num,
                            reg: out.reg,
                        });
                    }
                }
                let src = self.tokeo_operand(self.node(*inner))?;
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::Unwrap {
                    dst: out.reg,
                    src: src.reg,
                });
                Some(out)
            }
            Expr::Cast {
                expr: inner, ty, ..
            } => {
                if self.infer(expr) == Ty::Num {
                    let src = self.expr_as(self.node(*inner), Ty::Num)?;
                    return Some(Operand {
                        ty: Ty::Num,
                        reg: src.reg,
                    });
                }
                let src = self.expr_as(self.node(*inner), Ty::Val)?;
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::Cast {
                    dst: out.reg,
                    src: src.reg,
                    ty: ty.name.clone().into_boxed_str(),
                });
                Some(out)
            }
            Expr::Unary {
                op, expr: inner, ..
            } => match op {
                UnaryOp::BorrowImm | UnaryOp::BorrowMut => self.expr_to(self.node(*inner), dst),
                UnaryOp::Not => {
                    let src = self.expr_as(self.node(*inner), Ty::Bool)?;
                    let out = self.dst_or_temp(dst, Ty::Bool);
                    self.emit(Opcode::Not {
                        dst: out.reg,
                        src: src.reg,
                    });
                    Some(Operand {
                        ty: Ty::Bool,
                        reg: out.reg,
                    })
                }
                UnaryOp::Neg | UnaryOp::BitNot if self.infer(self.node(*inner)) == Ty::Num => {
                    let src = self.expr_as(self.node(*inner), Ty::Num)?;
                    let out = self.dst_or_temp(dst, Ty::Num);
                    self.emit(if *op == UnaryOp::Neg {
                        Opcode::Neg {
                            dst: out.reg,
                            src: src.reg,
                        }
                    } else {
                        Opcode::BitNot {
                            dst: out.reg,
                            src: src.reg,
                        }
                    });
                    Some(Operand {
                        ty: Ty::Num,
                        reg: out.reg,
                    })
                }
                UnaryOp::Neg | UnaryOp::BitNot => {
                    let src = self.expr_as(self.node(*inner), Ty::Val)?;
                    let out = self.dst_or_temp(dst, Ty::Val);
                    self.emit(Opcode::ValUnary {
                        op: if *op == UnaryOp::Neg {
                            UnaryCode::Neg
                        } else {
                            UnaryCode::BitNot
                        },
                        dst: out.reg,
                        src: src.reg,
                    });
                    Some(out)
                }
                UnaryOp::Jaribu => {
                    let src = self.tokeo_operand(self.node(*inner))?;
                    let out = self.dst_or_temp(dst, Ty::Val);
                    self.emit(Opcode::Jaribu {
                        dst: out.reg,
                        src: src.reg,
                    });
                    Some(out)
                }
            },
            Expr::Binary {
                left, op, right, ..
            } => self.binary(self.node(*left), op, self.node(*right), dst),
            Expr::If {
                cond,
                then_expr,
                else_if,
                else_expr,
                ..
            } => {
                let ty = self.infer(expr);
                let out = match dst {
                    Some(d) if d.ty == ty => d,
                    _ => self.temp(ty),
                };
                let mut ends = Vec::new();
                let mut pending = self.cond_false_jumps(self.node(*cond))?;
                self.expr_into(self.node(*then_expr), out)?;
                ends.push(self.emit(Opcode::Jump { target: 0 }));
                for (c, e) in else_if {
                    let here = self.here();
                    for j in pending {
                        self.patch(j, here);
                    }
                    pending = self.cond_false_jumps(self.node(*c))?;
                    self.expr_into(self.node(*e), out)?;
                    ends.push(self.emit(Opcode::Jump { target: 0 }));
                }
                let here = self.here();
                for j in pending {
                    self.patch(j, here);
                }
                match else_expr {
                    Some(e) => self.expr_into(self.node(*e), out)?,
                    None => {
                        let k = self.program.constant(StoredConstant::Tupu);
                        let tmp = self.temp(Ty::Val);
                        self.emit(Opcode::ConstVal { dst: tmp.reg, k });
                        self.convert(tmp, out)?;
                    }
                }
                let end = self.here();
                for j in ends {
                    self.patch(j, end);
                }
                Some(out)
            }
            Expr::Call { callee, args, .. } => self.call(self.node(*callee), args, dst),
            Expr::MethodCall {
                receiver,
                method_name,
                args,
                ..
            } => self.method_call(self.node(*receiver), method_name, args, dst),
            // A map with computed keys: each key is checked as its entry is added, after the
            // entry is evaluated and before the next one is (the tree-walker's order), through
            // the shared `ingiza`. Built in a fresh register, so an entry may read `dst`.
            Expr::Map { entries, .. } => {
                let map = self.temp(Ty::Val);
                self.emit(Opcode::MakeMap {
                    dst: map.reg,
                    entries: Box::new([]),
                });
                let discard = self.temp(Ty::Val);
                for (k, v) in entries {
                    let k = self.expr_as(self.node(*k), Ty::Val)?.reg;
                    let v = self.expr_as(self.node(*v), Ty::Val)?.reg;
                    self.emit(Opcode::MutMethod(Box::new(MutMethodOp {
                        method: asili_parser::Name::new("ingiza"),
                        recv: map.reg,
                        args: vec![k, v],
                        dst: discard.reg,
                    })));
                }
                let out = self.dst_or_temp(dst, Ty::Val);
                self.convert(map, out)?;
                Some(out)
            }
        }
    }

    /// The value `?`/`jaribu` unwrap: for an index expression, the `Tokeo` form of the read
    /// (out of range as a `KosaMipaka` error); otherwise the expression itself.
    fn tokeo_operand(&mut self, expr: &Expr) -> Option<Operand> {
        let Expr::Index { base, index, .. } = expr else {
            return self.expr_as(expr, Ty::Val);
        };
        let base_op = self.expr(self.node(*base))?;
        let out = self.temp(Ty::Val);
        if base_op.ty == Ty::List {
            let idx = self.expr_as(self.node(*index), Ty::Num)?.reg;
            self.emit(Opcode::ListGetTokeo {
                dst: out.reg,
                list: base_op.reg,
                idx,
            });
        } else {
            let base = self.as_val(base_op)?.reg;
            let idx = self.expr_as(self.node(*index), Ty::Val)?.reg;
            self.emit(Opcode::ValIndex {
                dst: out.reg,
                base,
                idx,
                mode: IndexMode::Tokeo,
            });
        }
        Some(out)
    }

    fn as_val(&mut self, op: Operand) -> Option<Operand> {
        if op.ty == Ty::Val {
            return Some(op);
        }
        let dst = self.temp(Ty::Val);
        self.convert(op, dst)?;
        Some(dst)
    }

    fn binary(
        &mut self,
        left: &Expr,
        op: &BinaryOp,
        right: &Expr,
        dst: Option<Operand>,
    ) -> Option<Operand> {
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            // Short-circuit: `out = left; if out (is false | is true) skip; out = right`.
            // Always a fresh register: `b = a na b` must still read the old `b`.
            let out = self.temp(Ty::Bool);
            self.expr_into(left, out)?;
            let skip = self.emit(if *op == BinaryOp::And {
                Opcode::JumpIfFalse {
                    cond: out.reg,
                    target: 0,
                }
            } else {
                Opcode::JumpIfTrue {
                    cond: out.reg,
                    target: 0,
                }
            });
            self.expr_into(right, out)?;
            let end = self.here();
            self.patch(skip, end);
            if let Some(d) = dst {
                self.convert(out, d)?;
                return Some(d);
            }
            return Some(out);
        }
        let numeric = self.infer(left) == Ty::Num && self.infer(right) == Ty::Num;
        if let Some(cmp) = is_cmp(op) {
            let bool_ops = matches!(cmp, CmpOp::Eq | CmpOp::Ne)
                && self.infer(left) == Ty::Bool
                && self.infer(right) == Ty::Bool;
            if numeric || bool_ops {
                let ty = if numeric { Ty::Num } else { Ty::Bool };
                let a = self.expr_as(left, ty)?;
                let b = self.expr_as(right, ty)?;
                let out = self.dst_or_temp(dst, Ty::Bool);
                self.emit(Opcode::Cmp {
                    op: cmp,
                    dst: out.reg,
                    a: a.reg,
                    b: b.reg,
                });
                return Some(Operand {
                    ty: Ty::Bool,
                    reg: out.reg,
                });
            }
            let a = self.expr_as(left, Ty::Val)?;
            let b = self.expr_as(right, Ty::Val)?;
            let tmp = self.temp(Ty::Val);
            self.emit(Opcode::ValBinary {
                op: binary_code(op)?,
                dst: tmp.reg,
                a: a.reg,
                b: b.reg,
            });
            let out = self.dst_or_temp(dst, Ty::Bool);
            self.emit(Opcode::UnboxBool {
                dst: out.reg,
                src: tmp.reg,
            });
            return Some(Operand {
                ty: Ty::Bool,
                reg: out.reg,
            });
        }
        if numeric {
            let a = self.expr_as(left, Ty::Num)?.reg;
            let b = self.expr_as(right, Ty::Num)?.reg;
            let out = self.dst_or_temp(dst, Ty::Num).reg;
            self.emit(match op {
                BinaryOp::Add => Opcode::Add { dst: out, a, b },
                BinaryOp::Sub => Opcode::Sub { dst: out, a, b },
                BinaryOp::Mul => Opcode::Mul { dst: out, a, b },
                BinaryOp::Div => Opcode::Div { dst: out, a, b },
                BinaryOp::Rem => Opcode::Rem { dst: out, a, b },
                BinaryOp::Pow => Opcode::Pow { dst: out, a, b },
                BinaryOp::BitAnd => Opcode::BitAnd { dst: out, a, b },
                BinaryOp::BitOr => Opcode::BitOr { dst: out, a, b },
                BinaryOp::BitXor => Opcode::BitXor { dst: out, a, b },
                BinaryOp::Shl => Opcode::Shl { dst: out, a, b },
                BinaryOp::Shr => Opcode::Shr { dst: out, a, b },
                _ => return None,
            });
            return Some(Operand {
                ty: Ty::Num,
                reg: out,
            });
        }
        let a = self.expr_as(left, Ty::Val)?;
        let b = self.expr_as(right, Ty::Val)?;
        let out = self.dst_or_temp(dst, Ty::Val);
        self.emit(Opcode::ValBinary {
            op: binary_code(op)?,
            dst: out.reg,
            a: a.reg,
            b: b.reg,
        });
        Some(out)
    }

    fn call(&mut self, callee: &Expr, args: &[ExprId], dst: Option<Operand>) -> Option<Operand> {
        let Expr::Ident { name, .. } = callee else {
            return None;
        };
        // A call names a `kazi` or builtin; a local of the same name does not shadow it.
        // The program's own `kazi` shadows a builtin of the same name.
        let own = self.program.functions.contains_key(name.as_str());
        if let Some(builtin) = self
            .program
            .builtins
            .get(name.as_str())
            .copied()
            .filter(|_| !own)
        {
            if let (Some(out), "orodha_rudia", 2) = (dst, name.as_str(), args.len()) {
                if out.ty == Ty::List && self.infer(self.node(args[0])) == Ty::Num {
                    let value = self.expr_as(self.node(args[0]), Ty::Num)?.reg;
                    let count = self.expr_as(self.node(args[1]), Ty::Num)?.reg;
                    self.emit(Opcode::ListRepeat {
                        dst: out.reg,
                        value,
                        count,
                    });
                    return Some(out);
                }
            }
            if matches!(name.as_str(), "sakafu" | "dari")
                && args.len() == 1
                && self.infer(self.node(args[0])) == Ty::Num
            {
                let src = self.expr_as(self.node(args[0]), Ty::Num)?.reg;
                let out = self.dst_or_temp(dst, Ty::Num).reg;
                self.emit(if name == "sakafu" {
                    Opcode::Floor { dst: out, src }
                } else {
                    Opcode::Ceil { dst: out, src }
                });
                return Some(Operand {
                    ty: Ty::Num,
                    reg: out,
                });
            }
            let regs = self.val_args(args)?;
            let out = self.dst_or_temp(dst, Ty::Val);
            self.emit(Opcode::CallBuiltin(Box::new(BuiltinOp {
                builtin,
                args: regs,
                dst: out.reg,
            })));
            return Some(out);
        }
        let index = self.program.functions.get(name.as_str())?.index;
        {
            let exprs = self.exprs;
            self.call_index(index, args.iter().map(|a| &exprs[*a]), dst)
        }
    }

    /// Call program function `index` with `args` evaluated in order.
    fn call_index<'e>(
        &mut self,
        index: u32,
        args: impl Iterator<Item = &'e Expr>,
        dst: Option<Operand>,
    ) -> Option<Operand> {
        let (params, ret, inline) = {
            let sig = self.program.functions.values().find(|s| s.index == index)?;
            (sig.params.clone(), sig.ret, sig.inline.clone())
        };
        let args: Vec<&Expr> = args.collect();
        if params.len() != args.len() {
            return None;
        }
        let mut operands = Vec::with_capacity(params.len());
        for (arg, ty) in args.into_iter().zip(params) {
            operands.push(self.expr_as(arg, ty)?);
        }
        let out = match dst {
            Some(d) if d.ty == ret => d,
            _ => self.temp(ret),
        };
        if let Some(inline) = inline.filter(|_| !self.program.options.lines) {
            if self.inline_call(&inline, &operands, out).is_some() {
                return Some(out);
            }
        }
        self.emit(Opcode::Call(Box::new(CallOp {
            function: index,
            args: operands,
            dst: out,
        })));
        Some(out)
    }

    /// `inline`'s body with its parameters bound to `args`, into `out`; on `None` nothing was
    /// emitted and the caller makes an ordinary call.
    fn inline_call(&mut self, inline: &Inline, args: &[Operand], out: Operand) -> Option<()> {
        let mark = (self.code.len(), self.program.failed_line);
        let params: HashMap<String, Local> = inline
            .params
            .iter()
            .zip(args)
            .map(|((name, ty), op)| {
                let local = Local {
                    op: *op,
                    type_name: Some(ty.clone()),
                };
                (name.clone(), local)
            })
            .collect();
        let scopes = std::mem::replace(&mut self.scopes, vec![params]);
        // Write the result last: compile into a fresh register when `out` is also an argument.
        let file = |ty: Ty| match ty {
            Ty::Num | Ty::Bool => 0,
            Ty::List => 1,
            Ty::Val => 2,
        };
        let into = if args
            .iter()
            .any(|a| file(a.ty) == file(out.ty) && a.reg == out.reg)
        {
            self.temp(out.ty)
        } else {
            out
        };
        self.emit(Opcode::CheckDepth);
        let body = self.node(inline.body);
        let done = self.expr_into(body, into).and_then(|()| {
            if into == out {
                Some(())
            } else {
                self.convert(into, out)
            }
        });
        self.scopes = scopes;
        if done.is_none() {
            self.code.truncate(mark.0);
            self.program.failed_line = mark.1;
        }
        done
    }

    fn method_call(
        &mut self,
        receiver: &Expr,
        method: &str,
        args: &[ExprId],
        dst: Option<Operand>,
    ) -> Option<Operand> {
        if self.program.impl_methods.contains(method) {
            // A user method: called directly when the receiver's `umbo` is known here, else
            // dispatched on the receiver at run time (below).
            let index = self
                .type_name(receiver)
                .and_then(|target| self.program.impl_index.get(&(target, method.to_string())))
                .copied();
            if let Some(index) = index {
                let exprs = self.exprs;
                return self.call_index(
                    index,
                    std::iter::once(receiver).chain(args.iter().map(|a| &exprs[*a])),
                    dst,
                );
            }
        }
        let recv_ty = self.infer(receiver);
        // Numeric-list fast paths.
        if recv_ty == Ty::List {
            let local = match receiver {
                Expr::Ident { name, .. } => self.lookup(name).map(|l| l.op),
                _ => None,
            };
            match (method, args.len(), local) {
                ("urefu", 0, _) => {
                    let list = self.expr_as(receiver, Ty::List)?.reg;
                    let out = self.dst_or_temp(dst, Ty::Num).reg;
                    self.emit(Opcode::ListLen { dst: out, list });
                    return Some(Operand {
                        ty: Ty::Num,
                        reg: out,
                    });
                }
                ("ongeza", 1, Some(list)) => {
                    let src = self.expr_as(self.node(args[0]), Ty::Num)?.reg;
                    self.emit(Opcode::ListPush {
                        list: list.reg,
                        src,
                    });
                    return self.tupu(dst);
                }
                ("ingiza", 2, Some(list)) => {
                    let idx = self.expr_as(self.node(args[0]), Ty::Num)?.reg;
                    let src = self.expr_as(self.node(args[1]), Ty::Num)?.reg;
                    self.emit(Opcode::ListSet {
                        list: list.reg,
                        idx,
                        src,
                    });
                    return self.tupu(dst);
                }
                ("ondoa", 1, Some(list)) => {
                    let idx = self.expr_as(self.node(args[0]), Ty::Num)?.reg;
                    let out = self.dst_or_temp(dst, Ty::Val);
                    self.emit(Opcode::ListRemoveVal {
                        dst: out.reg,
                        list: list.reg,
                        idx,
                    });
                    return Some(out);
                }
                (_, _, Some(list)) if methods::is_mutating(&Value::list(Vec::new()), method) => {
                    let regs = self.val_args(args)?;
                    let out = self.dst_or_temp(dst, Ty::Val);
                    self.emit(Opcode::ListMutate(Box::new(MutMethodOp {
                        method: asili_parser::Name::new(method),
                        recv: list.reg,
                        args: regs,
                        dst: out.reg,
                    })));
                    return Some(out);
                }
                _ => {}
            }
        }
        let Some(probe) = self.type_name(receiver).and_then(|t| probe_value(&t)) else {
            return self.dynamic_method_call(receiver, method, args, dst);
        };
        // In-place mutation of a generic local.
        if methods::is_mutating(&probe, method) {
            if let Expr::Ident { name, .. } = receiver {
                let local = self.lookup(name)?.op;
                if local.ty != Ty::Val {
                    return None;
                }
                return self.emit_mut_method(method, local.reg, args, dst);
            }
        }
        // `clona` of text or a list is a copy of the value (shared until written).
        if method == "clona"
            && args.is_empty()
            && matches!(probe, Value::Neno(_) | Value::Orodha(_))
        {
            let src = self.expr_as(receiver, Ty::Val)?;
            let out = self.dst_or_temp(dst, Ty::Val);
            self.convert(src, out)?;
            return Some(out);
        }
        if method == "urefu"
            && args.is_empty()
            && matches!(probe, Value::Neno(_) | Value::Orodha(_))
        {
            let src = self.expr_as(receiver, Ty::Val)?.reg;
            let out = self.dst_or_temp(dst, Ty::Num).reg;
            self.emit(Opcode::ValLen { dst: out, src });
            return Some(Operand {
                ty: Ty::Num,
                reg: out,
            });
        }
        // Everything else (including a method the type does not have, an error at run time)
        // through the shared method table.
        self.emit_call_method(receiver, method, args, dst)
    }

    /// A method call whose receiver type is not known statically: dispatched at run time
    /// through the shared method table, like the evaluator does.
    fn dynamic_method_call(
        &mut self,
        receiver: &Expr,
        method: &str,
        args: &[ExprId],
        dst: Option<Operand>,
    ) -> Option<Operand> {
        let local = match receiver {
            Expr::Ident { name, .. } => self.lookup(name).map(|l| l.op),
            _ => None,
        };
        // A named generic local may be mutated in place.
        if let Some(local) = local.filter(|l| l.ty == Ty::Val) {
            return self.emit_mut_method(method, local.reg, args, dst);
        }
        self.emit_call_method(receiver, method, args, dst)
    }

    /// Compile `args` into generic registers, in order.
    fn val_args(&mut self, args: &[ExprId]) -> Option<Vec<Reg>> {
        args.iter()
            .map(|arg| self.expr_as(self.node(*arg), Ty::Val).map(|op| op.reg))
            .collect()
    }

    /// `recv.method(args)` mutating the generic local `recv` in place.
    fn emit_mut_method(
        &mut self,
        method: &str,
        recv: Reg,
        args: &[ExprId],
        dst: Option<Operand>,
    ) -> Option<Operand> {
        let args = self.val_args(args)?;
        let out = self.dst_or_temp(dst, Ty::Val);
        self.emit(Opcode::MutMethod(Box::new(MutMethodOp {
            method: asili_parser::Name::new(method),
            recv,
            args,
            dst: out.reg,
        })));
        Some(out)
    }

    /// `receiver.method(args)` through the shared method table (receiver evaluated first).
    fn emit_call_method(
        &mut self,
        receiver: &Expr,
        method: &str,
        args: &[ExprId],
        dst: Option<Operand>,
    ) -> Option<Operand> {
        let recv = self.expr_as(receiver, Ty::Val)?.reg;
        let args = self.val_args(args)?;
        let out = self.dst_or_temp(dst, Ty::Val);
        self.emit(Opcode::CallMethod(Box::new(MethodOp {
            method: asili_parser::Name::new(method),
            recv,
            args,
            dst: out.reg,
        })));
        Some(out)
    }

    fn tupu(&mut self, dst: Option<Operand>) -> Option<Operand> {
        let k = self.program.constant(StoredConstant::Tupu);
        let out = self.dst_or_temp(dst, Ty::Val);
        if out.ty == Ty::Val {
            self.emit(Opcode::ConstVal { dst: out.reg, k });
        }
        Some(out)
    }
}

// ---------------------------------------------------------------------------------------------
// Running bytecode
// ---------------------------------------------------------------------------------------------

/// Native code for `program`, compiled in memory, or why there is none: no backend for this
/// platform, or a build failure.
pub(crate) fn native_for(
    program: &BytecodeProgram,
) -> Result<crate::aot::NativeLibrary, EvalError> {
    if !crate::nguvu::supported() {
        return Err(EvalError::Unknown(
            "jukwaa hili halina msimbo asilia (nguvu)".into(),
        ));
    }
    crate::nguvu::compile(program)
        .map_err(|e| EvalError::Unknown(format!("msimbo asilia haukujengwa: {e}")))
}

/// The syntax tree a bytecode program carries, for the tree-walker.
fn tree_of(program: &BytecodeProgram) -> Result<&Module, EvalError> {
    program
        .ast
        .as_ref()
        .ok_or_else(|| EvalError::Unknown("kilele hakina mti wa programu".into()))
}

/// Run `kuu(hoja)` as native code.
pub fn run_bytecode(program: &BytecodeProgram, args: Vec<String>) -> Result<(), EvalError> {
    run_bytecode_native(program, None, args)
}

/// Run `kuu(hoja)` on `library` (built for `program`, e.g. the image `pata jenga` wrote); without
/// one, native code is compiled in memory.
pub fn run_bytecode_native(
    program: &BytecodeProgram,
    library: Option<&crate::aot::NativeLibrary>,
    args: Vec<String>,
) -> Result<(), EvalError> {
    let built;
    let library = match library {
        Some(library) => library,
        None => {
            built = native_for(program)?;
            &built
        }
    };
    let hoja = Value::list(args.into_iter().map(Value::neno).collect());
    crate::host::Host::new(program, library, None).call_by_name(&program.entry, vec![hoja])?;
    Ok(())
}

/// Run `kuu` of a program other threads may share (`tenda`, server workers run its `kazi` on
/// the same native code).
pub(crate) fn run_shared_program(
    program: std::sync::Arc<BytecodeProgram>,
    library: Option<std::sync::Arc<crate::aot::NativeLibrary>>,
    args: Vec<String>,
) -> Result<(), EvalError> {
    let native = match library {
        Some(native) => native,
        None => std::sync::Arc::new(native_for(&program)?),
    };
    let shared = crate::spawn::Shared::Code {
        program: program.clone(),
        native: native.clone(),
    };
    let hoja = Value::list(args.into_iter().map(Value::neno).collect());
    crate::host::Host::new(&program, &native, Some(shared))
        .call_by_name(&program.entry, vec![hoja])?;
    Ok(())
}

/// Which engine runs a program's functions, for differential testing.
#[derive(Clone, Copy)]
pub enum Engine<'l> {
    /// The tree-walking evaluator, on the syntax tree the program carries.
    Tree,
    /// Native code built for this program.
    Native(&'l crate::aot::NativeLibrary),
}

/// Run a named function on a specific engine.
pub fn run_bytecode_function_on(
    engine: Engine<'_>,
    program: &BytecodeProgram,
    name: &str,
    args: Vec<Value>,
) -> Result<Value, EvalError> {
    match engine {
        Engine::Native(library) => {
            crate::host::Host::new(program, library, None).call_by_name(name, args)
        }
        Engine::Tree => {
            let module = tree_of(program)?;
            let f =
                ast_function(module, name).ok_or_else(|| EvalError::UndefinedVar(name.into()))?;
            crate::TreeContext::new(module)?.call(module, f, args)
        }
    }
}

/// Execute a named function as native code. Useful for embedders and focused tests; the CLI entry point above keeps the
/// `kuu(hoja)` interface.
pub fn run_bytecode_function(
    program: &BytecodeProgram,
    name: &str,
    args: Vec<Value>,
) -> Result<Value, EvalError> {
    run_bytecode_function_on(Engine::Native(&native_for(program)?), program, name, args)
}

/// Every `shughuli ya` method, inherent blocks first (the order the tree-walker searches them).
fn impl_methods(module: &Module) -> Vec<(&asili_parser::ImplDecl, &Function)> {
    let inherent = module.impls.iter().filter(|i| i.trait_name.is_none());
    let traits = module.impls.iter().filter(|i| i.trait_name.is_some());
    inherent
        .chain(traits)
        .flat_map(|i| i.body.iter().map(move |f| (i, f)))
        .collect()
}

/// The bytecode function name of a `shughuli ya` method: `Umbo::njia`, or `Umbo<Sifa>::njia`
/// for a trait's.
pub(crate) fn impl_function_name(target: &str, trait_name: Option<&str>, method: &str) -> String {
    match trait_name {
        None => format!("{target}::{method}"),
        Some(t) => format!("{target}<{t}>::{method}"),
    }
}

/// The syntax tree of program function `name` (a module function, or a method by its
/// `impl_function_name`).
pub(crate) fn ast_function<'m>(module: &'m Module, name: &str) -> Option<&'m Function> {
    module
        .functions
        .iter()
        .find(|f| f.name == name)
        .or_else(|| {
            module.impls.iter().find_map(|i| {
                i.body.iter().find(|f| {
                    impl_function_name(&i.target, i.trait_name.as_deref(), &f.name) == name
                })
            })
        })
}

/// The names a pattern binds, each once, in first-bound order.
fn pattern_names(pat: &Pattern, out: &mut Vec<String>) {
    match pat {
        Pattern::Ident { name, .. } => {
            if !out.iter().any(|n| n == name) {
                out.push(name.to_string());
            }
        }
        Pattern::Struct { fields, .. } => {
            for (_, p) in fields {
                pattern_names(p, out);
            }
        }
        Pattern::Enum { data: Some(p), .. } => pattern_names(p, out),
        Pattern::Jozi(a, b) => {
            pattern_names(a, out);
            pattern_names(b, out);
        }
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::Enum { data: None, .. } => {}
    }
}
