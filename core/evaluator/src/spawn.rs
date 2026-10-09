//! The program as other threads receive it: `tenda` and the server workers
//! (`mkondo_tumikia`, `mkondo_tumikia_http`) run the program's `kazi` on threads of their own.
//! A bytecode program and its native code are plain shared data, so those threads run the same
//! machine code as the thread that started them; each thread builds its host once and reuses it
//! for every call.

use std::sync::Arc;

use asili_parser::Module;

use crate::bytecode::BytecodeProgram;
use crate::value::{EvalError, Value};

/// A program shared between threads.
#[derive(Clone)]
pub(crate) enum Shared {
    /// Run on the tree-walker (an AST artifact, or a caller that is itself tree-walking).
    Tree(Arc<Module>),
    /// Run as native code built from this bytecode.
    Code {
        program: Arc<BytecodeProgram>,
        native: Arc<crate::aot::NativeLibrary>,
    },
}

/// Calls a `kazi` by name on the calling thread's engine.
pub(crate) type Caller<'a> = dyn FnMut(&str, Vec<Value>) -> Result<Value, EvalError> + 'a;

impl Shared {
    /// Whether the program has a `kazi` called `name`.
    pub(crate) fn has_kazi(&self, name: &str) -> bool {
        match self {
            Shared::Tree(module) => module.functions.iter().any(|f| f.name == name),
            Shared::Code { program, .. } => program.find_function(name).is_some(),
        }
    }

    /// Run `body` with a caller for this program's `kazi` on the current thread. The engine (a
    /// native-code host with its frame pool, or a tree-walker with its builtins and module
    /// constants) is built once here and serves every call `body` makes.
    pub(crate) fn with_caller<R>(&self, body: impl FnOnce(&mut Caller<'_>) -> R) -> R {
        match self {
            Shared::Tree(module) => {
                let mut tree = crate::TreeContext::new(module).map_err(|e| e.to_string());
                body(&mut |name: &str, args: Vec<Value>| {
                    let tree = tree.as_mut().map_err(|e| EvalError::Unknown(e.clone()))?;
                    let f = module
                        .functions
                        .iter()
                        .find(|f| f.name == name)
                        .ok_or_else(|| EvalError::UndefinedVar(name.to_string()))?;
                    tree.call(module, f, args)
                })
            }
            Shared::Code { program, native } => {
                let mut host = crate::host::Host::new(program, native, Some(self.clone()));
                body(&mut |name: &str, args: Vec<Value>| host.call_by_name(name, args))
            }
        }
    }
}
