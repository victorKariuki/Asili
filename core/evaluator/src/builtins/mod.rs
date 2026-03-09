//! Builtin function registry: msingi, mfumo, majira, matumizi, faili, hisabati, runtime.

mod faili;
mod hisabati;
mod kiungo;
mod majira;
mod matumizi;
mod mfumo;
mod msingi;
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
