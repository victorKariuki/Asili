//! Usajili wa kazi zilizojengwa (builtins): msingi, mfumo, majira, matumizi, faili, hisabati, runtime.

mod faili;
mod hisabati;
mod kiungo;
mod majira;
mod matumizi;
mod mfumo;
mod msingi;
// TODO: neno module defines string utility zilizojengwa (chapisha, onyo, makosa, paparika) but is
// never registered with zilizojengwa(). The matumizi module registers those functions instead.
// Decide: remove neno.rs as dead code, or repurpose it for string-specific zilizojengwa
// (gawanya, badilisha, anza_na, maliza_na, kwa_herufi_ndogo, kwa_herufi_kubwa) and register it.
#[allow(dead_code)]
mod neno;
mod runtime;
mod sambamba;
mod syscall;

use std::collections::HashMap;

use crate::value::{Value, EvalError};

pub(crate) type BuiltinFn = Box<dyn Fn(&[Value]) -> Result<Value, EvalError>>;

pub(crate) fn builtins() -> HashMap<String, BuiltinFn> {
    let mut m: HashMap<String, BuiltinFn> = HashMap::new();
    msingi::register(&mut m);
    mfumo::register(&mut m);
    majira::register(&mut m);
    matumizi::register(&mut m);
    faili::register(&mut m);
    hisabati::register(&mut m);
    kiungo::register(&mut m);
    runtime::register(&mut m);
    sambamba::register(&mut m);
    syscall::register(&mut m);
    m
}

pub(crate) fn builtin_names() -> Vec<String> {
    let mut m = HashMap::new();
    msingi::register(&mut m);
    mfumo::register(&mut m);
    majira::register(&mut m);
    matumizi::register(&mut m);
    faili::register(&mut m);
    hisabati::register(&mut m);
    kiungo::register(&mut m);
    runtime::register(&mut m);
    sambamba::register(&mut m);
    syscall::register(&mut m);

    let mut names: Vec<String> = m.keys().cloned().collect();
    // Ensure "chapisha" is faharisi 0 for backward compatibility
    names.sort_by(|a, b| {
        if a == "chapisha" { std::cmp::Ordering::Less }
        else if b == "chapisha" { std::cmp::Ordering::Greater }
        else { a.cmp(b) }
    });
    names
}
