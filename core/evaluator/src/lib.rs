//! Asili interpreter and TIR/ASB emission.

pub mod alloc;
#[cfg(not(target_arch = "wasm32"))]
pub mod aot;
mod asb;
pub mod builtins;
mod bytecode;
pub mod debug_hook;
mod env;
mod eval;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub mod nguvu;
mod numlist;
mod platform;
pub mod runtime;
mod signal;
mod tir;
mod value;

pub use crate::builtins::BuiltinFn;
pub use asb::{load_asb, load_asb_bytecode, parse_format, AsbLoadError};
#[cfg(not(target_arch = "wasm32"))]
pub use bytecode::run_bytecode_native;
pub use bytecode::{
    compile_module, compile_module_explained, run_bytecode, run_bytecode_function,
    run_bytecode_function_on, BytecodeProgram, Engine, Opcode,
};
pub use env::Env;
pub use eval::eval_expr;
pub use runtime::EvalMetrics;
pub use tir::{emit_asb_from_tir, lower_to_tir, validate_module, TypedIrFunction, TypedIrModule};
pub use value::{ErrorKind, EvalError, EvalOut, Value};

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
pub fn run_block(module: &Module, block: &Block, env: &mut Env) -> Result<Value, EvalError> {
    let mut rt = runtime::Runtime::new(env, module);
    match eval::eval_block_impl(block, &mut rt) {
        Ok(EvalOut::Return(v)) => Ok(v),
        Ok(_) => Ok(Value::Tupu),
        Err(e) => Err(e),
    }
}

/// Run a block in the current env without a new scope. Use for REPL so that `weka` bindings persist.
pub fn run_block_in_env(module: &Module, block: &Block, env: &mut Env) -> Result<Value, EvalError> {
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

/// Find `func_name` in `module` and check it takes exactly `argc` arguments.
fn find_function<'m>(
    module: &'m Module,
    func_name: &str,
    argc: usize,
) -> Result<&'m Function, EvalError> {
    let f = module
        .functions
        .iter()
        .find(|x| x.name == func_name)
        .ok_or_else(|| EvalError::UndefinedVar(func_name.to_string()))?;
    if f.params.len() != argc {
        return Err(EvalError::TypeErr(format!(
            "kazi {} inahitaji hoja {}",
            func_name,
            f.params.len()
        )));
    }
    Ok(f)
}

/// The one tree-walker entry path every `run_*` function uses: a fresh `Env` with the global
/// and module constants, `f`'s parameters bound to `args`, `f`'s body evaluated. `setup`
/// configures the runtime first (metrics, coverage, a debugger); `report` reads it afterwards,
/// whether or not the call failed.
fn run_in_fresh_runtime<T>(
    module: &Module,
    f: &Function,
    args: Vec<Value>,
    builtins: Option<HashMap<String, BuiltinFn>>,
    setup: impl FnOnce(&mut runtime::Runtime),
    report: impl FnOnce(runtime::Runtime) -> T,
) -> (Result<Value, EvalError>, T) {
    let mut env = Env::new();
    env.seed_global_constants();
    let mut rt = match builtins {
        Some(b) => runtime::Runtime::with_builtins(&mut env, module, b),
        None => runtime::Runtime::new(&mut env, module),
    };
    setup(&mut rt);
    let result = seed_module_constants(module, &mut rt).and_then(|()| {
        rt.env.push_scope();
        for (p, val) in f.params.iter().zip(args) {
            rt.env.define(&p.name, val);
        }
        let out = eval::eval_block_impl(&f.body, &mut rt);
        rt.env.pop_scope();
        match out? {
            EvalOut::Return(v) => Ok(v),
            _ => Ok(Value::Tupu),
        }
    });
    (result, report(rt))
}

/// Run a single function with custom builtins (for testing).
pub fn run_function_with_builtins(
    module: &Module,
    func_name: &str,
    args: Vec<Value>,
    builtins: HashMap<String, BuiltinFn>,
) -> Result<Value, EvalError> {
    let f = find_function(module, func_name, args.len())?;
    run_in_fresh_runtime(module, f, args, Some(builtins), |_| {}, |_| ()).0
}

/// Like `run_function` but returns peak evaluation depth for telemetry (development/validation).
pub fn run_function_with_telemetry(
    module: &Module,
    func_name: &str,
    args: Vec<Value>,
) -> Result<(Value, usize), EvalError> {
    let f = find_function(module, func_name, args.len())?;
    let (result, depth) = run_in_fresh_runtime(module, f, args, None, |_| {}, |rt| rt.peak_depth());
    result.map(|v| (v, depth))
}

pub fn run_function_with_metrics(
    module: &Module,
    func_name: &str,
    args: Vec<Value>,
) -> Result<(Value, EvalMetrics), EvalError> {
    let f = find_function(module, func_name, args.len())?;
    let (result, metrics) = run_in_fresh_runtime(
        module,
        f,
        args,
        None,
        |rt| rt.enable_metrics(),
        |rt| rt.metrics().cloned().unwrap_or_default(),
    );
    result.map(|v| (v, metrics))
}

/// Run `kuu` with CLI args as `hoja: Orodha<Neno>`.
pub fn run_main(module: &Module, args: Vec<String>) -> Result<(), EvalError> {
    let hoja = Value::Orodha(args.into_iter().map(Value::Neno).collect());
    run_function(module, "kuu", vec![hoja]).map(|_| ())
}

/// Like `run_main`, but with a real debugger (`pata-dap`'s `DapSession`, driving a
/// `debug_hook::RealDebugHook`) attached: `eval_stmt_impl` will snapshot bindings into `hook`
/// and call `hook.should_pause(line)` before every statement, genuinely pausing this thread at a
/// configured breakpoint until the debugger resumes it.
pub fn run_main_with_debug_hook(
    module: &Module,
    args: Vec<String>,
    hook: std::sync::Arc<dyn debug_hook::DebugHook>,
) -> Result<(), EvalError> {
    let hoja = Value::Orodha(args.into_iter().map(Value::Neno).collect());
    let f = module
        .functions
        .iter()
        .find(|x| x.name == "kuu")
        .ok_or_else(|| EvalError::UndefinedVar("kuu".to_string()))?;
    let args = if f.params.is_empty() {
        vec![]
    } else {
        vec![hoja]
    };
    run_in_fresh_runtime(
        module,
        f,
        args,
        None,
        |rt| rt.enable_debug_hook(hook),
        |_| (),
    )
    .0
    .map(|_| ())
}

fn test_result(function: &Function, result: Result<Value, EvalError>) -> TestResult {
    let (passed, message) = match result {
        Ok(_) => (true, "sawa".to_string()),
        Err(EvalError::Panic(msg)) => (false, msg),
        Err(e) => (false, e.to_string()),
    };
    TestResult {
        name: function.name.clone(),
        passed,
        message,
    }
}

/// Run a single test function (no args). Returns pass/fail from actual execution.
pub fn run_test_with_module(module: &Module, function: &Function) -> TestResult {
    test_result(function, run_function(module, &function.name, vec![]))
}

/// Like `run_test_with_module`, but also returns the set of source lines actually executed
/// while running this one test — real line-level coverage from `Runtime::executed_lines`
/// (`Stmt::line()` recorded on every statement evaluated), not a function-name presence check.
/// A test that panics still reports whatever lines ran before the panic, since partial coverage
/// from a failing test is real coverage, not nothing.
pub fn run_test_with_coverage(
    module: &Module,
    function: &Function,
) -> (TestResult, std::collections::HashSet<usize>) {
    let (result, lines) = run_in_fresh_runtime(
        module,
        function,
        vec![],
        None,
        |rt| rt.enable_coverage(),
        |rt| rt.executed_lines.unwrap_or_default(),
    );
    (test_result(function, result), lines)
}

/// Execute tests by running each function. Each test is tied to its module.
pub fn execute_tests(modules_and_tests: &[(Module, Function)], fail_fast: bool) -> Vec<TestResult> {
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
pub fn run_test_with_fixtures(
    module: &Module,
    function: &Function,
    timeout: Option<std::time::Duration>,
) -> TestResult {
    let setup_fns: Vec<&Function> = module
        .functions
        .iter()
        .filter(|f| f.attrs.iter().any(|a| a.name == "kabla"))
        .collect();
    let teardown_fns: Vec<&Function> = module
        .functions
        .iter()
        .filter(|f| f.attrs.iter().any(|a| a.name == "baada"))
        .collect();

    for setup in &setup_fns {
        if let Err(e) = run_function(module, &setup.name, vec![]) {
            return TestResult {
                name: function.name.clone(),
                passed: false,
                message: format!(
                    "kabla '{}' imeshindwa: {}",
                    setup.name,
                    fixture_error_message(e)
                ),
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
                    message: format!(
                        "baada '{}' imeshindwa: {}",
                        teardown.name,
                        fixture_error_message(e)
                    ),
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
fn run_test_with_timeout(
    module: &Module,
    function: &Function,
    timeout: std::time::Duration,
) -> TestResult {
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
            message: format!(
                "muda umekwisha baada ya {:?} (jaribio limeachwa likiendelea kwa nyuma)",
                timeout
            ),
        },
    }
}

/// Emit .asb as bytes.  The Sudoku-compatible subset is lowered to bytecode; unsupported syntax
/// deliberately keeps the serialized-AST artifact and therefore the existing evaluator fallback.
/// Why an `.asb` artifact could not be run.
#[derive(Debug)]
pub enum RunAsbError {
    /// The path is not an `.asb` or `.build.manifest` (exit code 2).
    Usage(String),
    /// The artifact (or its manifest or format) could not be found, read or decoded.
    Load(String),
    /// The program itself failed.
    Run(EvalError),
}

impl RunAsbError {
    /// Process exit code for command-line runners.
    pub fn exit_code(&self) -> i32 {
        match self {
            RunAsbError::Usage(_) => 2,
            RunAsbError::Load(_) | RunAsbError::Run(_) => 1,
        }
    }
}

impl std::fmt::Display for RunAsbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunAsbError::Usage(m) => f.write_str(m),
            RunAsbError::Load(m) => write!(f, "kuipakia asb: {m}"),
            RunAsbError::Run(e) => write!(f, "kuendesha kuu: {e}"),
        }
    }
}

/// The `.asb` a path refers to: the path itself, or the `kilele=` artifact named by a
/// `.build.manifest` (relative to the manifest's directory).
pub fn resolve_artifact(path: &std::path::Path) -> Result<std::path::PathBuf, RunAsbError> {
    if !path.exists() {
        return Err(RunAsbError::Load(format!(
            "faili haipo: {}",
            path.display()
        )));
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.ends_with(".asb") {
        return Ok(path.to_path_buf());
    }
    if !name.ends_with(".build.manifest") {
        return Err(RunAsbError::Usage(format!(
            "tenda inahitaji .asb au .build.manifest, si: {}",
            path.display()
        )));
    }
    let content = std::fs::read_to_string(path).map_err(|e| {
        RunAsbError::Load(format!(
            "imeshindwa kusoma manifest {}: {e}",
            path.display()
        ))
    })?;
    let artifact = content
        .lines()
        .find_map(|l| l.strip_prefix("kilele=").map(str::trim))
        .ok_or_else(|| {
            RunAsbError::Load(format!("manifest {} haina mstari kilele=", path.display()))
        })?;
    let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let asb = dir.join(artifact);
    if !asb.exists() {
        return Err(RunAsbError::Load(format!(
            "kilele haipo: {} (kutoka manifest)",
            asb.display()
        )));
    }
    Ok(asb)
}

/// Resolve, read and run an artifact path (`.asb` or `.build.manifest`): the whole of
/// `pata tenda` and the standalone runner.
pub fn run_artifact(path: &std::path::Path, args: Vec<String>) -> Result<(), RunAsbError> {
    let asb = resolve_artifact(path)?;
    let bytes = std::fs::read(&asb)
        .map_err(|e| RunAsbError::Load(format!("imeshindwa kusoma {}: {e}", asb.display())))?;
    run_asb(&bytes, Some(&asb), args)
}

/// Run an `.asb` artifact's `kuu`. Bytecode artifacts use the ahead-of-time native library
/// `pata jenga` built next to them (`<name>.so`/`.dylib`/`.dll`) when it exists and was built
/// from exactly this bytecode, and otherwise the register VM; serialized-AST artifacts use the
/// tree-walking evaluator.
pub fn run_asb(
    bytes: &[u8],
    asb_path: Option<&std::path::Path>,
    args: Vec<String>,
) -> Result<(), RunAsbError> {
    if parse_format(bytes).as_deref() == Some("bytecode") {
        let program = load_asb_bytecode(bytes).map_err(|e| RunAsbError::Load(e.to_string()))?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native code beside the artifact: the machine-code image `pata jenga` wrote
            // (`<name>.nguvu`). `ASILI_NGUVU=1` compiles in memory when there is none; otherwise
            // anything missing or stale just means running on the VM.
            let image = || {
                let path = asb_path?;
                let file = path.with_file_name(nguvu::image_file_name(path.file_stem()?.to_str()?));
                file.is_file()
                    .then(|| nguvu::load_image(&file, &program).ok())
                    .flatten()
            };
            let in_memory = std::env::var("ASILI_NGUVU").is_ok_and(|v| v == "1");
            let library = if !aot::enabled() || !nguvu::supported() {
                None
            } else if in_memory {
                image().or_else(|| nguvu::compile(&program).ok())
            } else {
                image()
            };
            return run_bytecode_native(&program, library.as_ref(), args).map_err(RunAsbError::Run);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = asb_path;
            return run_bytecode(&program, args).map_err(RunAsbError::Run);
        }
    }
    let module = load_asb(bytes).map_err(|e| RunAsbError::Load(e.to_string()))?;
    run_main(&module, args).map_err(RunAsbError::Run)
}

/// The `.asb` for a program: bytecode (run by the VM and native code) whenever the whole
/// module lowers to it, else the serialized AST for the tree-walker.
pub fn emit_asb(module: &Module, source: &str) -> Vec<u8> {
    match bytecode::compile_module(module) {
        Some(program) => asb::emit_bytecode_bytes(&program, source),
        None => asb::emit_asb_bytes(module, source),
    }
}

/// The `.asb` holding the serialized AST (the tree-walker's artifact), whatever the program.
pub fn emit_asb_ast(module: &Module, source: &str) -> Vec<u8> {
    asb::emit_asb_bytes(module, source)
}

/// Like [`emit_asb`] but bytecode is required (for `pata jenga --namna release`): every program
/// is compiled to bytecode, and when a construct can't be lowered the error names the `kazi` and
/// line that blocked it instead of silently falling back to the tree-walker's AST artifact.
pub fn emit_asb_bytecode(module: &Module, source: &str) -> Result<Vec<u8>, String> {
    bytecode::compile_module_explained(module)
        .map(|program| asb::emit_bytecode_bytes(&program, source))
}
