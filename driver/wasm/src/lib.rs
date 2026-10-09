//! Asili Wasm driver: runs Asili source in the browser (`wasm-browser`). Programs are lowered to
//! bytecode and run as native code — a wasm module of their own the page instantiates beside
//! this one (`asili_evaluator::nguvu::wasm`). I/O builtins (chapisha, majira, …) route through
//! `asili_evaluator`'s platform shim (console.log / console.error in the browser).

use asili_evaluator::{compile_module_explained, run_bytecode};
use asili_lexer::tokenize;
use asili_parser::{merge_modules, parse_tokens, ImportPath, Module};
use std::collections::HashMap;

/// Run Asili source: tokenize, parse, run `kuu` with empty args. Single-module only — any `leta`
/// import is left unresolved (its exports just won't be found). For multi-module programs use
/// `run_bundle`.
pub fn run_source(source: &str) -> Result<(), String> {
    let tokens = tokenize(source).map_err(|d| format!("lex: {:?}", d))?;
    let module = parse_tokens(&tokens).map_err(|d| format!("parse: {:?}", d))?;
    run_module(&module)
}

/// Lower `module` to bytecode and run its `kuu` as native code (a wasm module the page
/// instantiates beside this one; see `asili_evaluator::nguvu::wasm`).
fn run_module(module: &Module) -> Result<(), String> {
    let program = compile_module_explained(module)?;
    run_bytecode(&program, vec![]).map_err(|e| e.to_string())
}

/// Run a multi-module Asili program from in-memory sources (no filesystem access, so this is the
/// Wasm-appropriate counterpart to `pata/cli`'s disk-based resolver). `modules` maps a module
/// name (as it appears in `leta <name>`) to that module's source text; `entry_name` is the
/// program's entrypoint. Every `leta` target the entrypoint (transitively) names must have an
/// entry in `modules`, or it is silently skipped — same as an unresolved import today.
pub fn run_bundle(entry_name: &str, modules: &HashMap<String, String>) -> Result<(), String> {
    let entry_source = modules
        .get(entry_name)
        .ok_or_else(|| format!("bundle: entry module '{entry_name}' haipo kwenye modules"))?;
    let tokens = tokenize(entry_source).map_err(|d| format!("lex: {:?}", d))?;
    let entrypoint = parse_tokens(&tokens).map_err(|d| format!("parse: {:?}", d))?;

    let mut parsed: HashMap<String, Module> = HashMap::new();
    let mut pending: Vec<String> = entrypoint
        .imports
        .iter()
        .map(|imp| match &imp.path {
            ImportPath::Full(n) => n.clone(),
            ImportPath::Selective { module, .. } => module.clone(),
        })
        .collect();

    while let Some(name) = pending.pop() {
        if parsed.contains_key(&name) {
            continue;
        }
        let Some(source) = modules.get(&name) else {
            continue;
        };
        let tokens = tokenize(source).map_err(|d| format!("lex ({name}): {:?}", d))?;
        let module = parse_tokens(&tokens).map_err(|d| format!("parse ({name}): {:?}", d))?;
        for imp in &module.imports {
            let dep_name = match &imp.path {
                ImportPath::Full(n) => n.clone(),
                ImportPath::Selective { module, .. } => module.clone(),
            };
            if !parsed.contains_key(&dep_name) {
                pending.push(dep_name);
            }
        }
        parsed.insert(name, module);
    }

    let merged = merge_modules(&entrypoint, &parsed);
    run_module(&merged)
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-browser"))]
mod browser_entry {
    use super::{run_bundle, run_source};
    use std::collections::HashMap;
    use wasm_bindgen::prelude::*;

    /// Single-module entry point for `wasm-bindgen` consumers (e.g. a browser playground).
    #[wasm_bindgen]
    pub fn run(source: &str) -> Result<(), JsValue> {
        run_source(source).map_err(|e| JsValue::from_str(&e))
    }

    /// Multi-module entry point. `modules` is a JS object/Map-like value convertible to
    /// `HashMap<String, String>` (module name -> source) via `serde-wasm-bindgen` on the host
    /// side; here it arrives already converted for simplicity of the Rust API surface.
    #[wasm_bindgen(js_name = runBundle)]
    pub fn run_bundle_js(entry_name: &str, modules: JsValue) -> Result<(), JsValue> {
        let modules: HashMap<String, String> = serde_wasm_bindgen::from_value(modules)
            .map_err(|e| JsValue::from_str(&format!("modules: {e}")))?;
        run_bundle(entry_name, &modules).map_err(|e| JsValue::from_str(&e))
    }
}

#[cfg(test)]
mod tests {
    use super::{run_bundle, run_source};
    use std::collections::HashMap;

    #[test]
    fn run_source_minimal_kuu() {
        let src = r#"
kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  rejesha
}
"#;
        run_source(src).expect("run_source minimal kazu kuu");
    }

    #[test]
    fn run_bundle_resolves_multi_module_leta() {
        let mut modules = HashMap::new();
        modules.insert(
            "kuu".to_string(),
            "leta mathutil\numma umbo Punkt { x: Namba }\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n  weka p = Punkt { x: PI }\n  rejesha\n}".to_string(),
        );
        modules.insert(
            "mathutil".to_string(),
            "thabiti PI: Namba = 3.14\n".to_string(),
        );

        run_bundle("kuu", &modules).expect("run_bundle resolves cross-module constant");
    }

    #[test]
    fn run_bundle_missing_entry_is_an_error() {
        let modules = HashMap::new();
        let err = run_bundle("kuu", &modules).unwrap_err();
        assert!(err.contains("kuu"));
    }
}
