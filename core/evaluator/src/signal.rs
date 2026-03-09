//! Signal state for mfumo: pending signal flag and handler registry.
//! OS handler only sets an atomic; eval loop dispatches to registered kazi by name.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI32, Ordering};
#[cfg(unix)]
use std::sync::Mutex;

static PENDING_SIGNAL: AtomicI32 = AtomicI32::new(0);

#[cfg(unix)]
static HANDLERS: std::sync::OnceLock<Mutex<HashMap<i32, String>>> = std::sync::OnceLock::new();

#[cfg(unix)]
static INSTALLED: std::sync::OnceLock<Mutex<HashSet<i32>>> = std::sync::OnceLock::new();

#[cfg(unix)]
fn handlers() -> &'static Mutex<HashMap<i32, String>> {
    HANDLERS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(unix)]
fn installed() -> &'static Mutex<HashSet<i32>> {
    INSTALLED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Set pending signal (called from OS signal handler or from builtin for testing).
#[cfg(unix)]
pub fn set_pending(sig_id: i32) {
    PENDING_SIGNAL.store(sig_id, Ordering::Relaxed);
}

#[cfg(not(unix))]
pub fn set_pending(_sig_id: i32) {}

/// Take pending signal id and clear it. Returns 0 if none.
pub fn take_pending() -> i32 {
    PENDING_SIGNAL.swap(0, Ordering::Relaxed)
}

/// Register handler: when signal sig_id is received, dispatch to kazi with this name.
#[cfg(unix)]
pub fn register_handler(sig_id: i32, kazi_name: String) {
    handlers().lock().unwrap().insert(sig_id, kazi_name);
    let mut inst = installed().lock().unwrap();
    if inst.insert(sig_id) {
        let sig = sig_id;
        let _ = unsafe { signal_hook::low_level::register(sig, move || set_pending(sig)) };
    }
}

#[cfg(not(unix))]
pub fn register_handler(_sig_id: i32, _kazi_name: String) {}

/// Get kazi name for signal, if registered.
#[cfg(unix)]
pub fn get_handler(sig_id: i32) -> Option<String> {
    handlers().lock().unwrap().get(&sig_id).cloned()
}

#[cfg(not(unix))]
pub fn get_handler(_sig_id: i32) -> Option<String> {
    None
}

/// Clear handler for signal (rejesha_ishara).
#[cfg(unix)]
pub fn clear_handler(sig_id: i32) {
    handlers().lock().unwrap().remove(&sig_id);
}

#[cfg(not(unix))]
pub fn clear_handler(_sig_id: i32) {}
