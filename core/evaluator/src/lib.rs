//! Asili interpreter and TIR/ASB emission.

mod asb;
mod builtins;
mod bytecode;
mod env;
mod eval;
mod runtime;
mod signal;
mod tir;
mod value;

pub use asb::{load_asb, load_asb_bytecode, parse_format, AsbLoadError};
pub use bytecode::{run_bytecode, BytecodeProgram};
pub use env::Env;
pub use eval::eval_expr;
pub use tir::{emit_asb_from_tir, lower_to_tir, TypedIrFunction, TypedIrModule, validate_module};
pub use value::{EvalError, EvalOut, Value};

use asili_parser::{Block, Function, Module};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

/// Run a block with an existing env (e.g. for REPL). Uses the given module for symbol resolution.
/// Pushes a new scope for the block; bindings do not persist after the block ends.
pub fn run_block(
    module: &Module,
    block: &Block,
    env: &mut Env,
) -> Result<Value, EvalError> {
    let mut rt = runtime::Runtime::new(env, module);
    match eval::eval_block_impl(block, &mut rt) {
        Ok(EvalOut::Return(v)) => Ok(v),
        Ok(_) => Ok(Value::Tupu),
        Err(e) => Err(e),
    }
}

/// Run a block in the current env without a new scope. Use for REPL so that `weka` bindings persist.
pub fn run_block_in_env(
    module: &Module,
    block: &Block,
    env: &mut Env,
) -> Result<Value, EvalError> {
    run_block_in_env_with_telemetry(module, block, env).map(|(v, _)| v)
}

/// Like `run_block_in_env` but returns peak evaluation depth for telemetry (development/validation).
pub fn run_block_in_env_with_telemetry(
    module: &Module,
    block: &Block,
    env: &mut Env,
) -> Result<(Value, usize), EvalError> {
    let mut rt = runtime::Runtime::new(env, module);
    let out = match eval::eval_block_in_env(block, &mut rt) {
        Ok(EvalOut::Return(v)) => Ok(v),
        Ok(_) => Ok(Value::Tupu),
        Err(e) => Err(e),
    };
    out.map(|v| (v, rt.peak_depth()))
}

/// Run a single function by name with the given arguments. Used for both `kuu` and tests.
pub fn run_function(
    module: &Module,
    func_name: &str,
    args: Vec<Value>,
) -> Result<Value, EvalError> {
    run_function_with_telemetry(module, func_name, args).map(|(v, _)| v)
}

/// Like `run_function` but returns peak evaluation depth for telemetry (development/validation).
pub fn run_function_with_telemetry(
    module: &Module,
    func_name: &str,
    args: Vec<Value>,
) -> Result<(Value, usize), EvalError> {
    let f = module
        .functions
        .iter()
        .find(|x| x.name == func_name)
        .ok_or_else(|| EvalError::UndefinedVar(func_name.to_string()))?;
    if f.params.len() != args.len() {
        return Err(EvalError::TypeErr(format!(
            "kazi {} inahitaji hoja {}",
            func_name,
            f.params.len()
        )));
    }
    let mut env = Env::new();
    env.seed_global_constants();
    let mut rt = runtime::Runtime::new(&mut env, module);
    rt.env.push_scope();
    for (i, p) in f.params.iter().enumerate() {
        let val = args.get(i).cloned().unwrap_or(Value::Hamna);
        rt.env.define(&p.name, val);
    }
    let out = eval::eval_block_impl(&f.body, &mut rt);
    rt.env.pop_scope();
    match out {
        Ok(EvalOut::Return(v)) => Ok((v, rt.peak_depth())),
        Ok(_) => Ok((Value::Tupu, rt.peak_depth())),
        Err(e) => Err(e),
    }
}

/// Run `kuu` with CLI args as `hoja: Orodha<Neno>`.
pub fn run_main(module: &Module, args: Vec<String>) -> Result<(), EvalError> {
    let hoja = Value::Orodha(args.into_iter().map(Value::Neno).collect());
    run_function(module, "kuu", vec![hoja]).map(|_| ())
}

/// Run a single test function (no args). Returns pass/fail from actual execution.
pub fn run_test_with_module(module: &Module, function: &Function) -> TestResult {
    match run_function(module, &function.name, vec![]) {
        Ok(_) => TestResult {
            name: function.name.clone(),
            passed: true,
            message: "sawa".to_string(),
        },
        Err(EvalError::Panic(msg)) => TestResult {
            name: function.name.clone(),
            passed: false,
            message: msg,
        },
        Err(e) => TestResult {
            name: function.name.clone(),
            passed: false,
            message: e.to_string(),
        },
    }
}

/// Execute tests by running each function. Each test is tied to its module.
pub fn execute_tests(
    modules_and_tests: &[(Module, Function)],
    fail_fast: bool,
) -> Vec<TestResult> {
    let mut out = Vec::new();
    for (module, function) in modules_and_tests {
        let result = run_test_with_module(module, function);
        let failed = !result.passed;
        out.push(result);
        if failed && fail_fast {
            break;
        }
    }
    out
}

/// Emit .asb as bytes (header + serialized Module). Use for run-from-.asb.
pub fn emit_asb(module: &Module, source: &str) -> Vec<u8> {
    asb::emit_asb_bytes(module, source)
}
