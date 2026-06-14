//! Asili Wasm driver — Phase II.
//! Thin layer to run Asili source in a WebAssembly environment (browser or WASI).
//! I/O builtins (chapisha, majira, vigezo, pata_env, etc.) use spec-compliant stubs when built for wasm32.
//
// TODO(Phase II): run_source is single-module only — `leta` imports are silently ignored in WASM.
// To support imports: bundle all imported modules into a single merged Module before calling run_main,
// or implement a WASM-compatible module resolver (e.g. pass a Map<String, String> of module sources).
//
// TODO(Phase II): chapisha and other I/O builtins are no-ops in WASM (#[cfg(target_arch="wasm32")]).
// For browser targets: wire chapisha to console.log via wasm-bindgen.
// For WASI targets: wire chapisha to fd_write on stdout using the wasi crate.
//
// TODO(Phase II): run_source currently takes Asili source text. For production use, accept .asb bytes
// (load_asb) so the browser doesn't need to run the full parser — only the evaluator.

use asili_evaluator::{run_main, EvalError};
use asili_lexer::tokenize;
use asili_parser::parse_tokens;

/// Run Asili source: tokenize, parse, run `kuu` with empty args.
/// Single-module only; no import resolution. Returns `Ok(())` on success.
pub fn run_source(source: &str) -> Result<(), String> {
    let tokens = tokenize(source).map_err(|d| format!("lex: {:?}", d))?;
    let module = parse_tokens(&tokens).map_err(|d| format!("parse: {:?}", d))?;
    run_main(&module, vec![]).map_err(|e: EvalError| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run_source;

    #[test]
    fn run_source_minimal_kuu() {
        let src = r#"
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  rejesha
}
"#;
        run_source(src).expect("run_source minimal kazu kuu");
    }
}
