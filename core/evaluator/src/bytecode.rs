//! The compact ASB bytecode compiler and stack VM.
//!
//! This is deliberately a conservative lowering pass.  Programs containing syntax which is not
//! represented by this ISA return `None` from [`compile_module`] and continue to use the serialized
//! AST artifact.  That keeps the artifact format backwards compatible while allowing the hot,
//! data-oriented Sudoku subset to run without the tree-walk evaluator.

use crate::builtins::{builtin_names, builtins};
use crate::value::{self, EvalError, Value};
use asili_parser::{AssignOp, BinaryOp, Block, Expr, ForMode, Function, Module, Stmt, UnaryOp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeFunc {
    pub name: String,
    pub arity: u32,
    pub code: Vec<Opcode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Opcode {
    Const(u32),
    Return,
    /// Legacy one-argument builtin call retained for hand-authored ASB programs.
    CallBuiltin(u32),
    LoadLocal(u32),
    StoreLocal(u32),
    Nop,
    Pop,
    Jump(u32),
    JumpIfFalse(u32),
    Binary(BinaryCode),
    Unary(UnaryCode),
    MakeList(u32),
    Index,
    IndexLocal(u32),
    ListLen(u32),
    Unwrap,
    Cast(String),
    Call {
        function: u32,
        arity: u32,
    },
    CallBuiltinN {
        builtin: u32,
        arity: u32,
    },
    CallMethod {
        method: String,
        arity: u32,
    },
    ListPush(u32),
    ListSet(u32),
    ListRemove(u32),
    /// Push a numeric constant without materialising a generic `Value`.
    ConstNumber(u32),
    /// Load a numeric local into the unboxed numeric stack.
    LoadNumber(u32),
    /// Store a numeric stack value without boxing it in the local frame.
    StoreNumber(u32),
    /// Numeric arithmetic/comparison.  The operands remain unboxed for arithmetic.
    NumericBinary(BinaryCode),
    NumericUnary(UnaryCode),
    NumericBuiltin(NumericBuiltinCode),
    /// Numeric-list read; the result is a specialised result value until `UnwrapNumber`.
    IndexNumberLocal(u32),
    IndexNumberLocalFromSlot {
        list: u32,
        index: u32,
    },
    UnwrapNumber,
    ListPushNumber(u32),
    ListSetNumber(u32),
    StoreNumericList(u32),
    /// Fused loop increment used by range loops and other counted hot paths.
    IncrementNumberLocal {
        slot: u32,
        by: u32,
    },
    /// Fused numeric list read followed by addition (common in flat-array loops).
    IndexAddNumberLocal {
        slot: u32,
        index_slot: u32,
        add: u32,
    },
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
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryCode {
    Neg,
    Not,
    BitNot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumericBuiltinCode {
    Floor,
    ShiftLeft,
    ShiftRight,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BytecodeProgram {
    #[serde(default)]
    pub constants: Vec<StoredConstant>,
    pub functions: Vec<BytecodeFunc>,
    pub entry: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StoredConstant {
    Neno(String),
    Namba(f64),
    Tupu,
    Ukweli(bool),
    Herufi(char),
}

impl BytecodeProgram {
    pub fn find_function(&self, name: &str) -> Option<&BytecodeFunc> {
        self.functions.iter().find(|f| f.name == name)
    }

    fn get_constant(&self, idx: u32) -> Value {
        self.constants
            .get(idx as usize)
            .map(|c| match c {
                StoredConstant::Neno(s) => Value::Neno(s.clone()),
                StoredConstant::Namba(n) => Value::Namba(*n),
                StoredConstant::Ukweli(b) => Value::Ukweli(*b),
                StoredConstant::Herufi(c) => Value::Herufi(*c),
                StoredConstant::Tupu => Value::Tupu,
            })
            .unwrap_or(Value::Tupu)
    }
}

/// Lower the supported, performance-sensitive subset.  `None` means that the caller must emit
/// the normal serialized AST artifact instead.
pub fn compile_module(module: &Module) -> Option<BytecodeProgram> {
    let names: HashMap<String, u32> = module
        .functions
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.clone(), i as u32))
        .collect();
    let mut compiler = Compiler {
        constants: Vec::new(),
        functions: Vec::new(),
        function_names: names,
        numeric_functions: module
            .functions
            .iter()
            .filter(|f| f.return_type.name == "Namba")
            .map(|f| f.name.clone())
            .collect(),
        module_constants: HashMap::new(),
    };
    for constant in &module.constants {
        let index = compiler.literal(&constant.value)?;
        compiler
            .module_constants
            .insert(constant.name.clone(), index);
    }
    for function in &module.functions {
        let compiled = compiler.compile_function(function)?;
        compiler.functions.push(compiled);
    }
    Some(BytecodeProgram {
        constants: compiler.constants,
        functions: compiler.functions,
        entry: "kuu".to_string(),
    })
}

struct Compiler {
    constants: Vec<StoredConstant>,
    functions: Vec<BytecodeFunc>,
    function_names: HashMap<String, u32>,
    numeric_functions: std::collections::HashSet<String>,
    module_constants: HashMap<String, u32>,
}

fn is_numeric_list_type(name: &str) -> bool {
    name.replace(' ', "") == "Orodha<Namba>"
}

fn numeric_builtin(name: &str) -> bool {
    matches!(
        name,
        "sakafu"
            | "dari"
            | "mzizi"
            | "nguvu"
            | "sogeza_kushoto"
            | "sogeza_kulia"
            | "na_biti"
            | "au_biti"
            | "xor_biti"
    )
}

fn numeric_builtin_opcode(name: &str, arity: usize) -> Option<NumericBuiltinCode> {
    match (name, arity) {
        ("sakafu", 1) => Some(NumericBuiltinCode::Floor),
        ("sogeza_kushoto", 2) => Some(NumericBuiltinCode::ShiftLeft),
        ("sogeza_kulia", 2) => Some(NumericBuiltinCode::ShiftRight),
        ("na_biti", 2) => Some(NumericBuiltinCode::BitAnd),
        ("au_biti", 2) => Some(NumericBuiltinCode::BitOr),
        ("xor_biti", 2) => Some(NumericBuiltinCode::BitXor),
        _ => None,
    }
}

impl Compiler {
    fn literal(&mut self, expr: &Expr) -> Option<u32> {
        let constant = match expr {
            Expr::Number(s) => StoredConstant::Namba(value::parse_number(s)),
            Expr::String(s) => StoredConstant::Neno(s.clone()),
            Expr::Bool(b) => StoredConstant::Ukweli(*b),
            Expr::Char(c) => StoredConstant::Herufi(*c),
            Expr::Hamna => return None,
            Expr::Group(e) => return self.literal(e),
            _ => return None,
        };
        let index = self.constants.len() as u32;
        self.constants.push(constant);
        Some(index)
    }

    fn compile_function(&mut self, function: &Function) -> Option<BytecodeFunc> {
        let mut f = FunctionCompiler {
            parent: self,
            locals: HashMap::new(),
            numeric_locals: std::collections::HashSet::new(),
            numeric_lists: std::collections::HashSet::new(),
            next_local: 0,
            code: Vec::new(),
            loops: Vec::new(),
        };
        for param in &function.params {
            let slot = f.slot(&param.name);
            if param.ty.name == "Namba" {
                f.numeric_locals.insert(slot);
            } else if is_numeric_list_type(&param.ty.name) {
                f.numeric_lists.insert(slot);
            }
        }
        f.block(&function.body)?;
        f.code.push(Opcode::Return);
        Some(BytecodeFunc {
            name: function.name.clone(),
            arity: function.params.len() as u32,
            code: f.code,
        })
    }
}

struct LoopState {
    continue_target: u32,
    breaks: Vec<usize>,
}

struct FunctionCompiler<'a> {
    parent: &'a mut Compiler,
    locals: HashMap<String, u32>,
    numeric_locals: std::collections::HashSet<u32>,
    numeric_lists: std::collections::HashSet<u32>,
    next_local: u32,
    code: Vec<Opcode>,
    loops: Vec<LoopState>,
}

impl FunctionCompiler<'_> {
    fn slot(&mut self, name: &str) -> u32 {
        if let Some(slot) = self.locals.get(name) {
            return *slot;
        }
        let slot = self.next_local;
        self.next_local += 1;
        self.locals.insert(name.to_string(), slot);
        slot
    }

    fn existing_slot(&self, name: &str) -> Option<u32> {
        self.locals.get(name).copied()
    }

    fn emit(&mut self, op: Opcode) -> usize {
        let at = self.code.len();
        self.code.push(op);
        at
    }

    fn patch(&mut self, at: usize, target: usize) {
        match &mut self.code[at] {
            Opcode::Jump(n) | Opcode::JumpIfFalse(n) => *n = target as u32,
            _ => {}
        }
    }

    fn block(&mut self, block: &Block) -> Option<()> {
        for stmt in &block.statements {
            self.stmt(stmt)?;
        }
        Some(())
    }

    fn stmt(&mut self, stmt: &Stmt) -> Option<()> {
        match stmt {
            Stmt::Let {
                name, ty, value, ..
            } => {
                let slot = self.slot(name);
                let numeric =
                    ty.as_ref().is_some_and(|t| t.name == "Namba") || self.expr_numeric(value);
                let numeric_list = ty.as_ref().is_some_and(|t| is_numeric_list_type(&t.name));
                self.expr(value)?;
                if numeric {
                    self.numeric_locals.insert(slot);
                    self.emit(Opcode::StoreNumber(slot));
                } else if numeric_list {
                    self.emit(Opcode::StoreNumericList(slot));
                } else {
                    self.emit(Opcode::StoreLocal(slot));
                }
                if numeric_list {
                    self.numeric_lists.insert(slot);
                }
            }
            Stmt::Assign {
                name, op, value, ..
            } => {
                let slot = self.existing_slot(name)?;
                let numeric = self.numeric_locals.contains(&slot);
                if !matches!(op, AssignOp::Assign) {
                    if numeric {
                        self.emit(Opcode::LoadNumber(slot));
                    } else {
                        self.emit(Opcode::LoadLocal(slot));
                    }
                    self.expr(value)?;
                    let binary = match op {
                        AssignOp::AddAssign => BinaryCode::Add,
                        AssignOp::SubAssign => BinaryCode::Sub,
                        AssignOp::MulAssign => BinaryCode::Mul,
                        AssignOp::DivAssign => BinaryCode::Div,
                        AssignOp::Assign => unreachable!(),
                    };
                    self.emit(if numeric {
                        Opcode::NumericBinary(binary)
                    } else {
                        Opcode::Binary(binary)
                    });
                } else {
                    self.expr(value)?;
                }
                self.emit(if numeric {
                    Opcode::StoreNumber(slot)
                } else {
                    Opcode::StoreLocal(slot)
                });
            }
            Stmt::Expr { expr, .. } => {
                self.expr(expr)?;
                self.emit(Opcode::Pop);
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.expr(value)?;
                } else {
                    let empty = self.parent.literal(&Expr::String(String::new()))?;
                    self.emit(Opcode::Const(empty));
                    self.emit(Opcode::Pop);
                    self.emit(Opcode::Nop);
                }
                self.emit(Opcode::Return);
            }
            Stmt::If {
                cond,
                then_block,
                else_if,
                else_block,
                ..
            } => {
                self.expr(cond)?;
                let false_jump = self.emit(Opcode::JumpIfFalse(u32::MAX));
                self.block(then_block)?;
                let mut end_jumps = vec![self.emit(Opcode::Jump(u32::MAX))];
                self.patch(false_jump, self.code.len());
                for (condition, block) in else_if {
                    self.expr(condition)?;
                    let next_false = self.emit(Opcode::JumpIfFalse(u32::MAX));
                    self.block(block)?;
                    end_jumps.push(self.emit(Opcode::Jump(u32::MAX)));
                    self.patch(next_false, self.code.len());
                }
                if let Some(block) = else_block {
                    self.block(block)?;
                }
                let end = self.code.len();
                for jump in end_jumps {
                    self.patch(jump, end);
                }
            }
            Stmt::While { cond, body, .. } => {
                let condition = self.code.len();
                self.expr(cond)?;
                let exit = self.emit(Opcode::JumpIfFalse(u32::MAX));
                self.loops.push(LoopState {
                    continue_target: condition as u32,
                    breaks: Vec::new(),
                });
                self.block(body)?;
                self.emit(Opcode::Jump(condition as u32));
                let end = self.code.len();
                self.patch(exit, end);
                let loop_state = self.loops.pop()?;
                for jump in loop_state.breaks {
                    self.patch(jump, end);
                }
            }
            Stmt::For {
                var, mode, body, ..
            } => self.for_loop(var, mode, body)?,
            Stmt::Break { .. } => {
                let jump = self.emit(Opcode::Jump(u32::MAX));
                self.loops.last_mut()?.breaks.push(jump);
            }
            Stmt::Continue { .. } => {
                let target = self.loops.last()?.continue_target;
                self.emit(Opcode::Jump(target));
            }
            // These constructs can fall back to the AST evaluator.
            Stmt::Match { .. } | Stmt::Drop { .. } | Stmt::LetPattern { .. } => return None,
        }
        Some(())
    }

    fn for_loop(&mut self, var: &str, mode: &ForMode, body: &Block) -> Option<()> {
        let var_slot = self.slot(var);
        match mode {
            ForMode::Range { start, end } => {
                let end_slot = self.slot(&format!("__asb_end_{}", self.next_local));
                self.numeric_locals.insert(var_slot);
                self.numeric_locals.insert(end_slot);
                self.expr(start)?;
                self.emit(Opcode::StoreNumber(var_slot));
                self.expr(end)?;
                self.emit(Opcode::StoreNumber(end_slot));
                let condition = self.code.len();
                self.emit(Opcode::LoadNumber(var_slot));
                self.emit(Opcode::LoadNumber(end_slot));
                self.emit(Opcode::NumericBinary(BinaryCode::Lt));
                let exit = self.emit(Opcode::JumpIfFalse(u32::MAX));
                self.loops.push(LoopState {
                    continue_target: 0,
                    breaks: Vec::new(),
                });
                self.block(body)?;
                let increment = self.code.len();
                self.loops.last_mut()?.continue_target = increment as u32;
                self.emit(Opcode::IncrementNumberLocal {
                    slot: var_slot,
                    by: 1,
                });
                self.emit(Opcode::Jump(condition as u32));
                let end_pos = self.code.len();
                self.patch(exit, end_pos);
                let state = self.loops.pop()?;
                for jump in state.breaks {
                    self.patch(jump, end_pos);
                }
            }
            ForMode::InExpr(expr) => {
                let collection = self.slot(&format!("__asb_iter_{}", self.next_local));
                let index = self.slot(&format!("__asb_index_{}", self.next_local));
                self.numeric_locals.insert(index);
                self.expr(expr)?;
                self.emit(Opcode::StoreLocal(collection));
                let zero = self.parent.literal(&Expr::Number("0.0".into()))?;
                self.emit(Opcode::ConstNumber(zero));
                self.emit(Opcode::StoreNumber(index));
                let condition = self.code.len();
                self.emit(Opcode::LoadNumber(index));
                self.emit(Opcode::LoadLocal(collection));
                self.emit(Opcode::CallMethod {
                    method: "urefu".into(),
                    arity: 0,
                });
                self.emit(Opcode::Binary(BinaryCode::Lt));
                let exit = self.emit(Opcode::JumpIfFalse(u32::MAX));
                self.emit(Opcode::LoadLocal(collection));
                self.emit(Opcode::LoadLocal(index));
                self.emit(Opcode::Index);
                self.emit(Opcode::Unwrap);
                self.emit(Opcode::StoreLocal(var_slot));
                self.loops.push(LoopState {
                    continue_target: 0,
                    breaks: Vec::new(),
                });
                self.block(body)?;
                let increment = self.code.len();
                self.loops.last_mut()?.continue_target = increment as u32;
                self.emit(Opcode::IncrementNumberLocal { slot: index, by: 1 });
                self.emit(Opcode::Jump(condition as u32));
                let end_pos = self.code.len();
                self.patch(exit, end_pos);
                let state = self.loops.pop()?;
                for jump in state.breaks {
                    self.patch(jump, end_pos);
                }
            }
        }
        Some(())
    }

    fn expr_numeric(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Number(_) => true,
            Expr::Ident { name, .. } => self
                .existing_slot(name)
                .is_some_and(|slot| self.numeric_locals.contains(&slot)),
            Expr::Group(e) => self.expr_numeric(e),
            Expr::Unary { op, expr, .. } => {
                matches!(op, UnaryOp::Neg | UnaryOp::BitNot) && self.expr_numeric(expr)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                !matches!(
                    op,
                    BinaryOp::Eq
                        | BinaryOp::Ne
                        | BinaryOp::Gt
                        | BinaryOp::Lt
                        | BinaryOp::Ge
                        | BinaryOp::Le
                        | BinaryOp::And
                        | BinaryOp::Or
                ) && self.expr_numeric(left)
                    && self.expr_numeric(right)
            }
            Expr::Cast { ty, .. } => ty.name == "Namba",
            Expr::Call { callee, .. } => match &**callee {
                Expr::Ident { name, .. } => {
                    self.parent.numeric_functions.contains(name) || numeric_builtin(name)
                }
                _ => false,
            },
            Expr::Index { base, .. } => match &**base {
                Expr::Ident { name, .. } => self
                    .existing_slot(name)
                    .is_some_and(|slot| self.numeric_lists.contains(&slot)),
                _ => false,
            },
            Expr::Propagate { expr, .. } => self.expr_numeric(expr),
            Expr::MethodCall {
                receiver,
                method_name,
                args,
                ..
            } => {
                method_name == "urefu"
                    && args.is_empty()
                    && matches!(&**receiver, Expr::Ident { name, .. } if self.existing_slot(name).is_some())
            }
            _ => false,
        }
    }

    fn numeric_binary(&self, op: &BinaryOp, left: &Expr, right: &Expr) -> bool {
        let arithmetic = !matches!(op, BinaryOp::And | BinaryOp::Or);
        arithmetic && self.expr_numeric(left) && self.expr_numeric(right)
    }

    fn expr(&mut self, expr: &Expr) -> Option<()> {
        match expr {
            Expr::Number(_) => {
                let constant = self.parent.literal(expr)?;
                self.emit(Opcode::ConstNumber(constant));
            }
            Expr::String(_) | Expr::Bool(_) | Expr::Char(_) => {
                let constant = self.parent.literal(expr)?;
                self.emit(Opcode::Const(constant));
            }
            Expr::Ident { name, .. } => {
                if let Some(slot) = self.existing_slot(name) {
                    if self.numeric_locals.contains(&slot) {
                        self.emit(Opcode::LoadNumber(slot));
                    } else {
                        self.emit(Opcode::LoadLocal(slot));
                    }
                } else if let Some(index) = self.parent.module_constants.get(name) {
                    if matches!(
                        self.parent.constants.get(*index as usize),
                        Some(StoredConstant::Namba(_))
                    ) {
                        self.emit(Opcode::ConstNumber(*index));
                    } else {
                        self.emit(Opcode::Const(*index));
                    }
                } else {
                    return None;
                }
            }
            Expr::Group(e) => self.expr(e)?,
            Expr::List { elements, .. } => {
                for element in elements {
                    self.expr(element)?;
                }
                self.emit(Opcode::MakeList(elements.len() as u32));
            }
            Expr::Index { base, index, .. } => {
                if let Expr::Ident { name, .. } = &**base {
                    if let Some(slot) = self.existing_slot(name) {
                        if self.numeric_lists.contains(&slot) {
                            if let Some(index_slot) = self.index_slot(index) {
                                self.emit(Opcode::IndexNumberLocalFromSlot {
                                    list: slot,
                                    index: index_slot,
                                });
                                return Some(());
                            }
                        }
                        self.expr(index)?;
                        if self.numeric_lists.contains(&slot) {
                            self.emit(Opcode::IndexNumberLocal(slot));
                        } else {
                            self.emit(Opcode::IndexLocal(slot));
                        }
                        return Some(());
                    }
                }
                self.expr(base)?;
                self.expr(index)?;
                self.emit(Opcode::Index);
            }
            Expr::Propagate { expr, .. } => {
                self.expr(expr)?;
                if self.expr_numeric(expr) {
                    self.emit(Opcode::UnwrapNumber);
                } else {
                    self.emit(Opcode::Unwrap);
                }
            }
            Expr::Cast { expr, ty, .. } => {
                self.expr(expr)?;
                self.emit(Opcode::Cast(ty.name.clone()));
            }
            Expr::Unary { op, expr, .. } => {
                self.expr(expr)?;
                match op {
                    UnaryOp::Neg => {
                        self.emit(if self.expr_numeric(expr) {
                            Opcode::NumericUnary(UnaryCode::Neg)
                        } else {
                            Opcode::Unary(UnaryCode::Neg)
                        });
                    }
                    UnaryOp::Not => {
                        self.emit(Opcode::Unary(UnaryCode::Not));
                    }
                    UnaryOp::BitNot => {
                        self.emit(if self.expr_numeric(expr) {
                            Opcode::NumericUnary(UnaryCode::BitNot)
                        } else {
                            Opcode::Unary(UnaryCode::BitNot)
                        });
                    }
                    UnaryOp::BorrowImm | UnaryOp::BorrowMut => {}
                    UnaryOp::Jaribu => {
                        self.emit(Opcode::Unwrap);
                    }
                };
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                if matches!(op, BinaryOp::Add) {
                    if let Some((slot, index_slot, add)) = self.fused_index_add(left, right) {
                        self.emit(Opcode::IndexAddNumberLocal {
                            slot,
                            index_slot,
                            add,
                        });
                        return Some(());
                    }
                }
                self.expr(left)?;
                self.expr(right)?;
                let binary = match op {
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
                    BinaryOp::And => BinaryCode::And,
                    BinaryOp::Or => BinaryCode::Or,
                };
                if self.numeric_binary(op, left, right) {
                    self.emit(Opcode::NumericBinary(binary));
                } else {
                    self.emit(Opcode::Binary(binary));
                }
            }
            Expr::Call { callee, args, .. } => {
                let Expr::Ident { name, .. } = &**callee else {
                    return None;
                };
                if let Some(op) = numeric_builtin_opcode(name, args.len()) {
                    for arg in args {
                        self.expr(arg)?;
                    }
                    self.emit(Opcode::NumericBuiltin(op));
                    return Some(());
                }
                for arg in args {
                    self.expr(arg)?;
                }
                if let Some(function) = self.parent.function_names.get(name) {
                    self.emit(Opcode::Call {
                        function: *function,
                        arity: args.len() as u32,
                    });
                } else if let Some(builtin) = builtin_names().iter().position(|n| n == name) {
                    self.emit(Opcode::CallBuiltinN {
                        builtin: builtin as u32,
                        arity: args.len() as u32,
                    });
                } else {
                    return None;
                }
            }
            Expr::MethodCall {
                receiver,
                method_name,
                args,
                ..
            } => {
                if let Expr::Ident { name, .. } = &**receiver {
                    if let Some(slot) = self.existing_slot(name) {
                        match method_name.as_str() {
                            "urefu" if args.is_empty() => {
                                self.emit(Opcode::ListLen(slot));
                                return Some(());
                            }
                            "ongeza" if args.len() == 1 => {
                                self.expr(&args[0])?;
                                if self.numeric_lists.contains(&slot) && self.expr_numeric(&args[0])
                                {
                                    self.emit(Opcode::ListPushNumber(slot));
                                } else {
                                    self.emit(Opcode::ListPush(slot));
                                }
                                return Some(());
                            }
                            "ingiza" if args.len() == 2 => {
                                self.expr(&args[0])?;
                                self.expr(&args[1])?;
                                if self.numeric_lists.contains(&slot) && self.expr_numeric(&args[1])
                                {
                                    self.emit(Opcode::ListSetNumber(slot));
                                } else {
                                    self.emit(Opcode::ListSet(slot));
                                }
                                return Some(());
                            }
                            "ondoa" if args.len() == 1 => {
                                self.expr(&args[0])?;
                                self.emit(Opcode::ListRemove(slot));
                                return Some(());
                            }
                            _ => {}
                        }
                    }
                }
                self.expr(receiver)?;
                for arg in args {
                    self.expr(arg)?;
                }
                self.emit(Opcode::CallMethod {
                    method: method_name.clone(),
                    arity: args.len() as u32,
                });
            }
            Expr::Hamna
            | Expr::Map { .. }
            | Expr::StructLiteral { .. }
            | Expr::EnumConstruct { .. }
            | Expr::FieldAccess { .. }
            | Expr::If { .. } => return None,
        }
        Some(())
    }

    fn fused_index_add(&self, left: &Expr, right: &Expr) -> Option<(u32, u32, u32)> {
        let Expr::Propagate { expr, .. } = left else {
            return None;
        };
        let Expr::Index { base, index, .. } = &**expr else {
            return None;
        };
        let Expr::Ident { name: list, .. } = &**base else {
            return None;
        };
        let Expr::Ident {
            name: index_name, ..
        } = &**index
        else {
            return None;
        };
        let Expr::Number(literal) = right else {
            return None;
        };
        let slot = self.existing_slot(list)?;
        let index_slot = self.existing_slot(index_name)?;
        if !self.numeric_lists.contains(&slot) || !self.numeric_locals.contains(&index_slot) {
            return None;
        }
        let add = value::parse_number(literal);
        if !add.is_finite() || add.fract() != 0.0 || add < 0.0 || add > u32::MAX as f64 {
            return None;
        }
        Some((slot, index_slot, add as u32))
    }

    fn index_slot(&self, expr: &Expr) -> Option<u32> {
        match expr {
            Expr::Ident { name, .. } => self
                .existing_slot(name)
                .filter(|slot| self.numeric_locals.contains(slot)),
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } if matches!(&**right, Expr::Number(value) if value::parse_number(value) == 0.0) => {
                self.index_slot(left)
            }
            Expr::Group(inner) => self.index_slot(inner),
            _ => None,
        }
    }
}

/// Execute the entry function in a bytecode program.
pub fn run_bytecode(program: &BytecodeProgram, args: Vec<String>) -> Result<(), EvalError> {
    let index = program
        .functions
        .iter()
        .position(|f| f.name == program.entry)
        .ok_or_else(|| EvalError::Unknown(format!("kazi '{}' haikupatikana", program.entry)))?;
    let hoja = Value::Orodha(args.into_iter().map(Value::Neno).collect());
    let _ = execute_function(program, index, vec![hoja])?;
    Ok(())
}

/// Execute a named bytecode function.  This is useful for embedders and focused VM tests; the CLI
/// entry point above intentionally keeps the `kuu(hoja)` interface.
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
    execute_function(program, index, args)
}

#[derive(Clone, Copy, Debug)]
enum VmNumber {
    Integer(i64),
    Float(f64),
}

impl VmNumber {
    fn as_f64(self) -> f64 {
        match self {
            Self::Integer(n) => n as f64,
            Self::Float(n) => n,
        }
    }

    fn as_i64(self) -> i64 {
        self.as_f64() as i64
    }

    fn into_value(self) -> Value {
        Value::Namba(self.as_f64())
    }
}

enum VmValue {
    Number(VmNumber),
    Value(Value),
    /// The numeric-list read stays unboxed until a following `jaribu`/`?`.
    NumberResult(Result<VmNumber, Value>),
}

enum LocalSlot {
    Empty,
    Number(VmNumber),
    NumericList(Vec<VmNumber>),
    Value(Value),
}

impl LocalSlot {
    fn from_vm(value: VmValue) -> Self {
        match value {
            VmValue::Number(number) => Self::Number(number),
            VmValue::Value(value) => Self::Value(value),
            VmValue::NumberResult(result) => match result {
                Ok(number) => Self::Number(number),
                Err(error) => Self::Value(Value::Tokeo(Err(Box::new(error)))),
            },
        }
    }

    fn from_numeric_list(value: VmValue) -> Self {
        match value {
            VmValue::Value(Value::Orodha(values))
                if values.iter().all(|value| matches!(value, Value::Namba(_))) =>
            {
                Self::NumericList(
                    values
                        .into_iter()
                        .map(|value| match value {
                            Value::Namba(number) => VmNumber::Float(number),
                            _ => unreachable!(),
                        })
                        .collect(),
                )
            }
            other => Self::from_vm(other),
        }
    }

    fn to_vm(&self) -> VmValue {
        match self {
            Self::Empty => VmValue::Value(Value::Hamna),
            Self::Number(number) => VmValue::Number(*number),
            Self::NumericList(values) => VmValue::Value(Value::Orodha(
                values.iter().map(|value| value.into_value()).collect(),
            )),
            Self::Value(value) => VmValue::Value(value.clone()),
        }
    }
}

fn vm_value(value: Value) -> VmValue {
    match value {
        Value::Namba(number) => VmValue::Number(VmNumber::Float(number)),
        value => VmValue::Value(value),
    }
}

fn vm_into_value(value: VmValue) -> Value {
    match value {
        VmValue::Number(number) => number.into_value(),
        VmValue::Value(value) => value,
        VmValue::NumberResult(Ok(number)) => number.into_value(),
        VmValue::NumberResult(Err(error)) => Value::Tokeo(Err(Box::new(error))),
    }
}

fn pop_vm(stack: &mut Vec<VmValue>) -> Result<VmValue, EvalError> {
    stack
        .pop()
        .ok_or_else(|| EvalError::Unknown("stack imeharibika".into()))
}

fn pop_number(stack: &mut Vec<VmValue>) -> Result<VmNumber, EvalError> {
    match pop_vm(stack)? {
        VmValue::Number(number) => Ok(number),
        VmValue::Value(Value::Namba(number)) => Ok(VmNumber::Float(number)),
        other => Err(EvalError::TypeErr(format!(
            "operesheni inahitaji Namba, ilipata {:?}",
            vm_into_value(other)
        ))),
    }
}

fn local_number(local: &LocalSlot) -> Result<VmNumber, EvalError> {
    match local {
        LocalSlot::Number(number) => Ok(*number),
        LocalSlot::Value(Value::Namba(number)) => Ok(VmNumber::Float(*number)),
        _ => Err(EvalError::TypeErr("operesheni inahitaji Namba".into())),
    }
}

struct VmPools {
    locals: Vec<Vec<LocalSlot>>,
    stacks: Vec<Vec<VmValue>>,
}

fn builtin_cache() -> Result<Vec<crate::builtins::BuiltinFn>, EvalError> {
    let names = builtin_names();
    let mut table = builtins();
    names
        .into_iter()
        .map(|name| {
            table
                .remove(&name)
                .ok_or_else(|| EvalError::Unknown(format!("builtin haipo: {name}")))
        })
        .collect()
}

impl VmPools {
    fn take_locals(&mut self, len: usize) -> Vec<LocalSlot> {
        let mut locals = self.locals.pop().unwrap_or_default();
        locals.clear();
        locals.resize_with(len, || LocalSlot::Empty);
        locals
    }

    fn take_stack(&mut self) -> Vec<VmValue> {
        self.stacks.pop().unwrap_or_default()
    }

    fn put(&mut self, mut locals: Vec<LocalSlot>, mut stack: Vec<VmValue>) {
        locals.clear();
        stack.clear();
        self.locals.push(locals);
        self.stacks.push(stack);
    }
}

fn execute_function(
    program: &BytecodeProgram,
    function: usize,
    args: Vec<Value>,
) -> Result<Value, EvalError> {
    let mut pools = VmPools {
        locals: Vec::new(),
        stacks: Vec::new(),
    };
    let builtin_cache = builtin_cache()?;
    execute_function_with_pools(program, function, args, &mut pools, &builtin_cache)
}

fn execute_function_with_pools(
    program: &BytecodeProgram,
    function: usize,
    args: Vec<Value>,
    pools: &mut VmPools,
    builtin_cache: &[crate::builtins::BuiltinFn],
) -> Result<Value, EvalError> {
    let f = program
        .functions
        .get(function)
        .ok_or_else(|| EvalError::Unknown("faharisi ya kazi si halali".into()))?;
    let mut locals = pools.take_locals(f.arity as usize);
    for (i, value) in args.into_iter().enumerate() {
        if i < locals.len() {
            locals[i] = LocalSlot::from_vm(vm_value(value));
        }
    }
    let mut stack = pools.take_stack();
    let mut ip = 0usize;
    while ip < f.code.len() {
        match &f.code[ip] {
            Opcode::Const(i) => stack.push(vm_value(program.get_constant(*i))),
            Opcode::ConstNumber(i) => {
                let number = match program.constants.get(*i as usize) {
                    Some(StoredConstant::Namba(number)) => *number,
                    _ => return Err(EvalError::TypeErr("fahirisi ya namba si Namba".into())),
                };
                stack.push(VmValue::Number(VmNumber::Float(number)));
            }
            Opcode::LoadLocal(i) => stack.push(
                locals
                    .get(*i as usize)
                    .map(LocalSlot::to_vm)
                    .unwrap_or(VmValue::Value(Value::Hamna)),
            ),
            Opcode::LoadNumber(i) => {
                let number = locals
                    .get(*i as usize)
                    .map(local_number)
                    .transpose()?
                    .unwrap_or(VmNumber::Float(0.0));
                stack.push(VmValue::Number(number));
            }
            Opcode::StoreLocal(i) => {
                let value = pop_vm(&mut stack)?;
                if *i as usize >= locals.len() {
                    locals.resize_with(*i as usize + 1, || LocalSlot::Empty);
                }
                locals[*i as usize] = LocalSlot::from_vm(value);
            }
            Opcode::StoreNumber(i) => {
                let value = pop_number(&mut stack)?;
                if *i as usize >= locals.len() {
                    locals.resize_with(*i as usize + 1, || LocalSlot::Empty);
                }
                locals[*i as usize] = LocalSlot::Number(value);
            }
            Opcode::StoreNumericList(i) => {
                let value = pop_vm(&mut stack)?;
                if *i as usize >= locals.len() {
                    locals.resize_with(*i as usize + 1, || LocalSlot::Empty);
                }
                locals[*i as usize] = LocalSlot::from_numeric_list(value);
            }
            Opcode::Pop => {
                let _ = stack.pop();
            }
            Opcode::Jump(target) => {
                ip = *target as usize;
                continue;
            }
            Opcode::JumpIfFalse(target) => {
                let value = vm_into_value(pop_vm(&mut stack)?);
                if !matches!(value, Value::Ukweli(true)) {
                    ip = *target as usize;
                    continue;
                }
            }
            Opcode::Binary(op) => {
                let right = vm_into_value(pop_vm(&mut stack)?);
                let left = vm_into_value(pop_vm(&mut stack)?);
                stack.push(vm_value(binary(op, left, right)?));
            }
            Opcode::NumericBinary(op) => {
                let right = pop_number(&mut stack)?;
                let left = pop_number(&mut stack)?;
                stack.push(numeric_binary(op, left, right)?);
            }
            Opcode::Unary(op) => {
                let value = vm_into_value(pop_vm(&mut stack)?);
                stack.push(vm_value(unary(op, value)?));
            }
            Opcode::NumericUnary(op) => {
                let value = pop_number(&mut stack)?;
                stack.push(numeric_unary(op, value)?);
            }
            Opcode::NumericBuiltin(op) => {
                let right = pop_number(&mut stack)?;
                let left = match op {
                    NumericBuiltinCode::Floor => right,
                    _ => pop_number(&mut stack)?,
                };
                stack.push(numeric_builtin_value(op, left, right)?);
            }
            Opcode::MakeList(n) => {
                let n = *n as usize;
                if stack.len() < n {
                    return Err(EvalError::Unknown("stack imeharibika".into()));
                }
                let values = stack
                    .split_off(stack.len() - n)
                    .into_iter()
                    .map(vm_into_value)
                    .collect();
                stack.push(VmValue::Value(Value::Orodha(values)));
            }
            Opcode::Index => {
                let index = vm_into_value(pop_vm(&mut stack)?);
                let base = vm_into_value(pop_vm(&mut stack)?);
                stack.push(vm_value(index_value(&base, &index)?));
            }
            Opcode::IndexLocal(slot) => {
                let index = number_index(&vm_into_value(pop_vm(&mut stack)?))?;
                let value = match locals.get(*slot as usize) {
                    Some(LocalSlot::Value(Value::Orodha(values))) if index < values.len() => {
                        Value::Tokeo(Ok(Box::new(values[index].clone())))
                    }
                    Some(LocalSlot::NumericList(values)) if index < values.len() => {
                        Value::Tokeo(Ok(Box::new(values[index].into_value())))
                    }
                    Some(LocalSlot::Value(Value::Orodha(_))) => Value::Tokeo(Err(Box::new(
                        Value::Neno(format!("fahirisi nje ya mipaka: {index}")),
                    ))),
                    Some(LocalSlot::NumericList(_)) => Value::Tokeo(Err(Box::new(Value::Neno(
                        format!("fahirisi nje ya mipaka: {index}"),
                    )))),
                    _ => return Err(EvalError::TypeErr("fahirisi inahitaji Orodha".into())),
                };
                stack.push(vm_value(value));
            }
            Opcode::IndexNumberLocal(slot) => {
                let index = number_index(&vm_into_value(pop_vm(&mut stack)?))?;
                let value = match locals.get(*slot as usize) {
                    Some(LocalSlot::Value(Value::Orodha(values)))
                        if index < values.len() && matches!(values[index], Value::Namba(_)) =>
                    {
                        Ok(VmNumber::Float(match &values[index] {
                            Value::Namba(number) => *number,
                            _ => unreachable!(),
                        }))
                    }
                    Some(LocalSlot::NumericList(values)) if index < values.len() => {
                        Ok(values[index])
                    }
                    Some(LocalSlot::Value(Value::Orodha(_))) => {
                        Err(Value::Neno(format!("fahirisi nje ya mipaka: {index}")))
                    }
                    Some(LocalSlot::NumericList(_)) => {
                        Err(Value::Neno(format!("fahirisi nje ya mipaka: {index}")))
                    }
                    _ => return Err(EvalError::TypeErr("fahirisi inahitaji Orodha".into())),
                };
                stack.push(VmValue::NumberResult(value));
            }
            Opcode::IndexNumberLocalFromSlot { list, index } => {
                let index = local_number(
                    locals
                        .get(*index as usize)
                        .ok_or_else(|| EvalError::TypeErr("fahirisi inahitaji Namba".into()))?,
                )?
                .as_i64()
                .max(0) as usize;
                let value = match locals.get(*list as usize) {
                    Some(LocalSlot::NumericList(values)) if index < values.len() => {
                        Ok(values[index])
                    }
                    Some(LocalSlot::Value(Value::Orodha(values)))
                        if index < values.len() && matches!(values[index], Value::Namba(_)) =>
                    {
                        match &values[index] {
                            Value::Namba(number) => Ok(VmNumber::Float(*number)),
                            _ => unreachable!(),
                        }
                    }
                    Some(LocalSlot::NumericList(_)) | Some(LocalSlot::Value(Value::Orodha(_))) => {
                        Err(Value::Neno(format!("fahirisi nje ya mipaka: {index}")))
                    }
                    _ => return Err(EvalError::TypeErr("fahirisi inahitaji Orodha".into())),
                };
                stack.push(VmValue::NumberResult(value));
            }
            Opcode::ListLen(slot) => {
                let length = match locals.get(*slot as usize) {
                    Some(LocalSlot::Value(Value::Orodha(values))) => values.len(),
                    Some(LocalSlot::NumericList(values)) => values.len(),
                    Some(LocalSlot::Value(Value::Neno(value))) => {
                        unicode_segmentation::UnicodeSegmentation::graphemes(value.as_str(), true)
                            .count()
                    }
                    _ => return Err(EvalError::TypeErr("urefu inahitaji Orodha au Neno".into())),
                };
                stack.push(VmValue::Number(VmNumber::Integer(length as i64)));
            }
            Opcode::Unwrap => {
                let value = vm_into_value(pop_vm(&mut stack)?);
                match value {
                    Value::Tokeo(Ok(v)) | Value::Chaguo(Some(v)) => stack.push(vm_value(*v)),
                    Value::Tokeo(Err(e)) => {
                        pools.put(locals, stack);
                        return Ok(Value::Tokeo(Err(e)));
                    }
                    Value::Chaguo(None) => return Err(EvalError::Unknown("Chaguo: Hamna".into())),
                    _ => return Err(EvalError::TypeErr("jaribu inahitaji Tokeo/Chaguo".into())),
                }
            }
            Opcode::UnwrapNumber => match pop_vm(&mut stack)? {
                VmValue::NumberResult(Ok(number)) => stack.push(VmValue::Number(number)),
                VmValue::NumberResult(Err(error)) => {
                    pools.put(locals, stack);
                    return Ok(Value::Tokeo(Err(Box::new(error))));
                }
                value => match vm_into_value(value) {
                    Value::Tokeo(Ok(value)) | Value::Chaguo(Some(value)) => {
                        stack.push(vm_value(*value))
                    }
                    Value::Tokeo(Err(error)) => {
                        pools.put(locals, stack);
                        return Ok(Value::Tokeo(Err(error)));
                    }
                    _ => return Err(EvalError::TypeErr("jaribu inahitaji Tokeo/Chaguo".into())),
                },
            },
            Opcode::Cast(ty) => {
                let value = vm_into_value(pop_vm(&mut stack)?);
                stack.push(vm_value(cast(ty, value)?));
            }
            Opcode::Call { function, arity } => {
                let values = take_args(&mut stack, *arity as usize)?;
                let values = values.into_iter().map(vm_into_value).collect();
                stack.push(vm_value(execute_function_with_pools(
                    program,
                    *function as usize,
                    values,
                    pools,
                    builtin_cache,
                )?));
            }
            Opcode::CallBuiltinN { builtin, arity } => {
                let values = take_args(&mut stack, *arity as usize)?;
                let values = values.into_iter().map(vm_into_value).collect::<Vec<_>>();
                let callable = builtin_cache
                    .get(*builtin as usize)
                    .ok_or_else(|| EvalError::Unknown("faharisi ya builtin si halali".into()))?;
                stack.push(vm_value(callable(&values)?));
            }
            Opcode::CallBuiltin(builtin) => {
                let values = take_args(&mut stack, 1)?;
                let values = values.into_iter().map(vm_into_value).collect::<Vec<_>>();
                let callable = builtin_cache
                    .get(*builtin as usize)
                    .ok_or_else(|| EvalError::Unknown("faharisi ya builtin si halali".into()))?;
                stack.push(vm_value(callable(&values)?));
            }
            Opcode::CallMethod { method, arity } => {
                let values = take_args(&mut stack, *arity as usize)?;
                let values = values.into_iter().map(vm_into_value).collect::<Vec<_>>();
                let receiver = vm_into_value(pop_vm(&mut stack)?);
                stack.push(vm_value(method_call(receiver, method, values)?));
            }
            Opcode::ListPush(slot) => {
                let value = vm_into_value(pop_vm(&mut stack)?);
                match locals.get_mut(*slot as usize) {
                    Some(LocalSlot::Value(Value::Orodha(values))) => {
                        values.push(value);
                        stack.push(VmValue::Value(Value::Tupu));
                    }
                    Some(LocalSlot::NumericList(values)) => match value {
                        Value::Namba(number) => {
                            values.push(VmNumber::Float(number));
                            stack.push(VmValue::Value(Value::Tupu));
                        }
                        _ => return Err(EvalError::TypeErr("ongeza inahitaji Namba".into())),
                    },
                    _ => return Err(EvalError::TypeErr("ongeza inahitaji Orodha".into())),
                }
            }
            Opcode::ListPushNumber(slot) => {
                let value = pop_number(&mut stack)?;
                match locals.get_mut(*slot as usize) {
                    Some(LocalSlot::Value(Value::Orodha(values))) => {
                        values.push(value.into_value());
                        stack.push(VmValue::Value(Value::Tupu));
                    }
                    Some(LocalSlot::NumericList(values)) => {
                        values.push(value);
                        stack.push(VmValue::Value(Value::Tupu));
                    }
                    _ => return Err(EvalError::TypeErr("ongeza inahitaji Orodha".into())),
                }
            }
            Opcode::ListSet(slot) => {
                let value = vm_into_value(pop_vm(&mut stack)?);
                let index = number_index(&vm_into_value(pop_vm(&mut stack)?))?;
                match locals.get_mut(*slot as usize) {
                    Some(LocalSlot::Value(Value::Orodha(values))) if index < values.len() => {
                        values[index] = value;
                        stack.push(VmValue::Value(Value::Tupu));
                    }
                    Some(LocalSlot::Value(Value::Orodha(_))) => {
                        return Err(EvalError::TypeErr("ingiza: index nje ya mipaka".into()))
                    }
                    Some(LocalSlot::NumericList(values)) if index < values.len() => {
                        values[index] = match value {
                            Value::Namba(number) => VmNumber::Float(number),
                            _ => return Err(EvalError::TypeErr("ingiza inahitaji Namba".into())),
                        };
                        stack.push(VmValue::Value(Value::Tupu));
                    }
                    Some(LocalSlot::NumericList(_)) => {
                        return Err(EvalError::TypeErr("ingiza: index nje ya mipaka".into()))
                    }
                    _ => return Err(EvalError::TypeErr("ingiza inahitaji Orodha".into())),
                }
            }
            Opcode::ListSetNumber(slot) => {
                let value = pop_number(&mut stack)?;
                let index = number_index(&vm_into_value(pop_vm(&mut stack)?))?;
                match locals.get_mut(*slot as usize) {
                    Some(LocalSlot::Value(Value::Orodha(values))) if index < values.len() => {
                        values[index] = value.into_value();
                        stack.push(VmValue::Value(Value::Tupu));
                    }
                    Some(LocalSlot::Value(Value::Orodha(_))) => {
                        return Err(EvalError::TypeErr("ingiza: index nje ya mipaka".into()))
                    }
                    Some(LocalSlot::NumericList(values)) if index < values.len() => {
                        values[index] = value;
                        stack.push(VmValue::Value(Value::Tupu));
                    }
                    Some(LocalSlot::NumericList(_)) => {
                        return Err(EvalError::TypeErr("ingiza: index nje ya mipaka".into()))
                    }
                    _ => return Err(EvalError::TypeErr("ingiza inahitaji Orodha".into())),
                }
            }
            Opcode::ListRemove(slot) => {
                let index = number_index(&vm_into_value(pop_vm(&mut stack)?))?;
                match locals.get_mut(*slot as usize) {
                    Some(LocalSlot::Value(Value::Orodha(values))) if index < values.len() => stack
                        .push(VmValue::Value(Value::Chaguo(Some(Box::new(
                            values.remove(index),
                        ))))),
                    Some(LocalSlot::Value(Value::Orodha(_))) => {
                        stack.push(VmValue::Value(Value::Chaguo(None)))
                    }
                    Some(LocalSlot::NumericList(values)) if index < values.len() => {
                        stack.push(VmValue::Value(Value::Chaguo(Some(Box::new(
                            values.remove(index).into_value(),
                        )))))
                    }
                    Some(LocalSlot::NumericList(_)) => {
                        stack.push(VmValue::Value(Value::Chaguo(None)))
                    }
                    _ => return Err(EvalError::TypeErr("ondoa inahitaji Orodha".into())),
                }
            }
            Opcode::IncrementNumberLocal { slot, by } => {
                let value = local_number(
                    locals
                        .get(*slot as usize)
                        .ok_or_else(|| EvalError::TypeErr("ongezeko inahitaji Namba".into()))?,
                )?;
                let increment = VmNumber::Integer(*by as i64);
                locals[*slot as usize] = LocalSlot::Number(numeric_add(value, increment));
            }
            Opcode::IndexAddNumberLocal {
                slot,
                index_slot,
                add,
            } => {
                let index = match locals.get(*index_slot as usize) {
                    Some(LocalSlot::Number(number)) => number.as_i64() as usize,
                    _ => return Err(EvalError::TypeErr("fahirisi inahitaji Namba".into())),
                };
                let value = match locals.get(*slot as usize) {
                    Some(LocalSlot::Value(Value::Orodha(values))) => values
                        .get(index)
                        .and_then(|value| match value {
                            Value::Namba(number) => Some(*number),
                            _ => None,
                        })
                        .ok_or_else(|| EvalError::TypeErr("fahirisi inahitaji Namba".into()))?,
                    Some(LocalSlot::NumericList(values)) => values
                        .get(index)
                        .map(|value| value.as_f64())
                        .ok_or_else(|| EvalError::TypeErr("fahirisi inahitaji Namba".into()))?,
                    _ => return Err(EvalError::TypeErr("fahirisi inahitaji Orodha".into())),
                };
                stack.push(VmValue::Number(numeric_add(
                    VmNumber::Float(value),
                    VmNumber::Integer(*add as i64),
                )));
            }
            Opcode::Return => {
                let value = vm_into_value(stack.pop().unwrap_or(VmValue::Value(Value::Tupu)));
                pools.put(locals, stack);
                return Ok(value);
            }
            Opcode::Nop => {}
        }
        ip += 1;
    }
    pools.put(locals, stack);
    Ok(Value::Tupu)
}

fn take_args(stack: &mut Vec<VmValue>, arity: usize) -> Result<Vec<VmValue>, EvalError> {
    if stack.len() < arity {
        return Err(EvalError::Unknown("hoja chache kwenye stack".into()));
    }
    let mut args = stack.split_off(stack.len() - arity);
    // Evaluation pushes left-to-right, while split_off already preserves that order.
    Ok(std::mem::take(&mut args))
}

fn number_index(value: &Value) -> Result<usize, EvalError> {
    value::as_f64(value)
        .map(|n| (n as i64).max(0) as usize)
        .ok_or_else(|| EvalError::TypeErr("fahirisi inahitaji Namba".into()))
}

fn index_value(base: &Value, index: &Value) -> Result<Value, EvalError> {
    match base {
        Value::Orodha(values) => {
            let index = number_index(index)?;
            if index >= values.len() {
                Ok(Value::Tokeo(Err(Box::new(Value::Neno(format!(
                    "fahirisi nje ya mipaka: {index}"
                ))))))
            } else {
                Ok(Value::Tokeo(Ok(Box::new(values[index].clone()))))
            }
        }
        Value::Kamusi(map) => {
            let key = crate::value::MapKey::try_from_value(index)?;
            Ok(map.get(&key).cloned().unwrap_or(Value::Hamna))
        }
        _ => Err(EvalError::TypeErr(
            "fahirisi inahitaji Orodha au Kamusi".into(),
        )),
    }
}

fn method_call(receiver: Value, method: &str, _args: Vec<Value>) -> Result<Value, EvalError> {
    match (receiver, method) {
        (Value::Orodha(values), "urefu") => Ok(Value::Namba(values.len() as f64)),
        (Value::Neno(value), "urefu") => Ok(Value::Namba(
            unicode_segmentation::UnicodeSegmentation::graphemes(value.as_str(), true).count()
                as f64,
        )),
        _ => Err(EvalError::Unknown(format!(
            "bytecode method haijaungwa mkono: {method}"
        ))),
    }
}

fn numeric_add(left: VmNumber, right: VmNumber) -> VmNumber {
    match (left, right) {
        (VmNumber::Integer(a), VmNumber::Integer(b)) => VmNumber::Integer(a + b),
        (left, right) => VmNumber::Float(left.as_f64() + right.as_f64()),
    }
}

fn numeric_binary(op: &BinaryCode, left: VmNumber, right: VmNumber) -> Result<VmValue, EvalError> {
    let result = match op {
        BinaryCode::Add => VmValue::Number(numeric_add(left, right)),
        BinaryCode::Sub => VmValue::Number(match (left, right) {
            (VmNumber::Integer(a), VmNumber::Integer(b)) => VmNumber::Integer(a - b),
            (left, right) => VmNumber::Float(left.as_f64() - right.as_f64()),
        }),
        BinaryCode::Mul => VmValue::Number(match (left, right) {
            (VmNumber::Integer(a), VmNumber::Integer(b)) => VmNumber::Integer(a * b),
            (left, right) => VmNumber::Float(left.as_f64() * right.as_f64()),
        }),
        BinaryCode::Div => VmValue::Number(VmNumber::Float(left.as_f64() / right.as_f64())),
        BinaryCode::Rem => VmValue::Number(VmNumber::Float(left.as_f64() % right.as_f64())),
        BinaryCode::Pow => VmValue::Number(VmNumber::Float(left.as_f64().powf(right.as_f64()))),
        BinaryCode::Eq => VmValue::Value(Value::Ukweli(left.as_f64() == right.as_f64())),
        BinaryCode::Ne => VmValue::Value(Value::Ukweli(left.as_f64() != right.as_f64())),
        BinaryCode::Gt => VmValue::Value(Value::Ukweli(left.as_f64() > right.as_f64())),
        BinaryCode::Lt => VmValue::Value(Value::Ukweli(left.as_f64() < right.as_f64())),
        BinaryCode::Ge => VmValue::Value(Value::Ukweli(left.as_f64() >= right.as_f64())),
        BinaryCode::Le => VmValue::Value(Value::Ukweli(left.as_f64() <= right.as_f64())),
        BinaryCode::BitAnd => VmValue::Number(VmNumber::Integer(left.as_i64() & right.as_i64())),
        BinaryCode::BitXor => VmValue::Number(VmNumber::Integer(left.as_i64() ^ right.as_i64())),
        BinaryCode::BitOr => VmValue::Number(VmNumber::Integer(left.as_i64() | right.as_i64())),
        BinaryCode::Shl => VmValue::Number(VmNumber::Integer(
            left.as_i64().wrapping_shl(right.as_i64() as u32),
        )),
        BinaryCode::Shr => VmValue::Number(VmNumber::Integer(
            left.as_i64().wrapping_shr(right.as_i64() as u32),
        )),
        BinaryCode::And | BinaryCode::Or => {
            return Err(EvalError::TypeErr("na/au inahitaji Ukweli".into()))
        }
    };
    Ok(result)
}

fn numeric_unary(op: &UnaryCode, value: VmNumber) -> Result<VmValue, EvalError> {
    Ok(match op {
        UnaryCode::Neg => VmValue::Number(match value {
            VmNumber::Integer(number) => VmNumber::Integer(-number),
            VmNumber::Float(number) => VmNumber::Float(-number),
        }),
        UnaryCode::BitNot => VmValue::Number(VmNumber::Integer(!value.as_i64())),
        _ => return Err(EvalError::TypeErr("operesheni si ya Namba".into())),
    })
}

fn numeric_builtin_value(
    op: &NumericBuiltinCode,
    left: VmNumber,
    right: VmNumber,
) -> Result<VmValue, EvalError> {
    Ok(VmValue::Number(match op {
        NumericBuiltinCode::Floor => VmNumber::Float(right.as_f64().floor()),
        NumericBuiltinCode::ShiftLeft => {
            VmNumber::Integer(left.as_i64().wrapping_shl(right.as_i64() as u32))
        }
        NumericBuiltinCode::ShiftRight => {
            VmNumber::Integer(left.as_i64().wrapping_shr(right.as_i64() as u32))
        }
        NumericBuiltinCode::BitAnd => VmNumber::Integer(left.as_i64() & right.as_i64()),
        NumericBuiltinCode::BitOr => VmNumber::Integer(left.as_i64() | right.as_i64()),
        NumericBuiltinCode::BitXor => VmNumber::Integer(left.as_i64() ^ right.as_i64()),
    }))
}

fn cast(ty: &str, value: Value) -> Result<Value, EvalError> {
    match ty {
        "Neno" => Ok(Value::Neno(match value {
            Value::Namba(n) => n.to_string(),
            Value::Ukweli(true) => "kweli".into(),
            Value::Ukweli(false) => "si_kweli".into(),
            Value::Neno(s) => s,
            Value::Herufi(c) => c.to_string(),
            other => format!("{other:?}"),
        })),
        "Ukweli" => Ok(Value::Ukweli(match value {
            Value::Ukweli(b) => b,
            Value::Namba(n) => n != 0.0,
            _ => true,
        })),
        _ => Ok(value),
    }
}

fn unary(op: &UnaryCode, value: Value) -> Result<Value, EvalError> {
    match op {
        UnaryCode::Neg => {
            Ok(Value::Namba(-value::as_f64(&value).ok_or_else(|| {
                EvalError::TypeErr("- inahitaji Namba".into())
            })?))
        }
        UnaryCode::Not => Ok(Value::Ukweli(!matches!(value, Value::Ukweli(true)))),
        UnaryCode::BitNot => Ok(Value::Namba(
            !(value::as_f64(&value)
                .ok_or_else(|| EvalError::TypeErr("siyo_biti inahitaji Namba".into()))?
                as i64) as f64,
        )),
    }
}

fn binary(op: &BinaryCode, left: Value, right: Value) -> Result<Value, EvalError> {
    let numbers = || -> Result<(f64, f64), EvalError> {
        Ok((
            value::as_f64(&left)
                .ok_or_else(|| EvalError::TypeErr("operesheni inahitaji Namba".into()))?,
            value::as_f64(&right)
                .ok_or_else(|| EvalError::TypeErr("operesheni inahitaji Namba".into()))?,
        ))
    };
    Ok(match op {
        BinaryCode::Add => match (value::as_string(&left), value::as_string(&right)) {
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
            Value::Namba(((a as i64) << (b as i64)) as f64)
        }
        BinaryCode::Shr => {
            let (a, b) = numbers()?;
            Value::Namba(((a as i64) >> (b as i64)) as f64)
        }
        BinaryCode::And => match (left, right) {
            (Value::Ukweli(a), Value::Ukweli(b)) => Value::Ukweli(a && b),
            _ => return Err(EvalError::TypeErr("na inahitaji Ukweli".into())),
        },
        BinaryCode::Or => match (left, right) {
            (Value::Ukweli(a), Value::Ukweli(b)) => Value::Ukweli(a || b),
            _ => return Err(EvalError::TypeErr("au inahitaji Ukweli".into())),
        },
    })
}
