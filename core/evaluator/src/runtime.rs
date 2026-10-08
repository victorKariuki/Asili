//! Runtime context for evaluation.

use std::collections::HashMap;

use asili_parser::Module;

use crate::builtins;
use crate::env::Env;

/// Deepest chain of `kazi` calls a program may make, on every engine (the tree-walker and native
/// code count it the same way); one more fails with `undani mno`.
pub(crate) const MAX_CALL_DEPTH: usize = 10_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvalMetrics {
    pub statements: u64,
    pub expressions: u64,
    pub index_reads: u64,
    pub method_calls: u64,
    pub function_calls: u64,
}

pub(crate) struct Runtime<'a> {
    pub env: &'a mut Env,
    pub module: &'a Module,
    pub builtins: Builtins,
    pub depth: usize,
    /// `kazi` calls in progress (bounded by [`MAX_CALL_DEPTH`]).
    pub calls: usize,
    /// The error now leaving `kazi` was already traced (by the first one it left).
    pub error_traced: bool,
    /// Highest depth reached during this run; for telemetry in development.
    pub peak_depth: usize,
    pub metrics: Option<EvalMetrics>,
}

/// The tree-walker's builtins, by interned name.
pub(crate) type Builtins = asili_parser::FxHashMap<asili_parser::Name, builtins::BuiltinFn>;

/// `builtins` without the names `module` defines as its own `kazi`: a program's `kazi` shadows
/// an ambient builtin of the same name (as the analyzer and the bytecode compiler assume).
pub(crate) fn unshadowed(
    mut builtins: HashMap<String, builtins::BuiltinFn>,
    module: &Module,
) -> Builtins {
    for f in &module.functions {
        builtins.remove(f.name.as_str());
    }
    builtins
        .into_iter()
        .map(|(name, f)| (asili_parser::Name::new(&name), f))
        .collect()
}

impl<'a> Runtime<'a> {
    pub fn new(env: &'a mut Env, module: &'a Module) -> Self {
        Self {
            env,
            module,
            builtins: unshadowed(builtins::builtins(), module),
            depth: 0,
            calls: 0,
            error_traced: false,
            peak_depth: 0,
            metrics: None,
        }
    }

    pub fn with_builtins(env: &'a mut Env, module: &'a Module, builtins: Builtins) -> Self {
        Self {
            env,
            module,
            builtins,
            depth: 0,
            calls: 0,
            error_traced: false,
            peak_depth: 0,
            metrics: None,
        }
    }

    pub fn enable_metrics(&mut self) {
        self.metrics = Some(EvalMetrics::default());
    }

    pub fn metrics(&self) -> Option<&EvalMetrics> {
        self.metrics.as_ref()
    }

    pub fn count_statement(&mut self) {
        if let Some(metrics) = &mut self.metrics {
            metrics.statements += 1;
        }
    }

    pub fn count_expression(&mut self) {
        if let Some(metrics) = &mut self.metrics {
            metrics.expressions += 1;
        }
    }

    pub fn count_index_read(&mut self) {
        if let Some(metrics) = &mut self.metrics {
            metrics.index_reads += 1;
        }
    }

    pub fn count_method_call(&mut self) {
        if let Some(metrics) = &mut self.metrics {
            metrics.method_calls += 1;
        }
    }

    pub fn count_function_call(&mut self) {
        if let Some(metrics) = &mut self.metrics {
            metrics.function_calls += 1;
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
