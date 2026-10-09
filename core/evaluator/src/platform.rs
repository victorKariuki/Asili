//! I/O shim so builtins (matumizi, faili, ...) don't have to repeat target-detection logic.
//!
//! Three cases:
//! - Not wasm32: std's `println!`/`eprintln!`/stdin/fs.
//! - wasm32 with the `wasm-browser` feature: route stdout/stderr to `console.log`/
//!   `console.error` via wasm-bindgen, and load native code through the page
//!   (`load_native_module`). There is no stdin or real filesystem in a browser, so `read_stdin`
//!   and file ops report an explicit error rather than silently no-op.
//! - wasm32 without it: a silent no-op default, so an unconfigured `wasm32-unknown-unknown`
//!   build still compiles (it cannot load native code, so it cannot run programs).

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

use crate::value::EvalError;

/// Block-buffered program output. `println!` flushes on every newline even into a pipe or a
/// file — one `write` system call per `chapisha`. While a [`BlockOutput`] guard is alive and
/// stdout is not a terminal, output collects here instead and goes out in large writes (as C's
/// stdio does); it is flushed before reading input, before writing to stderr (so the two stay in
/// order when they share a file), before `toka`, and when the guard drops.
#[cfg(not(target_arch = "wasm32"))]
mod out {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};

    static BLOCK: AtomicBool = AtomicBool::new(false);
    static BUF: parking_lot::Mutex<Vec<u8>> = parking_lot::Mutex::new(Vec::new());
    const LIMIT: usize = 32 * 1024;

    pub fn write_line(s: &str) {
        if !BLOCK.load(Ordering::Relaxed) {
            println!("{s}");
            return;
        }
        let mut buf = BUF.lock();
        buf.extend_from_slice(s.as_bytes());
        buf.push(b'\n');
        if buf.len() >= LIMIT {
            drain(&mut buf);
        }
    }

    fn drain(buf: &mut Vec<u8>) {
        if !buf.is_empty() {
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(buf);
            let _ = out.flush();
            buf.clear();
        }
    }

    pub fn flush() {
        drain(&mut BUF.lock());
    }

    /// Buffer program output until dropped (a no-op when stdout is a terminal, where each line
    /// should appear as it is written).
    pub struct BlockOutput(bool);

    impl BlockOutput {
        pub fn begin() -> Self {
            let block = !std::io::IsTerminal::is_terminal(&std::io::stdout());
            if block {
                // A panic (an interpreter bug; `panic = "abort"` in release builds skips this
                // guard's drop) must not swallow what the program already printed.
                static HOOK: std::sync::Once = std::sync::Once::new();
                HOOK.call_once(|| {
                    let previous = std::panic::take_hook();
                    std::panic::set_hook(Box::new(move |info| {
                        if let Some(mut buf) = BUF.try_lock() {
                            drain(&mut buf);
                        }
                        previous(info);
                    }));
                });
                BLOCK.store(true, Ordering::Relaxed);
            }
            BlockOutput(block)
        }
    }

    impl Drop for BlockOutput {
        fn drop(&mut self) {
            if self.0 {
                BLOCK.store(false, Ordering::Relaxed);
                flush();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use out::{flush as flush_stdout, BlockOutput};

/// Nothing is buffered without std I/O.
#[cfg(target_arch = "wasm32")]
pub fn flush_stdout() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_stdout(s: &str) {
    out::write_line(s);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_stderr(s: &str) {
    out::flush();
    eprintln!("{s}");
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_stdin() -> Result<String, EvalError> {
    // A prompt written just before must be visible while waiting for the answer.
    out::flush();
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

#[cfg(all(target_arch = "wasm32", feature = "wasm-browser"))]
mod browser {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console, js_name = log)]
        pub fn log(s: &str);
        #[wasm_bindgen(js_namespace = console, js_name = error)]
        pub fn error(s: &str);
        /// The page's loader for native code (`asili_nguvu_load` on `globalThis`): instantiate
        /// `bytes` with `env.memory`, `env.table` and `env.base` (where the table grew by
        /// `count`), and return that base. See `load_native_module`.
        #[wasm_bindgen(catch, js_name = asili_nguvu_load)]
        pub fn nguvu_load(
            bytes: &[u8],
            count: u32,
            memory: JsValue,
            table: JsValue,
        ) -> Result<u32, JsValue>;
    }
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-browser"))]
pub fn write_stdout(s: &str) {
    browser::log(s);
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-browser"))]
pub fn write_stderr(s: &str) {
    browser::error(s);
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-browser"))]
pub fn read_stdin() -> Result<String, EvalError> {
    Err(EvalError::Panic(
        "omba: stdin haipatikani kwenye kivinjari".to_string(),
    ))
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasm-browser")))]
pub fn write_stdout(_s: &str) {}

#[cfg(all(target_arch = "wasm32", not(feature = "wasm-browser")))]
pub fn write_stderr(_s: &str) {}

/// Instantiate a wasm module of native code beside this one (sharing its memory and function
/// table) whose `count` functions fill the table from the index returned (see `nguvu::wasm`).
#[cfg(all(target_arch = "wasm32", feature = "wasm-browser"))]
pub fn load_native_module(bytes: &[u8], count: u32) -> Result<usize, String> {
    let memory = wasm_bindgen::memory();
    let table = wasm_bindgen::function_table();
    browser::nguvu_load(bytes, count, memory, table)
        .map(|base| base as usize)
        .map_err(|e| format!("msimbo asilia haukupakiwa: {e:?}"))
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasm-browser")))]
pub fn read_stdin() -> Result<String, EvalError> {
    Err(EvalError::Panic(
        "omba: stdin haipatikani kwenye WASM".to_string(),
    ))
}
