//! The program as other threads receive it: `tenda` and the server workers
//! (`mkondo_tumikia`, `mkondo_tumikia_http`) run the program's `kazi` on threads of their own.
//! A bytecode program and its native code are plain shared data, so those threads run the same
//! machine code as the thread that started them; each thread builds its host once and reuses it
//! for every call.

// Runtime code never panics on its own: an impossible state is an error the program sees
// (and its safe state handles), not a crash (see docs/design/safety-critical-roadmap.md §3).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

use std::sync::Arc;

use crate::bytecode::BytecodeProgram;
use crate::value::{EvalError, Value};

/// A program shared between threads.
#[derive(Clone)]
pub(crate) enum Shared {
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
            Shared::Code { program, .. } => program.find_function(name).is_some(),
        }
    }

    /// Run `body` with a caller for this program's `kazi` on the current thread. The host (its
    /// frame pool, builtins and module constants) is built once here and serves every call
    /// `body` makes.
    pub(crate) fn with_caller<R>(&self, body: impl FnOnce(&mut Caller<'_>) -> R) -> R {
        match self {
            Shared::Code { program, native } => {
                let mut host = crate::host::Host::new(program, native, Some(self.clone()));
                body(&mut |name: &str, args: Vec<Value>| host.call_by_name(name, args))
            }
        }
    }
}
