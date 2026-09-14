//! Asili interpreter and TIR/ASB emission.

mod asb;
pub mod builtins;
mod bytecode;
mod env;
mod eval;
mod platform;
pub mod runtime;
mod signal;
mod tir;
mod value;

pub use asb::{load_asb, load_asb_bytecode, parse_format, AsbLoadError};
pub use bytecode::{run_bytecode, BytecodeProgram};
pub use env::Env;
pub use eval::eval_expr;
pub use tir::{emit_asb_from_tir, lower_to_tir, TypedIrFunction, TypedIrModule, validate_module};
pub use value::{ErrorKind, EvalError, EvalOut, Value};
pub use crate::builtins::BuiltinFn;

use asili_parser::{Block, Function, Module};
use std::collections::HashMap;

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

/// Evaluate each module-level `thabiti` constant and bind it in `rt`'s current scope. Module-level
/// constants (including ones merged in from `leta`-imported modules — see merge_for_eval in
/// pata/cli) were previously type-checked and exported but never actually bound at runtime, so
/// referencing one by name failed with UndefinedVar. Call once per fresh Env, before pushing the
/// function's own scope, so constants act as globals for the rest of execution.
fn seed_module_constants(module: &Module, rt: &mut runtime::Runtime) -> Result<(), EvalError> {
    for c in &module.constants {
        let val = eval::eval_expr_impl(&c.value, rt)?;
        rt.env.define(&c.name, val);
    }
    Ok(())
}

/// Run a single function with custom builtins (for testing).
pub fn run_function_with_builtins(
    module: &Module,
    func_name: &str,
    args: Vec<Value>,
    builtins: HashMap<String, BuiltinFn>,
) -> Result<Value, EvalError> {
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
    let mut rt = runtime::Runtime::with_builtins(&mut env, module, builtins);
    seed_module_constants(module, &mut rt)?;
    rt.env.push_scope();
    for (i, p) in f.params.iter().enumerate() {
        let val = args.get(i).cloned().unwrap_or(Value::Hamna);
        rt.env.define(&p.name, val);
    }
    let out = eval::eval_block_impl(&f.body, &mut rt);
    rt.env.pop_scope();
    match out {
        Ok(EvalOut::Return(v)) => Ok(v),
        Ok(_) => Ok(Value::Tupu),
        Err(e) => Err(e),
    }
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
    seed_module_constants(module, &mut rt)?;
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
    execute_tests_with_timeout(modules_and_tests, fail_fast, None)
}

/// Like `execute_tests`, but with an optional per-test wall-clock timeout. Each test that has
/// one runs on its own spawned thread, joined with `recv_timeout` — the standard technique for
/// imposing a timeout on a function with no internal cancellation hook, since the evaluator
/// itself has no cooperative-interrupt mechanism (no bytecode-level "check for cancellation"
/// point, no async runtime to abort a task on). A test that actually times out is reported as a
/// failure (`message` says so explicitly), but its thread is **not** forcibly killed — safe Rust
/// has no thread-cancellation API — it keeps running in the background until it finishes or the
/// process exits. This is the same tradeoff most language test runners with wall-clock timeouts
/// make absent a VM-level interrupt; a genuinely hung test still occupies a thread afterward,
/// it just no longer blocks the rest of the suite from reporting results.
///
/// Runs `#[kabla]`/`#[baada]`-tagged functions in the test's own module around it (see
/// `run_test_with_fixtures`) — every test in this crate's public API that executes tests goes
/// through this one function, so fixture support reaches `pata jaribu`'s sequential, parallel,
/// and timed paths alike without each needing its own copy of the setup/teardown logic.
pub fn execute_tests_with_timeout(
    modules_and_tests: &[(Module, Function)],
    fail_fast: bool,
    timeout: Option<std::time::Duration>,
) -> Vec<TestResult> {
    let mut out = Vec::new();
    for (module, function) in modules_and_tests {
        let result = run_test_with_fixtures(module, function, timeout);
        let failed = !result.passed;
        out.push(result);
        if failed && fail_fast {
            break;
        }
    }
    out
}

/// Run one test, calling every `#[kabla]`-tagged function in its module immediately before and
/// every `#[baada]`-tagged one immediately after — module-scoped (every `#[kabla]`/`#[baada]`
/// in the same file runs around every test in that file), not per-project or per-test-function,
/// matching the natural grouping `modules_and_tests` already uses (one entry per (module, test)
/// pair, where `module` is that source file's merged module).
///
/// Fixture failures are real failures, reported distinctly from the test's own assertion
/// failing, so a broken `#[kabla]` doesn't get misread as the test itself being wrong:
///   - A `#[kabla]` panic/error fails the test with a message identifying the fixture by name,
///     and the test body itself never runs.
///   - A `#[baada]` panic/error fails the test (even if the test body itself passed) with a
///     message identifying the fixture by name — a teardown that can't clean up after itself is
///     a real problem the suite should surface, not silently swallow.
/// Every `#[baada]` still runs even if an earlier one in the same module panics, and even if the
/// test body itself failed — teardown functions exist to release resources acquired by setup,
/// and one broken teardown shouldn't prevent the others from having a chance to run.
pub fn run_test_with_fixtures(module: &Module, function: &Function, timeout: Option<std::time::Duration>) -> TestResult {
    let setup_fns: Vec<&Function> = module.functions.iter()
        .filter(|f| f.attrs.iter().any(|a| a.name == "kabla"))
        .collect();
    let teardown_fns: Vec<&Function> = module.functions.iter()
        .filter(|f| f.attrs.iter().any(|a| a.name == "baada"))
        .collect();

    for setup in &setup_fns {
        if let Err(e) = run_function(module, &setup.name, vec![]) {
            return TestResult {
                name: function.name.clone(),
                passed: false,
                message: format!("kabla '{}' imeshindwa: {}", setup.name, fixture_error_message(e)),
            };
        }
    }

    let mut result = match timeout {
        Some(d) => run_test_with_timeout(module, function, d),
        None => run_test_with_module(module, function),
    };

    for teardown in &teardown_fns {
        if let Err(e) = run_function(module, &teardown.name, vec![]) {
            // Only overwrite the reported failure reason if the test itself had actually
            // passed — a test that already failed on its own keeps its own failure message,
            // since that's almost certainly the more useful signal, but a teardown failure
            // after an otherwise-passing test is real and must not be silently dropped.
            if result.passed {
                result = TestResult {
                    name: function.name.clone(),
                    passed: false,
                    message: format!("baada '{}' imeshindwa: {}", teardown.name, fixture_error_message(e)),
                };
            }
        }
    }

    result
}

fn fixture_error_message(e: EvalError) -> String {
    match e {
        EvalError::Panic(msg) => msg,
        other => other.to_string(),
    }
}

/// Run one test with a wall-clock timeout, on a dedicated thread. See
/// `execute_tests_with_timeout`'s doc comment for what happens on an actual timeout (the thread
/// is not killed, only no longer waited on).
fn run_test_with_timeout(module: &Module, function: &Function, timeout: std::time::Duration) -> TestResult {
    let test_name = function.name.clone();
    let module_for_thread = module.clone();
    let function_for_thread = function.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let spawned = std::thread::Builder::new()
        .name(format!("pata-jaribio-{test_name}"))
        .spawn(move || {
            let result = run_test_with_module(&module_for_thread, &function_for_thread);
            let _ = tx.send(result);
        });

    if spawned.is_err() {
        // Thread spawn failure (e.g. resource exhaustion) is a real, honest error — not a test
        // failure to fall back on silently. The caller (execute_tests_with_timeout) still
        // records it as a failed TestResult so the rest of the suite keeps running, but the
        // message makes clear this isn't the test's own assertion failing.
        return TestResult {
            name: test_name,
            passed: false,
            message: "imeshindwa kuanzisha uzi wa jaribio (rasilimali za mfumo)".to_string(),
        };
    }

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => TestResult {
            name: test_name,
            passed: false,
            message: format!("muda umekwisha baada ya {:?} (jaribio limeachwa likiendelea kwa nyuma)", timeout),
        },
    }
}

/// Emit .asb as bytes (header + serialized Module). Use for run-from-.asb.
pub fn emit_asb(module: &Module, source: &str) -> Vec<u8> {
    asb::emit_asb_bytes(module, source)
}
