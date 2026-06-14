//! Builtin function registry: msingi, mfumo, majira, matumizi, faili, hisabati, runtime.

mod faili;
mod hisabati;
mod kiungo;
mod majira;
mod matumizi;
mod mfumo;
mod msingi;
// PHASE II: neno module is reserved for string-specific methods (gawanya, badilisha, anza_na, maliza_na, etc.)
// Currently, basic string output (chapisha, onyo, makosa, paparika) are in matumizi instead.
#[allow(dead_code)]
mod neno;
mod runtime;
mod sambamba;
mod syscall;

use std::collections::HashMap;

use crate::value::{Value, EvalError};

pub type BuiltinFn = Box<dyn Fn(&[Value]) -> Result<Value, EvalError>>;

pub fn builtins() -> HashMap<String, BuiltinFn> {
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

pub fn builtin_names() -> Vec<String> {
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
    // Ensure "chapisha" is index 0 for backward compatibility
    names.sort_by(|a, b| {
        if a == "chapisha" { std::cmp::Ordering::Less }
        else if b == "chapisha" { std::cmp::Ordering::Greater }
        else { a.cmp(b) }
    });
    names
}
