//! Runtime context for evaluation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use asili_parser::Module;

use crate::builtins;
use crate::debug_hook::DebugHook;
use crate::env::Env;

// MAX_EVAL_DEPTH limits evaluation depth to prevent stack overflow.
// NOTE: This counter includes both block nesting and expression depth, not just function call frames.
// A proper implementation would separate call-depth from block-nesting-depth.
pub(crate) const MAX_EVAL_DEPTH: usize = 1000;

pub(crate) struct Runtime<'a> {
    pub env: &'a mut Env,
    pub module: &'a Module,
    pub builtins: HashMap<String, builtins::BuiltinFn>,
    pub depth: usize,
    /// Highest depth reached during this run; for telemetry in development.
    pub peak_depth: usize,
    /// Source lines of statements actually executed during this run — real line-level coverage,
    /// recorded by `eval_stmt_impl` as each `Stmt` runs (see `Stmt::line`). `None` when coverage
    /// tracking wasn't requested (the common case — every `run_function`/`run_main` call site
    /// that doesn't care about coverage pays no `HashSet` insert cost), `Some` once
    /// `Runtime::with_coverage` opts in.
    pub executed_lines: Option<HashSet<usize>>,
    /// A real debugger attached to this run (a `pata-dap` session driving a `RealDebugHook`) —
    /// `None` for every ordinary run (the common case, zero cost). When `Some`, `eval_stmt_impl`
    /// snapshots the current environment into the hook and calls `should_pause` before executing
    /// each statement, so a configured breakpoint genuinely halts this thread until the debugger
    /// resumes it.
    pub debug_hook: Option<Arc<dyn DebugHook>>,
}

impl<'a> Runtime<'a> {
    pub fn new(env: &'a mut Env, module: &'a Module) -> Self {
        Self {
            env,
            module,
            builtins: builtins::builtins(),
            depth: 0,
            peak_depth: 0,
            executed_lines: None,
            debug_hook: None,
        }
    }

    pub fn with_builtins(env: &'a mut Env, module: &'a Module, builtins: HashMap<String, builtins::BuiltinFn>) -> Self {
        Self {
            env,
            module,
            builtins,
            depth: 0,
            peak_depth: 0,
            executed_lines: None,
            debug_hook: None,
        }
    }

    /// Opt this runtime into line-level coverage tracking.
    pub fn enable_coverage(&mut self) {
        self.executed_lines = Some(HashSet::new());
    }

    /// Attach a real debugger to this runtime — `eval_stmt_impl` will snapshot bindings into
    /// `hook` and call `hook.should_pause(line)` before every statement from here on.
    pub fn enable_debug_hook(&mut self, hook: Arc<dyn DebugHook>) {
        self.debug_hook = Some(hook);
    }

    /// Record that `line` executed, when coverage tracking is enabled — a no-op otherwise, so
    /// callers (`eval_stmt_impl`) can call this unconditionally without checking first.
    pub fn record_line(&mut self, line: usize) {
        if let Some(lines) = &mut self.executed_lines {
            lines.insert(line);
        }
    }

    /// Update peak depth from current depth. Call after incrementing `depth`.
    pub(crate) fn update_peak_depth(&mut self) {
        if self.depth > self.peak_depth {
            self.peak_depth = self.depth;
        }
    }

    /// Peak evaluation/call depth reached during execution (telemetry).
    pub fn peak_depth(&self) -> usize {
        self.peak_depth
    }
}
