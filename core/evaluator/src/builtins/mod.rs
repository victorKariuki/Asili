//! Builtin function registry: msingi, mfumo, majira, matumizi, faili, hisabati, runtime.

mod faili;
mod hisabati;
pub(crate) mod http;
mod json;
mod kasha_gc;
mod kiungo;
mod kumbukumbu;
mod majira;
mod matumizi;
mod mfumo;
pub(crate) mod mkondo;
mod msingi;
mod seti;
// PHASE II: neno module is reserved for string-specific methods (gawanya, badilisha, anza_na, maliza_na, etc.)
// Currently, basic string output (chapisha, onyo, makosa, paparika) are in matumizi instead.
#[allow(dead_code)]
mod neno;
mod runtime;
pub(crate) mod sambamba;
mod syscall;

use std::collections::HashMap;

use crate::value::{EvalError, Value};

pub type BuiltinFn = Box<dyn Fn(&[Value]) -> Result<Value, EvalError>>;

/// Builtins that need the running `Module` (they spawn named `kazi` on other threads), so they
/// are dispatched by the evaluator itself rather than through the plain builtin table, and the
/// bytecode VM (which carries no `Module`) leaves programs using them to the evaluator.
pub const MODULE_BUILTINS: &[&str] = &["tenda", "mkondo_tumikia", "mkondo_tumikia_http"];

/// Every builtin module's registrations: the single list of what exists.
fn register_all(m: &mut HashMap<String, BuiltinFn>) {
    msingi::register(m);
    mfumo::register(m);
    majira::register(m);
    matumizi::register(m);
    faili::register(m);
    hisabati::register(m);
    kiungo::register(m);
    runtime::register(m);
    sambamba::register(m);
    syscall::register(m);
    kasha_gc::register(m);
    mkondo::register(m);
    kumbukumbu::register(m);
    seti::register(m);
    json::register(m);
}

/// Builtins by name (the tree-walking evaluator's lookup table).
pub fn builtins() -> HashMap<String, BuiltinFn> {
    let mut m: HashMap<String, BuiltinFn> = HashMap::new();
    register_all(&mut m);
    m
}

/// Builtin names in their stable index order (`chapisha` first, then alphabetical); bytecode
/// refers to builtins by position in this list.
pub fn builtin_names() -> Vec<String> {
    let mut names: Vec<String> = builtins().into_keys().collect();
    names.sort_by(|a, b| (a != "chapisha", a).cmp(&(b != "chapisha", b)));
    names
}

/// Builtins indexed as in [`builtin_names`], plus a by-name index (the bytecode VM's table).
pub struct BuiltinTable {
    pub fns: Vec<BuiltinFn>,
    pub index: HashMap<String, usize>,
}

impl BuiltinTable {
    pub fn new() -> Self {
        let mut all = builtins();
        let mut fns = Vec::with_capacity(all.len());
        let mut index = HashMap::with_capacity(all.len());
        for (i, name) in builtin_names().into_iter().enumerate() {
            if let Some(f) = all.remove(&name) {
                fns.push(f);
                index.insert(name, i);
            }
        }
        BuiltinTable { fns, index }
    }
}

impl Default for BuiltinTable {
    fn default() -> Self {
        Self::new()
    }
}
