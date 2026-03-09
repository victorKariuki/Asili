//! Runtime context for evaluation.

use std::collections::HashMap;

use asili_parser::Module;

use crate::builtins;
use crate::env::Env;

pub(crate) const MAX_EVAL_DEPTH: usize = 100;

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
