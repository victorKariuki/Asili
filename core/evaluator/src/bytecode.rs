//! The ASB bytecode compiler and typed register VM.
//!
//! The compiler lowers a conservative, performance-sensitive subset of Asili to a register
//! machine with three register files per call frame:
//!
//! * `nums`: unboxed `f64`s, holding every `Namba` and `Ukweli` (as `0.0`/`1.0`) local and
//!   temporary whose type is statically known;
//! * `lists`: unboxed `Vec<f64>`s for `Orodha<Namba>` locals;
//! * `vals`: generic [`Value`]s for everything else.
//!
//! Numeric code therefore never touches the `Value` enum: `a + b` is one `Add` instruction on two
//! `f64` registers, `ikiwa x < y` is one fused compare-and-branch, `b[i]?` on an
//! `Orodha<Namba>` is one bounds-checked load, and every numeric literal lives in a register that
//! is filled once when the frame is entered. Generic operations reuse the tree-walking
//! evaluator's shared helpers (`eval::methods`) so both execution paths agree on behaviour and
//! error text.
//!
//! Programs containing syntax that is not represented here make [`compile_module`] return
//! `None`, and the caller keeps emitting the serialized-AST artifact instead.

use crate::builtins::{builtin_names, builtins, BuiltinFn};
use crate::eval::methods;
use crate::value::{self, EvalError, MapKey, Value};
use asili_parser::{AssignOp, BinaryOp, Block, Expr, ForMode, Function, Module, Stmt, UnaryOp};
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
    pub params: Vec<Operand>,
    pub ret: Ty,
    pub num_regs: u32,
    pub list_regs: u32,
    pub val_regs: u32,
    /// Numeric constants copied into their registers on frame entry.
    pub num_consts: Vec<(Reg, f64)>,
    pub code: Vec<Opcode>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryCode {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    BitAnd,
    BitXor,
    BitOr,
    Shl,
    Shr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryCode {
    Neg,
    BitNot,
}

/// In-place methods on a generic local (`Orodha`/`Kamusi` held in the `vals` file).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutMethod {
    Push,
    Insert,
    Remove,
    WekaKey,
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
    pub method: String,
    pub recv: Reg,
    pub args: Vec<Reg>,
    pub dst: Reg,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MutMethodOp {
    pub method: MutMethod,
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
    /// `b[i]?` on an `Orodha<Namba>`: out of bounds returns the `Tokeo` error from the function.
    ListGet {
        dst: Reg,
        list: Reg,
        idx: Reg,
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
        op: BinaryCode,
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BytecodeProgram {
    pub constants: Vec<StoredConstant>,
    pub functions: Vec<BytecodeFunc>,
    pub entry: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StoredConstant {
    Neno(String),
    Namba(f64),
    Tupu,
    Hamna,
    Ukweli(bool),
    Herufi(char),
}

impl StoredConstant {
    fn to_value(&self) -> Value {
        match self {
            StoredConstant::Neno(s) => Value::Neno(s.clone()),
            StoredConstant::Namba(n) => Value::Namba(*n),
            StoredConstant::Ukweli(b) => Value::Ukweli(*b),
            StoredConstant::Herufi(c) => Value::Herufi(*c),
            StoredConstant::Tupu => Value::Tupu,
            StoredConstant::Hamna => Value::Hamna,
        }
    }
}

impl BytecodeProgram {
    pub fn find_function(&self, name: &str) -> Option<&BytecodeFunc> {
        self.functions.iter().find(|f| f.name == name)
    }
}

// ---------------------------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------------------------

/// Builtins that need the whole `Module` (they spawn named `kazi`), which the VM does not carry.
const MODULE_BUILTINS: &[&str] = &["tenda", "mkondo_tumikia", "mkondo_tumikia_http"];

/// `Orodha` methods that take the name of a callback `kazi`.
const CALLBACK_METHODS: &[&str] = &["ramani", "chuja", "hesabu", "chunguza", "kila_na_fahirisi"];

/// Lower the supported subset. `None` means the caller must emit the serialized AST artifact.
pub fn compile_module(module: &Module) -> Option<BytecodeProgram> {
    let mut program = ProgramCompiler {
        constants: Vec::new(),
        functions: HashMap::new(),
        module_consts: HashMap::new(),
        builtins: builtin_names()
            .into_iter()
            .enumerate()
            .map(|(i, name)| (name, i as u32))
            .collect(),
    };
    for (i, function) in module.functions.iter().enumerate() {
        program.functions.insert(
            function.name.clone(),
            FunctionSig {
                index: i as u32,
                params: function
                    .params
                    .iter()
                    .map(|p| Ty::from_type_name(&p.ty.name))
                    .collect(),
                ret: Ty::from_type_name(&function.return_type.name),
                ret_name: function.return_type.name.clone(),
            },
        );
    }
    for constant in &module.constants {
        let stored = literal(&constant.value)?;
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
    let mut functions = Vec::with_capacity(module.functions.len());
    for function in &module.functions {
        functions.push(FunctionCompiler::compile(&mut program, function)?);
    }
    Some(BytecodeProgram {
        constants: program.constants,
        functions,
        entry: "kuu".to_string(),
    })
}

fn literal(expr: &Expr) -> Option<StoredConstant> {
    Some(match expr {
        Expr::Number(s) => StoredConstant::Namba(value::parse_number(s)),
        Expr::String(s) => StoredConstant::Neno(s.clone()),
        Expr::Bool(b) => StoredConstant::Ukweli(*b),
        Expr::Char(c) => StoredConstant::Herufi(*c),
        Expr::Hamna => StoredConstant::Hamna,
        Expr::Group(e) => return literal(e),
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
            ..
        } => match literal(expr)? {
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
}

struct ProgramCompiler {
    constants: Vec<StoredConstant>,
    functions: HashMap<String, FunctionSig>,
    /// name -> (type, value, declared type name)
    module_consts: HashMap<String, (Ty, StoredConstant, String)>,
    builtins: HashMap<String, u32>,
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
    breaks: Vec<usize>,
    continues: Vec<usize>,
}

struct FunctionCompiler<'a> {
    program: &'a mut ProgramCompiler,
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

fn binary_code(op: &BinaryOp) -> Option<BinaryCode> {
    Some(match op {
        BinaryOp::Add => BinaryCode::Add,
        BinaryOp::Sub => BinaryCode::Sub,
        BinaryOp::Mul => BinaryCode::Mul,
        BinaryOp::Div => BinaryCode::Div,
        BinaryOp::Rem => BinaryCode::Rem,
        BinaryOp::Pow => BinaryCode::Pow,
        BinaryOp::Eq => BinaryCode::Eq,
        BinaryOp::Ne => BinaryCode::Ne,
        BinaryOp::Gt => BinaryCode::Gt,
        BinaryOp::Lt => BinaryCode::Lt,
        BinaryOp::Ge => BinaryCode::Ge,
        BinaryOp::Le => BinaryCode::Le,
        BinaryOp::BitAnd => BinaryCode::BitAnd,
        BinaryOp::BitXor => BinaryCode::BitXor,
        BinaryOp::BitOr => BinaryCode::BitOr,
        BinaryOp::Shl => BinaryCode::Shl,
        BinaryOp::Shr => BinaryCode::Shr,
        BinaryOp::And | BinaryOp::Or => return None,
    })
}

/// Static result type name of a supported method, for chaining (`b.vipande(9).ramani(..)`).
fn method_result_type(receiver: &str, method: &str) -> Option<&'static str> {
    if receiver.starts_with("Orodha") {
        match method {
            "clona" | "vipande" | "kwa_neno" | "ramani" | "chuja" => Some("Orodha"),
            "jiunge" | "unganisha" => Some("Neno"),
            _ => None,
        }
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

fn pure_method_supported(receiver: &str, method: &str) -> bool {
    let probe = if receiver.starts_with("Orodha") {
        Value::Orodha(Vec::new())
    } else if receiver == "Neno" {
        Value::Neno(String::new())
    } else {
        return false;
    };
    methods::is_pure_method(&probe, method)
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
    fn compile(program: &'a mut ProgramCompiler, function: &Function) -> Option<BytecodeFunc> {
        let ret = Ty::from_type_name(&function.return_type.name);
        let mut f = FunctionCompiler {
            program,
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
            name: function.name.clone(),
            params,
            ret,
            num_regs: f.num_regs,
            list_regs: f.list_regs,
            val_regs: f.val_regs,
            num_consts: f.num_consts,
            code: f.code,
        })
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
                None => self
                    .program
                    .module_consts
                    .get(name)
                    .map(|(ty, _, _)| *ty)
                    .unwrap_or(Ty::Val),
            },
            Expr::Group(e) => self.infer(e),
            Expr::Unary { op, expr, .. } => match op {
                UnaryOp::Neg | UnaryOp::BitNot if self.infer(expr) == Ty::Num => Ty::Num,
                UnaryOp::Not => Ty::Bool,
                UnaryOp::BorrowImm | UnaryOp::BorrowMut => self.infer(expr),
                _ => Ty::Val,
            },
            Expr::Binary {
                left, op, right, ..
            } => {
                if is_cmp(op).is_some() || matches!(op, BinaryOp::And | BinaryOp::Or) {
                    Ty::Bool
                } else if self.infer(left) == Ty::Num && self.infer(right) == Ty::Num {
                    Ty::Num
                } else {
                    Ty::Val
                }
            }
            Expr::Cast { expr, ty, .. } => {
                if ty.name.replace(' ', "") == "Namba" && self.infer(expr).in_nums() {
                    Ty::Num
                } else {
                    Ty::Val
                }
            }
            Expr::Call { callee, args, .. } => match &**callee {
                Expr::Ident { name, .. } => {
                    if matches!(name.as_str(), "sakafu" | "dari")
                        && args.len() == 1
                        && self.infer(&args[0]) == Ty::Num
                        && self.lookup(name).is_none()
                    {
                        Ty::Num
                    } else if self.program.builtins.contains_key(name) {
                        Ty::Val
                    } else {
                        self.program
                            .functions
                            .get(name)
                            .map(|f| f.ret)
                            .unwrap_or(Ty::Val)
                    }
                }
                _ => Ty::Val,
            },
            Expr::MethodCall {
                method_name, args, ..
            } if method_name == "urefu" && args.is_empty() => Ty::Num,
            Expr::Propagate { expr, .. } => match &**expr {
                Expr::Index { base, .. } if self.infer(base) == Ty::List => Ty::Num,
                _ => Ty::Val,
            },
            Expr::If {
                then_expr,
                else_if,
                else_expr,
                ..
            } => {
                let mut ty = self.infer(then_expr);
                for (_, e) in else_if {
                    if self.infer(e) != ty {
                        ty = Ty::Val;
                    }
                }
                match else_expr {
                    Some(e) if self.infer(e) == ty => ty,
                    _ => Ty::Val,
                }
            }
            _ => Ty::Val,
        }
    }

    /// Static type name of a generic expression, where known.
    fn type_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::String(_) => Some("Neno".into()),
            Expr::List { .. } => Some("Orodha".into()),
            Expr::Group(e) => self.type_name(e),
            Expr::Ident { name, .. } => match self.lookup(name) {
                Some(local) if local.op.ty == Ty::List => Some("Orodha<Namba>".into()),
                Some(local) => local.type_name.clone(),
                None => self
                    .program
                    .module_consts
                    .get(name)
                    .map(|(_, _, t)| t.clone()),
            },
            Expr::Cast { ty, .. } if ty.name == "Neno" => Some("Neno".into()),
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } if self.type_name(left).as_deref() == Some("Neno")
                || self.type_name(right).as_deref() == Some("Neno") =>
            {
                Some("Neno".into())
            }
            Expr::MethodCall {
                receiver,
                method_name,
                ..
            } => {
                let recv = self.type_name(receiver)?;
                method_result_type(&recv, method_name).map(str::to_string)
            }
            Expr::Call { callee, .. } => match &**callee {
                Expr::Ident { name, .. } if !self.program.builtins.contains_key(name) => {
                    self.program.functions.get(name).map(|f| f.ret_name.clone())
                }
                _ => None,
            },
            _ => None,
        }
    }

    // -- statements ---------------------------------------------------------------------------

    fn block(&mut self, block: &Block) -> Option<()> {
        self.scopes.push(HashMap::new());
        for stmt in &block.statements {
            self.stmt(stmt)?;
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
                        let inferred = self.infer(value);
                        let inferred = if inferred == Ty::Val && self.numeric_list_literal(value) {
                            Ty::List
                        } else {
                            inferred
                        };
                        (inferred, self.type_name(value))
                    }
                };
                // The initializer is evaluated before the new binding is visible.
                let dst = self.temp(declared);
                self.expr_into(value, dst)?;
                self.scopes
                    .last_mut()?
                    .insert(name.clone(), Local { op: dst, type_name });
            }
            Stmt::Assign {
                name, op, value, ..
            } => {
                let dst = self.lookup(name)?.op;
                match op {
                    AssignOp::Assign => self.expr_into(value, dst)?,
                    compound => {
                        let bin = match compound {
                            AssignOp::AddAssign => BinaryOp::Add,
                            AssignOp::SubAssign => BinaryOp::Sub,
                            AssignOp::MulAssign => BinaryOp::Mul,
                            AssignOp::DivAssign => BinaryOp::Div,
                            AssignOp::Assign => unreachable!(),
                        };
                        let current = Expr::Ident {
                            name: name.clone(),
                            line: 0,
                            column: 0,
                        };
                        self.binary(&current, &bin, value, Some(dst))?;
                    }
                }
            }
            Stmt::Expr { expr, .. } => self.expr_stmt(expr)?,
            Stmt::Return { value, .. } => match value {
                Some(value) => {
                    let src = self.expr_as(value, self.ret)?;
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
                let skip = self.cond_false_jumps(cond)?;
                self.block(then_block)?;
                ends.push(self.emit(Opcode::Jump { target: 0 }));
                let mut pending = skip;
                for (cond, block) in else_if {
                    let here = self.here();
                    for j in pending {
                        self.patch(j, here);
                    }
                    pending = self.cond_false_jumps(cond)?;
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
                let exits = self.cond_false_jumps(cond)?;
                self.loops.push(LoopState {
                    label: label.clone(),
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
            Stmt::Match { .. } | Stmt::Drop { .. } | Stmt::LetPattern { .. } => return None,
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
                !elements.is_empty() && elements.iter().all(|e| self.infer(e) == Ty::Num)
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
                let start = self.expr_as(start, Ty::Num)?;
                let end = self.expr_as(end, Ty::Num)?;
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
                let (source, item) = if self.infer(collection) == Ty::List {
                    let snapshot = self.temp(Ty::List);
                    self.expr_into(collection, snapshot)?;
                    self.emit(Opcode::ListLen {
                        dst: len,
                        list: snapshot.reg,
                    });
                    let item = self.declare(var, Ty::Num, Some("Namba".into()));
                    (snapshot, item)
                } else {
                    // `Kamusi` iteration (key/value pairs) is left to the evaluator.
                    let type_name = self.type_name(collection)?;
                    if !type_name.starts_with("Orodha") {
                        return None;
                    }
                    let elem = type_name
                        .strip_prefix("Orodha<")
                        .and_then(|rest| rest.strip_suffix('>'))
                        .map(str::to_string);
                    let snapshot = self.temp(Ty::Val);
                    self.expr_into(collection, snapshot)?;
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
                if source.ty == Ty::List {
                    self.emit(Opcode::ListGet {
                        dst: item.reg,
                        list: source.reg,
                        idx,
                    });
                } else {
                    let boxed_idx = self.temp(Ty::Val).reg;
                    let tokeo = self.temp(Ty::Val).reg;
                    let unwrapped = self.temp(Ty::Val);
                    self.emit(Opcode::BoxNum {
                        dst: boxed_idx,
                        src: idx,
                    });
                    self.emit(Opcode::ValIndex {
                        dst: tokeo,
                        base: source.reg,
                        idx: boxed_idx,
                    });
                    self.emit(Opcode::Unwrap {
                        dst: unwrapped.reg,
                        src: tokeo,
                    });
                    self.convert(unwrapped, item)?;
                }
                self.loops.push(LoopState {
                    label,
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
            if let Expr::Ident { name, .. } = &**receiver {
                if let Some(local) = self.lookup(name).cloned() {
                    // Statement-level numeric-list mutations: no `Tupu`/`Chaguo` result.
                    let list = local.op.reg;
                    match (local.op.ty, method_name.as_str(), args.len()) {
                        (Ty::List, "ondoa", 1) => {
                            let idx = self.expr_as(&args[0], Ty::Num)?.reg;
                            self.emit(Opcode::ListRemove { list, idx });
                            return Some(());
                        }
                        (Ty::List, "ongeza", 1) => {
                            let src = self.expr_as(&args[0], Ty::Num)?.reg;
                            self.emit(Opcode::ListPush { list, src });
                            return Some(());
                        }
                        (Ty::List, "ingiza", 2) => {
                            let idx = self.expr_as(&args[0], Ty::Num)?.reg;
                            let src = self.expr_as(&args[1], Ty::Num)?.reg;
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
            Expr::Group(inner) => self.cond_false_jumps(inner),
            Expr::Binary {
                left,
                op: BinaryOp::And,
                right,
                ..
            } => {
                let mut jumps = self.cond_false_jumps(left)?;
                jumps.extend(self.cond_false_jumps(right)?);
                Some(jumps)
            }
            Expr::Binary {
                left, op, right, ..
            } if is_cmp(op).is_some()
                && self.infer(left) == Ty::Num
                && self.infer(right) == Ty::Num =>
            {
                let a = self.expr_as(left, Ty::Num)?;
                let b = self.expr_as(right, Ty::Num)?;
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
                let k = literal(expr)?;
                let k = self.program.constant(k);
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::ConstVal { dst: out.reg, k });
                Some(out)
            }
            Expr::Ident { name, .. } => {
                if let Some(local) = self.lookup(name) {
                    return Some(local.op);
                }
                let (ty, constant, _) = self.program.module_consts.get(name)?.clone();
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
            Expr::Group(e) => self.expr_to(e, dst),
            Expr::List { elements, .. } => {
                if dst.is_some_and(|d| d.ty == Ty::List)
                    && elements.iter().all(|e| self.infer(e) == Ty::Num)
                {
                    let mut items = Vec::with_capacity(elements.len());
                    for e in elements {
                        items.push(self.expr_as(e, Ty::Num)?.reg);
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
                    items.push(self.expr_as(e, Ty::Val)?.reg);
                }
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::MakeList {
                    dst: out.reg,
                    items: items.into_boxed_slice(),
                });
                Some(out)
            }
            Expr::Index { base, index, .. } => {
                let base_op = self.expr(base)?;
                if base_op.ty == Ty::List {
                    let idx = self.expr_as(index, Ty::Num)?;
                    let out = self.dst_or_temp(dst, Ty::Val);
                    self.emit(Opcode::ListGetTokeo {
                        dst: out.reg,
                        list: base_op.reg,
                        idx: idx.reg,
                    });
                    return Some(out);
                }
                let base_val = self.as_val(base_op)?;
                let idx = self.expr_as(index, Ty::Val)?;
                let out = self.dst_or_temp(dst, Ty::Val);
                self.emit(Opcode::ValIndex {
                    dst: out.reg,
                    base: base_val.reg,
                    idx: idx.reg,
                });
                Some(out)
            }
            Expr::Propagate { expr: inner, .. } => {
                if let Expr::Index { base, index, .. } = &**inner {
                    if self.infer(base) == Ty::List {
                        let list = self.expr_as(base, Ty::List)?;
                        let idx = self.expr_as(index, Ty::Num)?;
                        let out = self.dst_or_temp(dst, Ty::Num);
                        self.emit(Opcode::ListGet {
                            dst: out.reg,
                            list: list.reg,
                            idx: idx.reg,
                        });
                        return Some(Operand {
                            ty: Ty::Num,
                            reg: out.reg,
                        });
                    }
                }
                let src = self.expr_as(inner, Ty::Val)?;
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
                    let src = self.expr_as(inner, Ty::Num)?;
                    return Some(Operand {
                        ty: Ty::Num,
                        reg: src.reg,
                    });
                }
                let src = self.expr_as(inner, Ty::Val)?;
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
                UnaryOp::BorrowImm | UnaryOp::BorrowMut => self.expr_to(inner, dst),
                UnaryOp::Not => {
                    let src = self.expr_as(inner, Ty::Bool)?;
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
                UnaryOp::Neg | UnaryOp::BitNot if self.infer(inner) == Ty::Num => {
                    let src = self.expr_as(inner, Ty::Num)?;
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
                    let src = self.expr_as(inner, Ty::Val)?;
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
                    let src = self.expr_as(inner, Ty::Val)?;
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
            } => self.binary(left, op, right, dst),
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
                let mut pending = self.cond_false_jumps(cond)?;
                self.expr_into(then_expr, out)?;
                ends.push(self.emit(Opcode::Jump { target: 0 }));
                for (c, e) in else_if {
                    let here = self.here();
                    for j in pending {
                        self.patch(j, here);
                    }
                    pending = self.cond_false_jumps(c)?;
                    self.expr_into(e, out)?;
                    ends.push(self.emit(Opcode::Jump { target: 0 }));
                }
                let here = self.here();
                for j in pending {
                    self.patch(j, here);
                }
                match else_expr {
                    Some(e) => self.expr_into(e, out)?,
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
            Expr::Call { callee, args, .. } => self.call(callee, args, dst),
            Expr::MethodCall {
                receiver,
                method_name,
                args,
                ..
            } => self.method_call(receiver, method_name, args, dst),
            Expr::Map { .. }
            | Expr::StructLiteral { .. }
            | Expr::EnumConstruct { .. }
            | Expr::FieldAccess { .. } => None,
        }
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

    fn call(&mut self, callee: &Expr, args: &[Expr], dst: Option<Operand>) -> Option<Operand> {
        let Expr::Ident { name, .. } = callee else {
            return None;
        };
        if self.lookup(name).is_some() || MODULE_BUILTINS.contains(&name.as_str()) {
            return None;
        }
        if let Some(builtin) = self.program.builtins.get(name).copied() {
            if let (Some(out), "orodha_rudia", 2) = (dst, name.as_str(), args.len()) {
                if out.ty == Ty::List && self.infer(&args[0]) == Ty::Num {
                    let value = self.expr_as(&args[0], Ty::Num)?.reg;
                    let count = self.expr_as(&args[1], Ty::Num)?.reg;
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
                && self.infer(&args[0]) == Ty::Num
            {
                let src = self.expr_as(&args[0], Ty::Num)?.reg;
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
            let mut regs = Vec::with_capacity(args.len());
            for arg in args {
                regs.push(self.expr_as(arg, Ty::Val)?.reg);
            }
            let out = self.dst_or_temp(dst, Ty::Val);
            self.emit(Opcode::CallBuiltin(Box::new(BuiltinOp {
                builtin,
                args: regs,
                dst: out.reg,
            })));
            return Some(out);
        }
        let (index, params, ret) = {
            let sig = self.program.functions.get(name)?;
            (sig.index, sig.params.clone(), sig.ret)
        };
        if params.len() != args.len() {
            return None;
        }
        let mut operands = Vec::with_capacity(args.len());
        for (arg, ty) in args.iter().zip(params) {
            operands.push(self.expr_as(arg, ty)?);
        }
        let out = match dst {
            Some(d) if d.ty == ret => d,
            _ => self.temp(ret),
        };
        self.emit(Opcode::Call(Box::new(CallOp {
            function: index,
            args: operands,
            dst: out,
        })));
        Some(out)
    }

    fn method_call(
        &mut self,
        receiver: &Expr,
        method: &str,
        args: &[Expr],
        dst: Option<Operand>,
    ) -> Option<Operand> {
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
                    let src = self.expr_as(&args[0], Ty::Num)?.reg;
                    self.emit(Opcode::ListPush {
                        list: list.reg,
                        src,
                    });
                    return self.tupu(dst);
                }
                ("ingiza", 2, Some(list)) => {
                    let idx = self.expr_as(&args[0], Ty::Num)?.reg;
                    let src = self.expr_as(&args[1], Ty::Num)?.reg;
                    self.emit(Opcode::ListSet {
                        list: list.reg,
                        idx,
                        src,
                    });
                    return self.tupu(dst);
                }
                ("ondoa", 1, Some(list)) => {
                    let idx = self.expr_as(&args[0], Ty::Num)?.reg;
                    let out = self.dst_or_temp(dst, Ty::Val);
                    self.emit(Opcode::ListRemoveVal {
                        dst: out.reg,
                        list: list.reg,
                        idx,
                    });
                    return Some(out);
                }
                ("ongeza" | "ingiza" | "ondoa", _, _) => return None,
                _ => {}
            }
        }
        let recv_type = self.type_name(receiver)?;
        // In-place mutation of a generic local.
        let mutating = match method {
            "ongeza" => Some(MutMethod::Push),
            "ingiza" => Some(MutMethod::Insert),
            "ondoa" => Some(MutMethod::Remove),
            "weka_key" => Some(MutMethod::WekaKey),
            _ => None,
        };
        if let Some(kind) = mutating {
            let Expr::Ident { name, .. } = receiver else {
                return None;
            };
            let local = self.lookup(name)?.op;
            let supported = match kind {
                MutMethod::Push | MutMethod::Remove => recv_type.starts_with("Orodha"),
                MutMethod::Insert => {
                    recv_type.starts_with("Orodha") || recv_type.starts_with("Kamusi")
                }
                MutMethod::WekaKey => recv_type.starts_with("Kamusi"),
            };
            if local.ty != Ty::Val || !supported {
                return None;
            }
            let mut regs = Vec::with_capacity(args.len());
            for arg in args {
                regs.push(self.expr_as(arg, Ty::Val)?.reg);
            }
            let out = self.dst_or_temp(dst, Ty::Val);
            self.emit(Opcode::MutMethod(Box::new(MutMethodOp {
                method: kind,
                recv: local.reg,
                args: regs,
                dst: out.reg,
            })));
            return Some(out);
        }
        if method == "urefu"
            && args.is_empty()
            && (recv_type == "Neno" || recv_type.starts_with("Orodha"))
        {
            let src = self.expr_as(receiver, Ty::Val)?.reg;
            let out = self.dst_or_temp(dst, Ty::Num).reg;
            self.emit(Opcode::ValLen { dst: out, src });
            return Some(Operand {
                ty: Ty::Num,
                reg: out,
            });
        }
        let callback = recv_type.starts_with("Orodha") && CALLBACK_METHODS.contains(&method);
        if !callback && !pure_method_supported(&recv_type, method) {
            return None;
        }
        let recv = self.expr_as(receiver, Ty::Val)?.reg;
        let mut regs = Vec::with_capacity(args.len());
        for arg in args {
            regs.push(self.expr_as(arg, Ty::Val)?.reg);
        }
        let out = self.dst_or_temp(dst, Ty::Val);
        self.emit(Opcode::CallMethod(Box::new(MethodOp {
            method: method.to_string(),
            recv,
            args: regs,
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
// Interpreter
// ---------------------------------------------------------------------------------------------

/// Execute the entry function in a bytecode program.
pub fn run_bytecode(program: &BytecodeProgram, args: Vec<String>) -> Result<(), EvalError> {
    let index = program
        .functions
        .iter()
        .position(|f| f.name == program.entry)
        .ok_or_else(|| EvalError::Unknown(format!("kazi '{}' haikupatikana", program.entry)))?;
    let hoja = Value::Orodha(args.into_iter().map(Value::Neno).collect());
    let mut vm = Vm::new(program)?;
    vm.call_values(index, vec![hoja])?;
    Ok(())
}

/// Execute the entry function using an ahead-of-time compiled library built for `program`
/// (see [`crate::aot`]); without one this is [`run_bytecode`].
#[cfg(not(target_arch = "wasm32"))]
pub fn run_bytecode_native(
    program: &BytecodeProgram,
    library: Option<&crate::aot::NativeLibrary>,
    args: Vec<String>,
) -> Result<(), EvalError> {
    let index = program
        .functions
        .iter()
        .position(|f| f.name == program.entry)
        .ok_or_else(|| EvalError::Unknown(format!("kazi '{}' haikupatikana", program.entry)))?;
    let hoja = Value::Orodha(args.into_iter().map(Value::Neno).collect());
    let mut vm = match library {
        Some(lib) => Vm::with_aot(program, lib)?,
        None => Vm::new(program)?,
    };
    vm.call_values(index, vec![hoja])?;
    Ok(())
}

/// Which execution engine runs bytecode, for differential testing of the native tiers.
#[derive(Clone, Copy)]
pub enum Engine<'l> {
    /// The register VM's interpreter only.
    Interpreter,
    /// The Cranelift JIT (with the interpreter for anything it hands back).
    #[cfg(not(target_arch = "wasm32"))]
    Jit,
    /// An LLVM AOT library built for this program.
    #[cfg(not(target_arch = "wasm32"))]
    Aot(&'l crate::aot::NativeLibrary),
    /// Keeps the lifetime used on targets without native tiers.
    #[cfg(target_arch = "wasm32")]
    #[doc(hidden)]
    _Unused(std::marker::PhantomData<&'l ()>),
}

/// Run a named function on a specific engine.
pub fn run_bytecode_function_on(
    engine: Engine<'_>,
    program: &BytecodeProgram,
    name: &str,
    args: Vec<Value>,
) -> Result<Value, EvalError> {
    let index = program
        .functions
        .iter()
        .position(|f| f.name == name)
        .ok_or_else(|| EvalError::UndefinedVar(name.to_string()))?;
    let mut vm = match engine {
        Engine::Interpreter => Vm::new_inner(program, false)?,
        #[cfg(target_arch = "wasm32")]
        Engine::_Unused(_) => Vm::new_inner(program, false)?,
        #[cfg(not(target_arch = "wasm32"))]
        Engine::Jit => {
            let mut vm = Vm::new_inner(program, false)?;
            vm.jit = crate::jit::compile_program(program, &NATIVE_RUNTIME);
            vm
        }
        #[cfg(not(target_arch = "wasm32"))]
        Engine::Aot(lib) => Vm::with_aot(program, lib)?,
    };
    vm.call_values(index, args)
}

/// Execute a named bytecode function. Useful for embedders and focused VM tests; the CLI entry
/// point above keeps the `kuu(hoja)` interface.
pub fn run_bytecode_function(
    program: &BytecodeProgram,
    name: &str,
    args: Vec<Value>,
) -> Result<Value, EvalError> {
    let index = program
        .functions
        .iter()
        .position(|f| f.name == name)
        .ok_or_else(|| EvalError::UndefinedVar(name.to_string()))?;
    let mut vm = Vm::new(program)?;
    vm.call_values(index, args)
}

/// One call's register files. Native code reads `nums` through a raw pointer and `lists`
/// through `jit::list_ptr`/`list_len`, so neither may be resized while the call runs.
#[derive(Default)]
pub(crate) struct Frame {
    pub(crate) nums: Vec<f64>,
    pub(crate) lists: Vec<Vec<f64>>,
    pub(crate) vals: Vec<Value>,
}

pub(crate) enum Flow {
    Next,
    Finish(Ret),
    Fail(EvalError),
}

pub(crate) enum Ret {
    Num(f64),
    List(Vec<f64>),
    Val(Value),
}

struct Vm<'p> {
    program: &'p BytecodeProgram,
    builtins: Vec<BuiltinFn>,
    builtin_index: HashMap<String, usize>,
    pool: Vec<Frame>,
    depth: usize,
    #[cfg(not(target_arch = "wasm32"))]
    jit: Option<crate::jit::Jit>,
    /// Ahead-of-time compiled functions (`pata jenga`'s LLVM library), preferred over the JIT.
    #[cfg(not(target_arch = "wasm32"))]
    aot: Option<&'p crate::aot::NativeLibrary>,
    /// Outcome of an instruction that native code handed to `exec_slow` and that ended the call.
    #[cfg(not(target_arch = "wasm32"))]
    pending: Option<Flow>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) static NATIVE_RUNTIME: crate::native::Runtime = crate::native::Runtime {
    exec: native_exec,
    list_ptr: crate::native::list_ptr,
    list_len: crate::native::list_len,
    list_push: crate::native::list_push,
    list_remove: crate::native::list_remove,
};

/// `exec_slow` entry point for native code: `0` to continue, else a `native::STATUS_*`.
#[cfg(not(target_arch = "wasm32"))]
extern "C" fn native_exec(
    vm: *mut std::ffi::c_void,
    frame: *mut Frame,
    function: u32,
    pc: u32,
) -> u32 {
    // SAFETY: native code only calls this with the `Vm` and `Frame` that `run_native` passed
    // in, both of which outlive the call and are not otherwise borrowed while it runs.
    let vm = unsafe { &mut *(vm as *mut Vm<'static>) };
    let frame = unsafe { &mut *frame };
    let program = vm.program;
    let op = &program.functions[function as usize].code[pc as usize];
    match vm.exec_slow(op, frame) {
        Flow::Next => 0,
        flow @ Flow::Finish(_) => {
            vm.pending = Some(flow);
            crate::native::STATUS_FINISH as u32
        }
        flow @ Flow::Fail(_) => {
            vm.pending = Some(flow);
            crate::native::STATUS_FAIL as u32
        }
    }
}

const MAX_CALL_DEPTH: usize = 10_000;

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

impl<'p> Vm<'p> {
    fn new(program: &'p BytecodeProgram) -> Result<Self, EvalError> {
        Self::new_inner(program, true)
    }

    fn new_inner(program: &'p BytecodeProgram, allow_jit: bool) -> Result<Self, EvalError> {
        #[cfg(target_arch = "wasm32")]
        let _ = allow_jit;
        let names = builtin_names();
        let mut table = builtins();
        let mut list = Vec::with_capacity(names.len());
        let mut builtin_index = HashMap::with_capacity(names.len());
        for (i, name) in names.into_iter().enumerate() {
            let f = table
                .remove(&name)
                .ok_or_else(|| EvalError::Unknown(format!("builtin haipo: {name}")))?;
            list.push(f);
            builtin_index.insert(name, i);
        }
        Ok(Vm {
            program,
            builtins: list,
            builtin_index,
            pool: Vec::new(),
            depth: 0,
            #[cfg(not(target_arch = "wasm32"))]
            aot: None,
            #[cfg(not(target_arch = "wasm32"))]
            jit: if allow_jit && crate::jit::enabled() {
                crate::jit::compile_program(program, &NATIVE_RUNTIME)
            } else {
                None
            },
            #[cfg(not(target_arch = "wasm32"))]
            pending: None,
        })
    }

    /// A VM running `library`'s machine code; the JIT is not needed and not built.
    #[cfg(not(target_arch = "wasm32"))]
    fn with_aot(
        program: &'p BytecodeProgram,
        library: &'p crate::aot::NativeLibrary,
    ) -> Result<Self, EvalError> {
        let mut vm = Self::new_inner(program, false)?;
        vm.aot = Some(library);
        Ok(vm)
    }

    fn frame_for(&mut self, f: &BytecodeFunc) -> Frame {
        let mut frame = self.pool.pop().unwrap_or_default();
        frame.nums.clear();
        frame.nums.resize(f.num_regs as usize, 0.0);
        frame.lists.clear();
        frame.lists.resize_with(f.list_regs as usize, Vec::new);
        frame.vals.clear();
        frame.vals.resize(f.val_regs as usize, Value::Hamna);
        for (reg, n) in &f.num_consts {
            frame.nums[*reg as usize] = *n;
        }
        frame
    }

    fn release(&mut self, frame: Frame) {
        self.pool.push(frame);
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
            Ret::List(l) => Value::Orodha(l.into_iter().map(Value::Namba).collect()),
            Ret::Val(v) => v,
        })
    }

    fn invoke(&mut self, index: usize, frame: Frame) -> Result<Ret, EvalError> {
        self.depth += 1;
        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(EvalError::Unknown("undani mno".into()));
        }
        let result = stacker::maybe_grow(64 * 1024, 2 * 1024 * 1024, || {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(native) = self.aot.map(|lib| lib.funcs[index]) {
                return self.run_native(native, index, frame);
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(native) = self.jit.as_ref().and_then(|j| j.funcs[index]) {
                return self.run_native(native, index, frame);
            }
            self.run(index, frame, 0)
        });
        self.depth -= 1;
        result
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_native(
        &mut self,
        native: crate::native::NativeFn,
        index: usize,
        mut frame: Frame,
    ) -> Result<Ret, EvalError> {
        let nums = frame.nums.as_mut_ptr();
        let vm = self as *mut Vm<'p> as *mut std::ffi::c_void;
        // SAFETY: `native` was compiled from `program.functions[index]` for exactly this frame
        // layout (`frame_for` sized every register file), and the frame is not resized while
        // the call runs.
        let status = unsafe { native(&NATIVE_RUNTIME, vm, &mut frame, nums) };
        let pc = (status & 0xffff_ffff) as usize;
        if status >> 32 == crate::native::STATUS_DEOPT {
            return self.run(index, frame, pc);
        }
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
        let Value::Orodha(items) = recv else {
            return Err(EvalError::Unknown(format!(
                "bytecode method haijaungwa mkono: {method}"
            )));
        };
        let cb = args
            .first()
            .and_then(value::as_string)
            .ok_or_else(|| EvalError::TypeErr("njia inahitaji jina la kazi".into()))?;
        let truthy = |v: &Value| matches!(v, Value::Ukweli(true));
        match method {
            "ramani" => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.callback(&cb, vec![item])?);
                }
                Ok(Value::Orodha(out))
            }
            "chuja" => {
                let mut out = Vec::new();
                for item in items {
                    if truthy(&self.callback(&cb, vec![item.clone()])?) {
                        out.push(item);
                    }
                }
                Ok(Value::Orodha(out))
            }
            "hesabu" => {
                let mut count = 0.0;
                for item in items {
                    if truthy(&self.callback(&cb, vec![item])?) {
                        count += 1.0;
                    }
                }
                Ok(Value::Namba(count))
            }
            "chunguza" => {
                for item in items {
                    if truthy(&self.callback(&cb, vec![item])?) {
                        return Ok(Value::Ukweli(true));
                    }
                }
                Ok(Value::Ukweli(false))
            }
            "kila_na_fahirisi" => {
                for (i, item) in items.into_iter().enumerate() {
                    self.callback(&cb, vec![Value::Namba(i as f64), item])?;
                }
                Ok(Value::Tupu)
            }
            _ => Err(EvalError::Unknown(format!(
                "bytecode method haijaungwa mkono: {method}"
            ))),
        }
    }

    fn run(&mut self, index: usize, mut frame: Frame, start: usize) -> Result<Ret, EvalError> {
        let program = self.program;
        let code = &program.functions[index].code;
        let mut pc = start;
        macro_rules! finish {
            ($ret:expr) => {{
                let ret = $ret;
                self.release(frame);
                return Ok(ret);
            }};
        }
        macro_rules! fail {
            ($err:expr) => {{
                let err = $err;
                self.release(frame);
                return Err(err);
            }};
        }
        loop {
            let Some(op) = code.get(pc) else {
                finish!(Ret::Val(Value::Tupu));
            };
            pc += 1;
            let n = &mut frame.nums;
            match op {
                Opcode::Mov { dst, src } => n[*dst as usize] = n[*src as usize],
                Opcode::Add { dst, a, b } => n[*dst as usize] = n[*a as usize] + n[*b as usize],
                Opcode::Sub { dst, a, b } => n[*dst as usize] = n[*a as usize] - n[*b as usize],
                Opcode::Mul { dst, a, b } => n[*dst as usize] = n[*a as usize] * n[*b as usize],
                Opcode::Div { dst, a, b } => n[*dst as usize] = n[*a as usize] / n[*b as usize],
                Opcode::Rem { dst, a, b } => n[*dst as usize] = n[*a as usize] % n[*b as usize],
                Opcode::Pow { dst, a, b } => n[*dst as usize] = n[*a as usize].powf(n[*b as usize]),
                Opcode::BitAnd { dst, a, b } => {
                    n[*dst as usize] = ((n[*a as usize] as i64) & (n[*b as usize] as i64)) as f64
                }
                Opcode::BitOr { dst, a, b } => {
                    n[*dst as usize] = ((n[*a as usize] as i64) | (n[*b as usize] as i64)) as f64
                }
                Opcode::BitXor { dst, a, b } => {
                    n[*dst as usize] = ((n[*a as usize] as i64) ^ (n[*b as usize] as i64)) as f64
                }
                Opcode::Shl { dst, a, b } => {
                    n[*dst as usize] =
                        (n[*a as usize] as i64).wrapping_shl(shift_amount(n[*b as usize])) as f64
                }
                Opcode::Shr { dst, a, b } => {
                    n[*dst as usize] =
                        (n[*a as usize] as i64).wrapping_shr(shift_amount(n[*b as usize])) as f64
                }
                Opcode::Neg { dst, src } => n[*dst as usize] = -n[*src as usize],
                Opcode::BitNot { dst, src } => n[*dst as usize] = !(n[*src as usize] as i64) as f64,
                Opcode::Not { dst, src } => n[*dst as usize] = flag(n[*src as usize] == 0.0),
                Opcode::Floor { dst, src } => n[*dst as usize] = n[*src as usize].floor(),
                Opcode::Ceil { dst, src } => n[*dst as usize] = n[*src as usize].ceil(),
                Opcode::Trunc { dst, src } => n[*dst as usize] = (n[*src as usize] as i64) as f64,
                Opcode::Cmp { op, dst, a, b } => {
                    n[*dst as usize] = flag(compare(*op, n[*a as usize], n[*b as usize]))
                }
                Opcode::Jump { target } => pc = *target as usize,
                Opcode::JumpIfFalse { cond, target } => {
                    if n[*cond as usize] == 0.0 {
                        pc = *target as usize;
                    }
                }
                Opcode::JumpIfTrue { cond, target } => {
                    if n[*cond as usize] != 0.0 {
                        pc = *target as usize;
                    }
                }
                Opcode::JumpIfNot { op, a, b, target } => {
                    if !compare(*op, n[*a as usize], n[*b as usize]) {
                        pc = *target as usize;
                    }
                }
                Opcode::ForStep { ctr, end, target } => {
                    let next = n[*ctr as usize] + 1.0;
                    n[*ctr as usize] = next;
                    if next < n[*end as usize] {
                        pc = *target as usize;
                    }
                }
                Opcode::ListGet { dst, list, idx } => {
                    let i = to_index(n[*idx as usize]);
                    let l = &frame.lists[*list as usize];
                    match l.get(i) {
                        Some(v) => frame.nums[*dst as usize] = *v,
                        None => {
                            let err = methods::out_of_bounds(i, l.len());
                            finish!(Ret::Val(err));
                        }
                    }
                }
                Opcode::ListSet { list, idx, src } => {
                    let i = to_index(n[*idx as usize]);
                    let v = n[*src as usize];
                    match frame.lists[*list as usize].get_mut(i) {
                        Some(slot) => *slot = v,
                        None => fail!(type_err("ingiza: index nje ya mipaka")),
                    }
                }
                Opcode::ListLen { dst, list } => {
                    n[*dst as usize] = frame.lists[*list as usize].len() as f64
                }
                Opcode::Return { src } => {
                    let ret = match src.ty {
                        Ty::Num | Ty::Bool => Ret::Num(frame.nums[src.reg as usize]),
                        Ty::List => Ret::List(std::mem::take(&mut frame.lists[src.reg as usize])),
                        Ty::Val => Ret::Val(std::mem::replace(
                            &mut frame.vals[src.reg as usize],
                            Value::Hamna,
                        )),
                    };
                    finish!(ret);
                }
                Opcode::ReturnTupu => finish!(Ret::Val(Value::Tupu)),
                other => match self.exec_slow(other, &mut frame) {
                    Flow::Next => {}
                    Flow::Finish(ret) => finish!(ret),
                    Flow::Fail(err) => fail!(err),
                },
            }
        }
    }

    /// Execute one non-control instruction. Shared by the interpreter (for everything off the
    /// numeric fast path) and by JIT-compiled code (for instructions it does not compile).
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
        let n = &mut frame.nums;
        match op {
            Opcode::Mov { dst, src } => n[*dst as usize] = n[*src as usize],
            Opcode::Add { dst, a, b } => n[*dst as usize] = n[*a as usize] + n[*b as usize],
            Opcode::Sub { dst, a, b } => n[*dst as usize] = n[*a as usize] - n[*b as usize],
            Opcode::Mul { dst, a, b } => n[*dst as usize] = n[*a as usize] * n[*b as usize],
            Opcode::Div { dst, a, b } => n[*dst as usize] = n[*a as usize] / n[*b as usize],
            Opcode::Rem { dst, a, b } => n[*dst as usize] = n[*a as usize] % n[*b as usize],
            Opcode::Pow { dst, a, b } => n[*dst as usize] = n[*a as usize].powf(n[*b as usize]),
            Opcode::Floor { dst, src } => n[*dst as usize] = n[*src as usize].floor(),
            Opcode::Ceil { dst, src } => n[*dst as usize] = n[*src as usize].ceil(),
            Opcode::ListRepeat { dst, value, count } => {
                let count = n[*count as usize];
                if !(count.is_finite() && count >= 0.0 && count.fract() == 0.0) {
                    fail!(type_err(
                        "orodha_rudia inahitaji idadi ya Namba kamili isiyo hasi"
                    ));
                }
                let v = n[*value as usize];
                frame.lists[*dst as usize] = vec![v; count as usize];
            }
            Opcode::MakeNumList { dst, items } => {
                let list: Vec<f64> = items.iter().map(|r| n[*r as usize]).collect();
                frame.lists[*dst as usize] = list;
            }
            Opcode::ListGet { dst, list, idx } => {
                let i = to_index(n[*idx as usize]);
                let l = &frame.lists[*list as usize];
                match l.get(i) {
                    Some(v) => frame.nums[*dst as usize] = *v,
                    None => {
                        let err = methods::out_of_bounds(i, l.len());
                        finish!(Ret::Val(err));
                    }
                }
            }
            Opcode::ListGetTokeo { dst, list, idx } => {
                let i = to_index(n[*idx as usize]);
                let l = &frame.lists[*list as usize];
                let v = match l.get(i) {
                    Some(v) => Value::Tokeo(Ok(Box::new(Value::Namba(*v)))),
                    None => methods::out_of_bounds(i, l.len()),
                };
                frame.vals[*dst as usize] = v;
            }
            Opcode::ListSet { list, idx, src } => {
                let i = to_index(n[*idx as usize]);
                let v = n[*src as usize];
                match frame.lists[*list as usize].get_mut(i) {
                    Some(slot) => *slot = v,
                    None => fail!(type_err("ingiza: index nje ya mipaka")),
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
                let v = Value::Orodha(
                    frame.lists[*src as usize]
                        .iter()
                        .map(|n| Value::Namba(*n))
                        .collect(),
                );
                frame.vals[*dst as usize] = v;
            }
            Opcode::ConstVal { dst, k } => {
                frame.vals[*dst as usize] = program
                    .constants
                    .get(*k as usize)
                    .map(StoredConstant::to_value)
                    .unwrap_or(Value::Hamna)
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
                frame.nums[*dst as usize] =
                    flag(matches!(frame.vals[*src as usize], Value::Ukweli(true)))
            }
            Opcode::ValBinary { op, dst, a, b } => {
                match value_binary(op, &frame.vals[*a as usize], &frame.vals[*b as usize]) {
                    Ok(v) => frame.vals[*dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::ValUnary { op, dst, src } => {
                let x = match value::as_f64(&frame.vals[*src as usize]) {
                    Some(x) => x,
                    None => fail!(type_err(match op {
                        UnaryCode::Neg => "- inahitaji Namba",
                        UnaryCode::BitNot => "siyo_biti inahitaji Namba",
                    })),
                };
                frame.vals[*dst as usize] = Value::Namba(match op {
                    UnaryCode::Neg => -x,
                    UnaryCode::BitNot => !(x as i64) as f64,
                });
            }
            Opcode::ValIndex { dst, base, idx } => {
                match methods::index_value(&frame.vals[*base as usize], &frame.vals[*idx as usize])
                {
                    Ok(v) => frame.vals[*dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::ValLen { dst, src } => {
                let len = match &frame.vals[*src as usize] {
                    Value::Orodha(items) => items.len(),
                    Value::Neno(s) => {
                        unicode_segmentation::UnicodeSegmentation::graphemes(s.as_str(), true)
                            .count()
                    }
                    _ => fail!(type_err("urefu inahitaji Orodha au Neno")),
                };
                frame.nums[*dst as usize] = len as f64;
            }
            Opcode::Unwrap { dst, src } => {
                let out = match &frame.vals[*src as usize] {
                    Value::Tokeo(Ok(inner)) | Value::Chaguo(Some(inner)) => Ok((**inner).clone()),
                    err @ Value::Tokeo(Err(_)) => Err(Ok(err.clone())),
                    Value::Chaguo(None) => Err(Err(EvalError::Unknown("? Chaguo Hamna".into()))),
                    _ => Err(Err(type_err("? inahitaji Tokeo/Chaguo"))),
                };
                match out {
                    Ok(v) => frame.vals[*dst as usize] = v,
                    Err(Ok(err)) => finish!(Ret::Val(err)),
                    Err(Err(e)) => fail!(e),
                }
            }
            Opcode::Jaribu { dst, src } => {
                let out = match &frame.vals[*src as usize] {
                    Value::Tokeo(Ok(inner)) | Value::Chaguo(Some(inner)) => (**inner).clone(),
                    Value::Tokeo(Err(e)) => {
                        fail!(EvalError::Unknown(format!("KOSA: {e:?}")))
                    }
                    Value::Chaguo(None) => fail!(EvalError::Unknown("Chaguo: Hamna".into())),
                    _ => fail!(type_err("jaribu inahitaji Tokeo/Chaguo")),
                };
                frame.vals[*dst as usize] = out;
            }
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
                frame.vals[*dst as usize] = Value::Orodha(list);
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
                            Value::Orodha(l.into_iter().map(Value::Namba).collect())
                    }
                    _ => fail!(type_err("aina ya thamani ya kurudi si sahihi")),
                }
            }
            Opcode::CallBuiltin(call) => {
                let args: Vec<Value> = call
                    .args
                    .iter()
                    .map(|r| frame.vals[*r as usize].clone())
                    .collect();
                match (self.builtins[call.builtin as usize])(&args) {
                    Ok(v) => frame.vals[call.dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::CallMethod(call) => {
                let recv = frame.vals[call.recv as usize].clone();
                let args: Vec<Value> = call
                    .args
                    .iter()
                    .map(|r| frame.vals[*r as usize].clone())
                    .collect();
                match self.call_method(recv, &call.method, args) {
                    Ok(v) => frame.vals[call.dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::MutMethod(call) => {
                let args: Vec<Value> = call
                    .args
                    .iter()
                    .map(|r| frame.vals[*r as usize].clone())
                    .collect();
                match mut_method(&mut frame.vals[call.recv as usize], call.method, args) {
                    Ok(v) => frame.vals[call.dst as usize] = v,
                    Err(e) => fail!(e),
                }
            }
            Opcode::Jump { .. }
            | Opcode::JumpIfFalse { .. }
            | Opcode::JumpIfTrue { .. }
            | Opcode::JumpIfNot { .. }
            | Opcode::ForStep { .. }
            | Opcode::Return { .. }
            | Opcode::ReturnTupu
            | Opcode::Neg { .. }
            | Opcode::BitNot { .. }
            | Opcode::Not { .. }
            | Opcode::Trunc { .. }
            | Opcode::Cmp { .. }
            | Opcode::BitAnd { .. }
            | Opcode::BitOr { .. }
            | Opcode::BitXor { .. }
            | Opcode::Shl { .. }
            | Opcode::Shr { .. } => {
                fail!(EvalError::Unknown(
                    "amri ya udhibiti nje ya mzunguko".into()
                ))
            }
        }
        Flow::Next
    }
}

fn copy_operand(from: &Frame, src: Operand, to: &mut Frame, dst: Operand) {
    match dst.ty {
        Ty::Num | Ty::Bool => to.nums[dst.reg as usize] = from.nums[src.reg as usize],
        Ty::List => to.lists[dst.reg as usize] = from.lists[src.reg as usize].clone(),
        Ty::Val => to.vals[dst.reg as usize] = from.vals[src.reg as usize].clone(),
    }
}

fn list_from_value(v: &Value) -> Result<Vec<f64>, EvalError> {
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

fn mut_method(recv: &mut Value, method: MutMethod, args: Vec<Value>) -> Result<Value, EvalError> {
    let mut args = args.into_iter();
    match (recv, method) {
        (Value::Orodha(items), MutMethod::Push) => {
            items.push(args.next().unwrap_or(Value::Hamna));
            Ok(Value::Tupu)
        }
        (Value::Orodha(items), MutMethod::Insert) => {
            let idx = args
                .next()
                .as_ref()
                .and_then(value::as_f64)
                .map(|n| n as usize)
                .ok_or_else(|| type_err("ingiza inahitaji index na thamani"))?;
            let v = args.next().unwrap_or(Value::Hamna);
            match items.get_mut(idx) {
                Some(slot) => {
                    *slot = v;
                    Ok(Value::Tupu)
                }
                None => Err(type_err("ingiza: index nje ya mipaka")),
            }
        }
        (Value::Orodha(items), MutMethod::Remove) => {
            let idx = args
                .next()
                .as_ref()
                .and_then(value::as_f64)
                .map(|n| n as usize)
                .unwrap_or(0);
            Ok(if idx < items.len() {
                Value::Chaguo(Some(Box::new(items.remove(idx))))
            } else {
                Value::Chaguo(None)
            })
        }
        (Value::Kamusi(map), MutMethod::Insert | MutMethod::WekaKey) => {
            let key = args
                .next()
                .ok_or_else(|| type_err("ingiza inahitaji ufunguo na thamani"))?;
            let v = args.next().unwrap_or(Value::Hamna);
            map.insert(MapKey::try_from_value(&key)?, v);
            Ok(Value::Tupu)
        }
        _ => Err(type_err("njia ya kubadilisha haijaungwa mkono")),
    }
}

fn value_binary(op: &BinaryCode, left: &Value, right: &Value) -> Result<Value, EvalError> {
    let numbers = || -> Result<(f64, f64), EvalError> {
        Ok((
            value::as_f64(left).ok_or_else(|| type_err("operesheni inahitaji Namba"))?,
            value::as_f64(right).ok_or_else(|| type_err("operesheni inahitaji Namba"))?,
        ))
    };
    Ok(match op {
        BinaryCode::Add => match (value::as_string(left), value::as_string(right)) {
            (Some(a), Some(b)) => Value::Neno(format!("{a}{b}")),
            _ => {
                let (a, b) = numbers()?;
                Value::Namba(a + b)
            }
        },
        BinaryCode::Sub => {
            let (a, b) = numbers()?;
            Value::Namba(a - b)
        }
        BinaryCode::Mul => {
            let (a, b) = numbers()?;
            Value::Namba(a * b)
        }
        BinaryCode::Div => {
            let (a, b) = numbers()?;
            Value::Namba(a / b)
        }
        BinaryCode::Rem => {
            let (a, b) = numbers()?;
            Value::Namba(a % b)
        }
        BinaryCode::Pow => {
            let (a, b) = numbers()?;
            Value::Namba(a.powf(b))
        }
        BinaryCode::Eq => Value::Ukweli(left == right),
        BinaryCode::Ne => Value::Ukweli(left != right),
        BinaryCode::Gt | BinaryCode::Lt | BinaryCode::Ge | BinaryCode::Le => {
            if let (Some(a), Some(b)) = (value::as_string(left), value::as_string(right)) {
                let ord = a.cmp(&b);
                return Ok(Value::Ukweli(match op {
                    BinaryCode::Gt => ord.is_gt(),
                    BinaryCode::Lt => ord.is_lt(),
                    BinaryCode::Ge => ord.is_ge(),
                    _ => ord.is_le(),
                }));
            }
            let (a, b) = numbers()?;
            Value::Ukweli(match op {
                BinaryCode::Gt => a > b,
                BinaryCode::Lt => a < b,
                BinaryCode::Ge => a >= b,
                _ => a <= b,
            })
        }
        BinaryCode::BitAnd => {
            let (a, b) = numbers()?;
            Value::Namba((a as i64 & b as i64) as f64)
        }
        BinaryCode::BitXor => {
            let (a, b) = numbers()?;
            Value::Namba((a as i64 ^ b as i64) as f64)
        }
        BinaryCode::BitOr => {
            let (a, b) = numbers()?;
            Value::Namba((a as i64 | b as i64) as f64)
        }
        BinaryCode::Shl => {
            let (a, b) = numbers()?;
            Value::Namba((a as i64).wrapping_shl(shift_amount(b)) as f64)
        }
        BinaryCode::Shr => {
            let (a, b) = numbers()?;
            Value::Namba((a as i64).wrapping_shr(shift_amount(b)) as f64)
        }
    })
}
