//! Standalone WASI binary: `asili_wasi <faili.as>` runs a single-module Asili program under
//! `wasmtime`/Node WASI/etc. Real stdout/stderr/stdin/fs come from `asili_evaluator`'s platform
//! shim when built with `--features wasm-wasi` (see driver/wasm/README.md).

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args();
    let _bin = args.next();
    let Some(path) = args.next() else {
        eprintln!("matumizi: asili_wasi <faili.as>");
        return ExitCode::FAILURE;
    };
    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("imeshindwa kusoma {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    match asili_wasm::run_source(&source) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("kosa: {e}");
            ExitCode::FAILURE
        }
    }
}
