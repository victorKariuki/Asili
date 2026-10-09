//! The safe state: a program's `#[hali_salama]` `kazi` (no parameters) runs once, before the
//! program stops, whenever it fails in a way it cannot recover from — an error leaving `kuu`, a
//! fault inside the runtime (a panic), the watchdog (`mlinzi_anza`) expiring, or the memory limit
//! (`kikomo_kumbukumbu`) being passed. It is where a controller puts its outputs into their safe
//! positions. It runs on a fresh host, so it does not depend on the state that failed.

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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Once};

use crate::bytecode::BytecodeProgram;

/// The running program and its native code, when it has a safe state.
static PROGRAM: Mutex<Option<(Arc<BytecodeProgram>, Arc<crate::aot::NativeLibrary>)>> =
    Mutex::new(None);
/// Whether the safe state has run (it runs at most once per registered program).
static ENTERED: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK: Once = Once::new();

/// Make `program` the one whose safe state runs on failure (when it has one).
pub(crate) fn register(program: &Arc<BytecodeProgram>, native: &Arc<crate::aot::NativeLibrary>) {
    if program.safe_state.is_none() {
        return;
    }
    *crate::sync::lock(&PROGRAM) = Some((program.clone(), native.clone()));
    ENTERED.store(false, Ordering::SeqCst);
    // A fault in the runtime enters the safe state before the process stops (in every build:
    // shipped builds abort on panic, so this hook is the last code that runs).
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            enter(&format!("kosa la ndani la Asili: {info}"));
            previous(info);
        }));
    });
}

/// Run the registered program's safe state, once, saying why on stderr.
pub(crate) fn enter(reason: &str) {
    let Some((program, native)) = crate::sync::lock(&PROGRAM).clone() else {
        return;
    };
    let Some(index) = program.safe_state else {
        return;
    };
    if ENTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    eprintln!("hali salama: {reason}");
    // The safe state always gets to run: the memory limit no longer applies.
    crate::alloc::set_limit(0);
    let name = program.functions[index as usize].name.clone();
    let shared = crate::spawn::Shared::Code {
        program: program.clone(),
        native: native.clone(),
    };
    if let Err(e) =
        crate::host::Host::new(&program, &native, Some(shared)).call_by_name(&name, Vec::new())
    {
        eprintln!("hali salama '{name}' imeshindwa: {e}");
    }
    crate::platform::flush_stdout();
}
