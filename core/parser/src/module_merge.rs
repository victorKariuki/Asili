//! Merge an entrypoint `Module` with its resolved `leta` imports into one evaluable `Module`.
//!
//! Shared by `pata/cli`'s disk-based resolver (`pata/cli/src/pipeline/resolve.rs`) and
//! `driver/wasm`'s in-memory bundler, so the merge rules (what counts as a public export, how
//! `leta X::{a, b}` selective imports filter) live in exactly one place.

use crate::{ImportPath, Module};
use std::collections::HashMap;

/// Merge `entrypoint` with each module it `leta`s from `resolved` (name -> already-parsed
/// `Module`). Only names actually reachable via an import are pulled in: public functions,
/// structs and traits (or the selective list named by `leta X::{a, b}`), every module-level
/// `thabiti` constant (constants have no visibility modifier), and any impl block from an
/// imported module.
pub fn merge_modules(entrypoint: &Module, resolved: &HashMap<String, Module>) -> Module {
    let mut functions = entrypoint.functions.clone();
    let mut constants = entrypoint.constants.clone();
    let mut structs = entrypoint.structs.clone();
    let mut traits = entrypoint.traits.clone();
    let mut impls = entrypoint.impls.clone();
    for imp in &entrypoint.imports {
        let (module_name, names_to_import) = match &imp.path {
            ImportPath::Full(name) => (name.as_str(), None as Option<Vec<String>>),
            ImportPath::Selective { module: name, names } => (name.as_str(), Some(names.clone())),
        };
        let Some(module) = resolved.get(module_name) else { continue };
        for f in &module.functions {
            let include = match &names_to_import {
                None => f.is_public,
                Some(names) => names.contains(&f.name),
            };
            if include && !functions.iter().any(|x| x.name == f.name) {
                functions.push(f.clone());
            }
        }
        for c in &module.constants {
            let include = match &names_to_import {
                None => true,
                Some(names) => names.contains(&c.name),
            };
            if include && !constants.iter().any(|x| x.name == c.name) {
                constants.push(c.clone());
            }
        }
        for s in &module.structs {
            let include = match &names_to_import {
                None => s.is_public,
                Some(names) => names.contains(&s.name),
            };
            if include && !structs.iter().any(|x| x.name == s.name) {
                structs.push(s.clone());
            }
        }
        for t in &module.traits {
            let include = match &names_to_import {
                None => t.is_public,
                Some(names) => names.contains(&t.name),
            };
            if include && !traits.iter().any(|x| x.name == t.name) {
                traits.push(t.clone());
            }
        }
        for imp_decl in &module.impls {
            if !impls.iter().any(|x| x.target == imp_decl.target) {
                impls.push(imp_decl.clone());
            }
        }
    }
    Module {
        imports: entrypoint.imports.clone(),
        constants,
        enums: entrypoint.enums.clone(),
        functions,
        structs,
        traits,
        impls,
    }
}
