//! Asili's runtime: programs are lowered to bytecode ([`compile_module`]) and run as native
//! code built by Asili's own backend ([`nguvu`]: machine code, or a wasm module in the browser),
//! whose host ([`host`]) runs what the machine code hands back on the shared value semantics
//! (`eval`). Also builtins, `.asb` artifacts and the test runner.

pub mod alloc;
pub mod aot;
mod asb;
pub mod builtins;
pub mod bundle;
mod bytecode;
mod bytecode_verify;
mod compiled;
pub mod debug_hook;
mod env;
mod eval;
mod hali_salama;
mod host;
mod native;
pub mod nguvu;
mod numlist;
mod platform;
mod repl;
pub mod salama;
mod scalars;
mod signal;
mod spawn;
mod sync;
mod tir;
mod value;

pub use crate::builtins::BuiltinFn;
pub use asb::{artifact_formats, load_asb_bytecode, parse_format, AsbLoadError};
pub use bytecode::run_bytecode_native;
pub use bytecode::{
    compile_module, compile_module_explained, compile_module_with, run_bytecode,
    run_bytecode_function, run_bytecode_function_on, BytecodeProgram, CompileOptions, Opcode,
};
pub use compiled::NativeProgram;
pub use repl::ReplSession;
pub use tir::{emit_asb_from_tir, lower_to_tir, validate_module, TypedIrFunction, TypedIrModule};
pub use value::{ErrorKind, EvalError, Value};

use asili_parser::{Function, Module};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

/// Run `module`'s function `func_name` with `args` as native code (built for this one call).
pub fn run_function(
    module: &Module,
    func_name: &str,
    args: Vec<Value>,
) -> Result<Value, EvalError> {
    NativeProgram::build(module)?.call(func_name, args)
}

/// The arguments `kuu` takes: `hoja: Orodha<Neno>` when it declares a parameter.
fn main_args(module: &Module, args: Vec<String>) -> Vec<Value> {
    let takes_args = module
        .functions
        .iter()
        .any(|f| f.name == "kuu" && !f.params.is_empty());
    if takes_args {
        vec![Value::list(args.into_iter().map(Value::neno).collect())]
    } else {
        Vec::new()
    }
}

/// Run `kuu` with CLI args as `hoja: Orodha<Neno>`, as native code.
pub fn run_main(module: &Module, args: Vec<String>) -> Result<(), EvalError> {
    NativeProgram::build(module)?
        .call("kuu", main_args(module, args))
        .map(|_| ())
}

/// Like `run_main`, but with a real debugger (`pata-dap`'s `DapSession`, driving a
/// `debug_hook::RealDebugHook`) attached: the host snapshots bindings into `hook` and calls
/// `hook.should_pause(line)` before every statement, genuinely pausing this thread at a
/// configured breakpoint until the debugger resumes it.
pub fn run_main_with_debug_hook(
    module: &Module,
    args: Vec<String>,
    hook: std::sync::Arc<dyn debug_hook::DebugHook>,
) -> Result<(), EvalError> {
    let options = bytecode::CompileOptions {
        lines: true,
        bindings: true,
    };
    NativeProgram::build_with(module, options)?
        .call_with_debugger("kuu", main_args(module, args), hook)
        .map(|_| ())
}

fn test_result(function: &Function, result: Result<Value, EvalError>) -> TestResult {
    let (passed, message) = match result {
        Ok(_) => (true, "sawa".to_string()),
        Err(EvalError::Panic(msg)) => (false, msg),
        Err(e) => (false, e.to_string()),
    };
    TestResult {
        name: function.name.to_string(),
        passed,
        message,
    }
}

/// Run a single test function (no args) as native code. Returns pass/fail from actual
/// execution; a module that cannot be built to native code fails the test with the reason.
pub fn run_test_with_module(module: &Module, function: &Function) -> TestResult {
    match NativeProgram::build(module) {
        Ok(program) => test_result(function, program.call(&function.name, vec![])),
        Err(e) => test_result(function, Err(e)),
    }
}

/// Like `run_test_with_module`, but also returns the set of source lines actually executed
/// while running this one test — real line-level coverage from a build that marks every
/// statement (`Opcode::Line`), not a function-name presence check.
/// A test that panics still reports whatever lines ran before the panic, since partial coverage
/// from a failing test is real coverage, not nothing.
pub fn run_test_with_coverage(
    module: &Module,
    function: &Function,
) -> (TestResult, std::collections::HashSet<usize>) {
    let options = bytecode::CompileOptions {
        lines: true,
        ..Default::default()
    };
    match NativeProgram::build_with(module, options) {
        Ok(program) => {
            let (result, lines) = program.call_with_coverage(&function.name, vec![]);
            (test_result(function, result), lines)
        }
        Err(e) => (test_result(function, Err(e)), Default::default()),
    }
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
    let program = match NativeProgram::build(module) {
        Ok(program) => program,
        Err(e) => return test_result(function, Err(e)),
    };
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
        if let Err(e) = program.call(&setup.name, vec![]) {
            return TestResult {
                name: function.name.to_string(),
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
        Some(d) => run_test_with_timeout(&program, function, d),
        None => test_result(function, program.call(&function.name, vec![])),
    };

    for teardown in &teardown_fns {
        if let Err(e) = program.call(&teardown.name, vec![]) {
            // Only overwrite the reported failure reason if the test itself had actually
            // passed — a test that already failed on its own keeps its own failure message,
            // since that's almost certainly the more useful signal, but a teardown failure
            // after an otherwise-passing test is real and must not be silently dropped.
            if result.passed {
                result = TestResult {
                    name: function.name.to_string(),
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
    program: &NativeProgram,
    function: &Function,
    timeout: std::time::Duration,
) -> TestResult {
    let test_name = function.name;
    let program_for_thread = program.clone();
    let function_for_thread = function.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let spawned = std::thread::Builder::new()
        .name(format!("pata-jaribio-{test_name}"))
        .spawn(move || {
            let result = test_result(
                &function_for_thread,
                program_for_thread.call(&function_for_thread.name, vec![]),
            );
            let _ = tx.send(result);
        });

    if spawned.is_err() {
        // Thread spawn failure (e.g. resource exhaustion) is a real, honest error — not a test
        // failure to fall back on silently. The caller (execute_tests_with_timeout) still
        // records it as a failed TestResult so the rest of the suite keeps running, but the
        // message makes clear this isn't the test's own assertion failing.
        return TestResult {
            name: test_name.to_string(),
            passed: false,
            message: "imeshindwa kuanzisha uzi wa jaribio (rasilimali za mfumo)".to_string(),
        };
    }

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => TestResult {
            name: test_name.to_string(),
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

/// Run an `.asb` artifact's `kuu` as native code: the image `pata jenga` built next to it
/// (`<name>.nguvu`) when it exists and was built from exactly this bytecode for this machine,
/// else compiled in memory. An artifact holding only a syntax tree (from an older `pata`) must
/// be rebuilt.
pub fn run_asb(
    bytes: &[u8],
    asb_path: Option<&std::path::Path>,
    args: Vec<String>,
) -> Result<(), RunAsbError> {
    #[cfg(not(target_arch = "wasm32"))]
    let _output = platform::BlockOutput::begin();
    on_known_stack(|| run_asb_here(bytes, Image::Beside(asb_path), args))
}

/// Run a program carried inside the running executable ([`bundle`]).
pub fn run_bundle(bundle: &bundle::Bundle, args: Vec<String>) -> Result<(), RunAsbError> {
    #[cfg(not(target_arch = "wasm32"))]
    let _output = platform::BlockOutput::begin();
    on_known_stack(|| run_asb_here(&bundle.asb, Image::Bytes(bundle.image.as_deref()), args))
}

/// Where a bytecode artifact's native image comes from.
enum Image<'a> {
    /// `<name>.nguvu` beside the artifact at this path.
    Beside(Option<&'a std::path::Path>),
    /// Carried with it (a standalone executable).
    Bytes(Option<&'a [u8]>),
}

/// Run `f` on a stack whose size `stacker` knows. Native calls grow the stack on demand
/// (`stacker::maybe_grow` in the host); where the remaining stack is unknown — musl's main
/// thread reports only its committed pages — every call near the edge would map, and on return
/// unmap, a fresh segment (800,000 times for `fib(32)`). One large, lazily committed segment up
/// front avoids that.
fn on_known_stack<R>(f: impl FnOnce() -> R) -> R {
    #[cfg(not(target_arch = "wasm32"))]
    {
        const SEGMENT: usize = 64 << 20;
        if stacker::remaining_stack().is_some_and(|r| r >= SEGMENT / 2) {
            return f();
        }
        stacker::grow(SEGMENT, f)
    }
    #[cfg(target_arch = "wasm32")]
    f()
}

fn run_asb_here(bytes: &[u8], source: Image<'_>, args: Vec<String>) -> Result<(), RunAsbError> {
    if parse_format(bytes).as_deref() != Some("bytecode") {
        return Err(RunAsbError::Load(
            "kilele hiki kina mti wa programu tu (cha pata ya zamani): kijenge upya kwa `pata jenga`"
                .into(),
        ));
    }
    let program = load_asb_bytecode(bytes).map_err(|e| RunAsbError::Load(e.to_string()))?;
    // Native code beside the artifact: the machine-code image `pata jenga` wrote
    // (`<name>.nguvu`). When it is missing, stale or for another machine, native code is
    // compiled in memory (`run_shared_program`).
    let image = || match source {
        Image::Beside(path) => {
            let path = path?;
            let file = path.with_file_name(nguvu::image_file_name(path.file_stem()?.to_str()?));
            file.is_file()
                .then(|| nguvu::load_image(&file, &program).ok())
                .flatten()
        }
        Image::Bytes(bytes) => nguvu::load_image_bytes(bytes?, &program).ok(),
    };
    let library = if nguvu::supported() { image() } else { None };
    bytecode::run_shared_program(
        std::sync::Arc::new(program),
        library.map(std::sync::Arc::new),
        args,
    )
    .map_err(RunAsbError::Run)
}

/// The `.asb` for a program: its bytecode (run as native code), or the `kazi` and line that
/// could not be lowered.
pub fn emit_asb(module: &Module, source: &str) -> Result<Vec<u8>, String> {
    bytecode::compile_module_explained(module)
        .map(|program| asb::emit_bytecode_bytes(&program, source))
}
