//! I/O shim so builtins (matumizi, faili, ...) don't have to repeat target-detection logic.
//!
//! Three cases:
//! - Non-wasm32, or wasm32 with the `wasm-wasi` feature: std's `println!`/`eprintln!`/stdin/fs
//!   already work correctly (Rust's std has first-class WASI support — no extra crate needed).
//! - wasm32 with the `wasm-browser` feature (and not `wasm-wasi`): route stdout/stderr to
//!   `console.log`/`console.error` via wasm-bindgen. There is no stdin or real filesystem in a
//!   browser, so `read_stdin` and file ops report an explicit error rather than silently no-op.
//! - wasm32 with neither feature: keep the historical silent-no-op default, so an unconfigured
//!   `wasm32-unknown-unknown` build still compiles and runs (just without real I/O).

use crate::value::EvalError;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
pub fn write_stdout(s: &str) {
    println!("{s}");
}

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
pub fn write_stderr(s: &str) {
    eprintln!("{s}");
}

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-wasi"))]
pub fn read_stdin() -> Result<String, EvalError> {
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(_) => {
            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }
            Ok(line)
        }
        Err(e) => Err(EvalError::Panic(format!("omba: hitilafu ya kusoma: {e}"))),
    }
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-browser", not(feature = "wasm-wasi")))]
mod browser {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console, js_name = log)]
        pub fn log(s: &str);
        #[wasm_bindgen(js_namespace = console, js_name = error)]
        pub fn error(s: &str);
    }
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-browser", not(feature = "wasm-wasi")))]
pub fn write_stdout(s: &str) {
    browser::log(s);
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-browser", not(feature = "wasm-wasi")))]
pub fn write_stderr(s: &str) {
    browser::error(s);
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-browser", not(feature = "wasm-wasi")))]
pub fn read_stdin() -> Result<String, EvalError> {
    Err(EvalError::Panic("omba: stdin haipatikani kwenye kivinjari".to_string()))
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasm-browser"), not(feature = "wasm-wasi")))]
pub fn write_stdout(_s: &str) {}

#[cfg(all(target_arch = "wasm32", not(feature = "wasm-browser"), not(feature = "wasm-wasi")))]
pub fn write_stderr(_s: &str) {}

#[cfg(all(target_arch = "wasm32", not(feature = "wasm-browser"), not(feature = "wasm-wasi")))]
pub fn read_stdin() -> Result<String, EvalError> {
    Err(EvalError::Panic("omba: stdin haipatikani kwenye WASM".to_string()))
}
