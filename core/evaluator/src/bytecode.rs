//! Minimal bytecode ISA and VM for format=bytecode .asb (future expansion).
//
// TODO(Phase II/IV): This ISA is a skeleton. Missing opcodes needed for real programs:
//   - Jump(offset) / JumpIfFalse(offset) — control flow
//   - Call(fn_idx, arity) / CallMethod(method_idx, arity) — function dispatch
//   - Add / Sub / Mul / Div / Rem / Pow — arithmetic
//   - Eq / Ne / Lt / Gt / Le / Ge — comparisons
//   - And / Or / Not — logical
//   - MakeList(n) / MakeMap(n) / MakeStruct(type_idx, n) — collection constructors
//   - GetField(field_idx) / SetField(field_idx) — struct access
//   - Index / SetIndex — collection indexing
//   - Borrow / BorrowMut / Drop — ownership (Phase III)
// run_bytecode currently only handles Const, Return, LoadLocal, StoreLocal, and CallBuiltin(0).

use crate::value::{EvalError, Value};
use serde::{Deserialize, Serialize};

/// Single bytecode function: name, arity, instruction stream.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeFunc {
    pub name: String,
    pub arity: u32,
    pub code: Vec<Opcode>,
}

/// Minimal ISA; expand for full language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Opcode {
    /// Push constant index (constants table).
    Const(u32),
    /// Return from current function.
    Return,
    /// Call builtin by index (0 = chapisha, etc.).
    CallBuiltin(u32),
    /// Load local by index.
    LoadLocal(u32),
    /// Store local by index.
    StoreLocal(u32),
    /// TODO: Reserved — replace with Jump, Call, arithmetic ops, etc. when ISA is expanded.
    Nop,
}

/// Bytecode program: constants pool and functions. Entry is "kuu".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BytecodeProgram {
    #[serde(default)]
    pub constants: Vec<StoredConstant>,
    pub functions: Vec<BytecodeFunc>,
    pub entry: String,
}

/// Constant stored in bytecode (simplified for serialization; expand to full Value later).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StoredConstant {
    Neno(String),
    Namba(f64),
    Tupu,
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
                StoredConstant::Tupu => Value::Tupu,
            })
            .unwrap_or(Value::Tupu)
    }
}

use crate::builtins::{builtin_names, builtins};

// ... (other imports) ...

/// Run the bytecode program: invoke entry (e.g. "kuu") with args.
pub fn run_bytecode(program: &BytecodeProgram, args: Vec<String>) -> Result<(), EvalError> {
    let func = program
        .find_function(&program.entry)
        .ok_or_else(|| EvalError::Unknown(format!("kazi '{}' haikupatikana", program.entry)))?;
    if func.arity != 1 {
        return Err(EvalError::Unknown(
            "bytecode VM: kuu inahitaji hoja moja (Orodha<Neno>)".to_string(),
        ));
    }
    let hoja = Value::Orodha(args.into_iter().map(Value::Neno).collect());
    let mut stack: Vec<Value> = Vec::new();
    let mut locals: Vec<Value> = vec![hoja];
    let code = &func.code;
    let mut ip = 0;

    let builtin_map = builtins();
    let builtin_list = builtin_names();

    while ip < code.len() {
        match &code[ip] {
            Opcode::Const(idx) => {
                let v = program.get_constant(*idx);
                stack.push(v);
            }
            Opcode::Return => break,
            Opcode::LoadLocal(idx) => {
                let v = locals.get(*idx as usize).cloned().unwrap_or(Value::Hamna);
                stack.push(v);
            }
            Opcode::StoreLocal(idx) => {
                let v = stack.pop().unwrap_or(Value::Tupu);
                while locals.len() <= *idx as usize {
                    locals.push(Value::Hamna);
                }
                locals[*idx as usize] = v;
            }
            Opcode::CallBuiltin(idx) => {
                let name = builtin_list.get(*idx as usize)
                    .ok_or_else(|| EvalError::Unknown(format!("faharisi ya batili iliyojengwa: {idx}")))?;
                
                let builtin_fn = builtin_map.get(name)
                    .ok_or_else(|| EvalError::Unknown(format!("iliyojengwa haikupatikana: {name}")))?;

                // Assume 1 argument for now (matching previous chapisha implementation).
                // This will need better arity management in the ISA/VM later.
                let args = vec![stack.pop().unwrap_or(Value::Tupu)];
                let result = builtin_fn(&args)?;
                stack.push(result);
            }
            Opcode::Nop => {}
        }
        ip += 1;
    }
    Ok(())
}
