//! Runtime context for evaluation.

use std::collections::HashMap;

use asili_parser::Module;

use crate::builtins;
use crate::env::Env;

// MAX_EVAL_DEPTH limits evaluation depth to prevent stack overflow.
// NOTE: This counter includes both block nesting and expression depth, not just function call frames.
// A proper implementation would separate call-depth from block-nesting-depth.
// Set to 500 to accommodate typical nested control flow (if/while/for chains) while still catching infinite recursion.
pub(crate) const MAX_EVAL_DEPTH: usize = 500;

pub(crate) struct Runtime<'a> {
    pub env: &'a mut Env,
    pub module: &'a Module,
    pub builtins: HashMap<String, builtins::BuiltinFn>,
    pub depth: usize,
    /// Highest depth reached during this run; for telemetry in development.
    pub peak_depth: usize,
}

impl<'a> Runtime<'a> {
    pub fn new(env: &'a mut Env, module: &'a Module) -> Self {
        Self {
            env,
            module,
            builtins: builtins::builtins(),
            depth: 0,
            peak_depth: 0,
        }
    }

    pub fn with_builtins(env: &'a mut Env, module: &'a Module, builtins: HashMap<String, builtins::BuiltinFn>) -> Self {
        Self {
            env,
            module,
            builtins,
            depth: 0,
            peak_depth: 0,
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
