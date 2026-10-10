//! Builtin function registry: msingi, mfumo, majira, matumizi, faili, hisabati, runtime.

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

mod faili;
mod hisabati;
pub(crate) mod http;
mod http_mteja;
mod http_thamani;
mod json;
mod kasha_gc;
mod kiungo;
mod kumbukumbu;
mod majira;
mod matumizi;
mod mfumo;
pub(crate) mod mkondo;
mod msingi;
mod runtime;
mod ruwaza;
pub(crate) mod sambamba;
mod seti;
mod syscall;
mod usimbaji;

use std::collections::HashMap;

use crate::value::{EvalError, Value};

pub type BuiltinFn = Box<dyn Fn(&[Value]) -> Result<Value, EvalError>>;

/// Builtins that need the running program (they run its named `kazi` on other threads), so each
/// engine dispatches them itself, passing the program as a `spawn::Shared`, rather than through
/// the plain builtin table. Native code's host hands threads its own bytecode and native code.
pub const MODULE_BUILTINS: [&str; 4] =
    ["tenda", "mkondo_tumikia", "mkondo_tumikia_http", "anzisha"];

/// Every builtin module's registrations: the single list of what exists.
/// Argument `i` as a `Neno` (empty when missing or not text) — the lenient string-argument
/// convention every builtin shares.
pub(crate) fn arg_str(args: &[Value], i: usize) -> String {
    crate::value::as_string(args.get(i).unwrap_or(&Value::Hamna)).unwrap_or_default()
}

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
    http_mteja::register(m);
    ruwaza::register(m);
    usimbaji::register(m);
    // Listed so compilers resolve them; each engine intercepts the call and passes the program
    // (see `MODULE_BUILTINS`), so these bodies run only for a by-name callback.
    for name in MODULE_BUILTINS {
        m.insert(
            name.to_string(),
            Box::new(move |_args: &[Value]| {
                Err(EvalError::TypeErr(format!(
                    "{name} haiwezi kupitishwa kama kazi"
                )))
            }),
        );
    }
}

/// Builtins by name (their real names, e.g. for the language server).
pub fn builtins() -> HashMap<String, BuiltinFn> {
    let mut m: HashMap<String, BuiltinFn> = HashMap::new();
    register_all(&mut m);
    m
}

/// Builtin names in their stable index order (`chapisha` first, then alphabetical); bytecode
/// refers to builtins by position in this list.
pub fn builtin_names() -> Vec<String> {
    let mut names: Vec<String> = builtins().into_keys().collect();
    names.sort_by(|a, b| index_order(a).cmp(&index_order(b)));
    names
}

/// The sort key of [`builtin_names`]' order.
fn index_order(name: &str) -> (bool, &str) {
    (name != "chapisha", name)
}

/// Builtins indexed as in [`builtin_names`], plus a by-name index (native code's host's table).
pub struct BuiltinTable {
    pub fns: Vec<BuiltinFn>,
    pub index: HashMap<String, usize>,
}

impl BuiltinTable {
    pub fn new() -> Self {
        let mut all: Vec<(String, BuiltinFn)> = builtins().into_iter().collect();
        all.sort_by(|(a, _), (b, _)| index_order(a).cmp(&index_order(b)));
        let mut fns = Vec::with_capacity(all.len());
        let mut index = HashMap::with_capacity(all.len());
        for (i, (name, f)) in all.into_iter().enumerate() {
            fns.push(f);
            index.insert(name, i);
        }
        BuiltinTable { fns, index }
    }
}

impl Default for BuiltinTable {
    fn default() -> Self {
        Self::new()
    }
}
