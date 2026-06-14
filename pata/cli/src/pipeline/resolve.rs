//! Module loader: resolve imports, build export tables, detect cycles.
//
// TODO: build_export_table only exports public functions. Constants declared at module level
// with `thabiti` are never added to the export table (the `constants` field is always empty
// for user modules). Programs that `leta` a module expecting its constants will get Unknown type.
//
// TODO: merge_for_eval only merges functions from imported modules — not structs, traits, or impls.
// A program that imports a struct definition from another module cannot use it at runtime because
// the struct is not present in the merged Module passed to the evaluator.

use crate::pipeline::interface_registry::{InterfaceRegistry, StdlibEnv};
use asili_diagnostics::Diagnostic;
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, parse_value_type, FnContract, ImportPath, Module, ValueType};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Export table for a module: public functions and constants.
#[derive(Default, Clone, Debug)]
pub struct ExportTable {
    pub functions: HashMap<String, FnContract>,
    pub constants: HashMap<String, ValueType>,
}

/// A resolved module: AST plus its export table.
#[derive(Clone, Debug)]
pub struct ResolvedModule {
    pub module: Module,
    pub exports: ExportTable,
    /// True if loaded from lib/std/*.asi (names may overlap with StdlibEnv).
    pub is_stdlib: bool,
}

/// Full resolved program: entrypoint plus all resolved dependencies.
#[derive(Clone, Debug)]
pub struct ResolvedProgram {
    #[allow(dead_code)]
    pub entrypoint: Module,
    pub resolved: HashMap<String, ResolvedModule>,
    pub merged_for_eval: Module,
}

/// Build export table from a module AST (public functions only; constants from AST if present).
pub fn build_export_table(module: &Module) -> ExportTable {
    let mut functions = HashMap::new();
    let constants = HashMap::new();
    for f in &module.functions {
        if f.is_public {
            let params = f.params.iter().map(|p| parse_value_type(&p.ty.name)).collect();
            let ret = parse_value_type(&f.return_type.name);
            functions.insert(f.name.clone(), FnContract { params, ret });
        }
    }
    // Constants: we could scan for top-level thabiti/weka; for now leave empty for user modules.
    ExportTable { functions, constants }
}

/// Search path order: root, root/lib, then root/lib/std (stdlib .asi). Returns (path, true if stdlib .asi).
pub fn find_module_file(name: &str, root: &Path) -> Option<(PathBuf, bool)> {
    let candidates = [
        (root.join(format!("{name}.as")), false),
        (root.join("lib").join(format!("{name}.as")), false),
        (root.join("lib").join(name).join("mod.as"), false),
        (root.join("lib").join("std").join(format!("{name}.asi")), true),
    ];
    for (p, is_asi) in &candidates {
        if p.is_file() {
            return Some((p.clone(), *is_asi));
        }
    }
    None
}

/// Resolve a single module by name: load file, parse, recursively resolve its imports.
fn resolve_one(
    name: &str,
    root: &Path,
    resolved: &mut HashMap<String, ResolvedModule>,
    loading: &mut HashSet<String>,
    errors: &mut Vec<Diagnostic>,
    registry: &mut InterfaceRegistry,
) {
    if resolved.contains_key(name) {
        return;
    }
    if loading.contains(name) {
        errors.push(
            Diagnostic::new("RES001", "mzunguko wa moduli: moduli imejirejea")
                .with_stage("resolve"),
        );
        return;
    }
    if let Some(iface) = registry.get(name) {
        let (functions, constants) = iface.to_export_table();
        let exports = ExportTable { functions, constants };
        resolved.insert(
            name.to_string(),
            ResolvedModule {
                module: Module {
                    imports: vec![],
                    functions: vec![],
                    structs: vec![],
                    traits: vec![],
                    impls: vec![],
                },
                exports,
                is_stdlib: true,
            },
        );
        return;
    }
    let (path, is_asi) = match find_module_file(name, root) {
        Some(p) => p,
        None => {
            let searched = format!(
                "{}; {}; {}; {}",
                root.join(format!("{name}.as")).display(),
                root.join("lib").join(format!("{name}.as")).display(),
                root.join("lib").join(name).join("mod.as").display(),
                root.join("lib").join("std").join(format!("{name}.asi")).display()
            );
            errors.push(
                Diagnostic::new("RES002", format!("moduli '{name}' haikupatikana: {searched}"))
                    .with_stage("resolve"),
            );
            return;
        },
    };
    loading.insert(name.to_string());

    if is_asi {
        loading.remove(name);
        match registry.get_or_load(name, &path) {
            Ok(iface) => {
                let (functions, constants) = iface.to_export_table();
                let exports = ExportTable { functions, constants };
                let module = Module {
                    imports: vec![],
                    functions: vec![],
                    structs: vec![],
                    traits: vec![],
                    impls: vec![],
                };
                resolved.insert(
                    name.to_string(),
                    ResolvedModule {
                        module,
                        exports,
                        is_stdlib: true,
                    },
                );
            }
            Err(e) => {
                errors.push(
                    Diagnostic::new("RES003", e.message.clone()).with_stage("resolve"),
                );
            }
        }
        return;
    }

    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            errors.push(
                Diagnostic::new("RES003", format!("imeshindwa kusoma {}: {e}", path.display()))
                    .with_stage("resolve"),
            );
            loading.remove(name);
            return;
        },
    };
    let tokens = match tokenize(&source) {
        Ok(t) => t,
        Err(lex_errs) => {
            for d in lex_errs {
                errors.push(d);
            }
            loading.remove(name);
            return;
        },
    };
    let module = match parse_tokens(&tokens) {
        Ok(m) => m,
        Err(parse_errs) => {
            for d in parse_errs {
                errors.push(d);
            }
            loading.remove(name);
            return;
        },
    };
    for imp in &module.imports {
        let dep_name = match &imp.path {
            ImportPath::Full(n) => n.as_str(),
            ImportPath::Selective { module: n, .. } => n.as_str(),
        };
        resolve_one(dep_name, root, resolved, loading, errors, registry);
    }
    loading.remove(name);
    let exports = build_export_table(&module);
    resolved.insert(
        name.to_string(),
        ResolvedModule {
            module,
            exports,
            is_stdlib: false,
        },
    );
}

/// Resolve all imports of the entrypoint module; return entrypoint + resolved map or errors.
pub fn resolve_all(
    entrypoint: &Module,
    root: &Path,
    registry: &mut InterfaceRegistry,
) -> Result<ResolvedProgram, Vec<Diagnostic>> {
    let mut resolved = HashMap::new();
    let mut loading = HashSet::new();
    let mut errors = Vec::new();
    for imp in &entrypoint.imports {
        let name = match &imp.path {
            ImportPath::Full(n) => n.as_str(),
            ImportPath::Selective { module: n, .. } => n.as_str(),
        };
        resolve_one(name, root, &mut resolved, &mut loading, &mut errors, registry);
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let merged_for_eval = merge_for_eval(entrypoint, &resolved);
    Ok(ResolvedProgram {
        entrypoint: entrypoint.clone(),
        resolved,
        merged_for_eval,
    })
}

/// Merge entrypoint + resolved modules into one module for evaluation (imported names only).
fn merge_for_eval(entrypoint: &Module, resolved: &HashMap<String, ResolvedModule>) -> Module {
    let mut functions = entrypoint.functions.clone();
    let structs = entrypoint.structs.clone();
    let traits = entrypoint.traits.clone();
    let impls = entrypoint.impls.clone();
    for imp in &entrypoint.imports {
        let (module_name, names_to_import) = match &imp.path {
            ImportPath::Full(name) => (name.as_str(), None as Option<Vec<String>>),
            ImportPath::Selective { module: name, names } => (name.as_str(), Some(names.clone())),
        };
        let Some(res) = resolved.get(module_name) else { continue };
        for f in &res.module.functions {
            let include = match &names_to_import {
                None => f.is_public,
                Some(names) => names.contains(&f.name),
            };
            if include && !functions.iter().any(|x| x.name == f.name) {
                functions.push(f.clone());
            }
        }
    }
    Module {
        imports: entrypoint.imports.clone(),
        functions,
        structs,
        traits,
        impls,
    }
}

/// Build merged extern env for semantic: prelude (msingi) + for each import, add that module's exported names.
pub fn merge_for_semantic(
    module: &Module,
    resolved: &HashMap<String, ResolvedModule>,
    prelude: &StdlibEnv,
) -> (HashMap<String, FnContract>, HashMap<String, ValueType>) {
    let mut functions = prelude.functions.clone();
    let mut constants = prelude.constants.clone();
    for imp in &module.imports {
        let (module_name, names_to_import) = match &imp.path {
            ImportPath::Full(name) => (name.as_str(), None as Option<Vec<String>>),
            ImportPath::Selective { module: name, names } => (name.as_str(), Some(names.clone())),
        };
        let Some(res) = resolved.get(module_name) else { continue };
        for (name, contract) in &res.exports.functions {
            let include = match &names_to_import {
                None => true,
                Some(names) => names.contains(name),
            };
            if include {
                functions.insert(name.clone(), contract.clone());
            }
        }
        for (name, ty) in &res.exports.constants {
            let include = match &names_to_import {
                None => true,
                Some(names) => names.contains(name),
            };
            if include {
                constants.insert(name.clone(), ty.clone());
            }
        }
    }
    (functions, constants)
}

/// Return resolved module names in dependency order (dependencies first).
pub fn dependency_order(resolved: &HashMap<String, ResolvedModule>) -> Vec<String> {
    let names: Vec<String> = resolved.keys().cloned().collect();
    let mut order = Vec::new();
    let mut remaining: HashSet<String> = names.into_iter().collect();
    while !remaining.is_empty() {
        let mut found = None;
        for name in &remaining {
            let res = &resolved[name];
            let deps: Vec<&str> = res
                .module
                .imports
                .iter()
                .map(|imp| match &imp.path {
                    ImportPath::Full(n) => n.as_str(),
                    ImportPath::Selective { module: n, .. } => n.as_str(),
                })
                .filter(|n| remaining.contains(*n))
                .collect();
            if deps.is_empty() {
                found = Some(name.clone());
                break;
            }
        }
        // HACK: dependency_order() panics if a cycle slips through (e.g. RES001 was not triggered).
    // Should return Result<Vec<String>, Diagnostic> so the caller can surface the error cleanly.
    let name = found.expect("cycle in resolved modules (should be prevented by RES001)");
        remaining.remove(&name);
        order.push(name);
    }
    order
}

/// Check for duplicate import names when merging (same name from two imports or prelude).
pub fn check_duplicate_imports(
    entrypoint: &Module,
    resolved: &HashMap<String, ResolvedModule>,
    prelude: &StdlibEnv,
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let mut seen_functions: HashMap<String, String> = HashMap::new();
    let mut seen_constants: HashMap<String, String> = HashMap::new();
    for name in prelude.functions.keys() {
        seen_functions.insert(name.clone(), "msingi".to_string());
    }
    for name in prelude.constants.keys() {
        seen_constants.insert(name.clone(), "msingi".to_string());
    }
    for imp in &entrypoint.imports {
        let (module_name, names_to_import) = match &imp.path {
            ImportPath::Full(name) => (name.as_str(), None as Option<Vec<String>>),
            ImportPath::Selective { module: name, names } => (name.as_str(), Some(names.clone())),
        };
        let Some(res) = resolved.get(module_name) else { continue };
        for name in res.exports.functions.keys() {
            let include = names_to_import.as_ref().map_or(true, |n| n.contains(name));
            if include {
                if let Some(from) = seen_functions.get(name) {
                    if *from != "msingi" || !res.is_stdlib {
                        errors.push(
                            Diagnostic::new("SEM090", format!("jina lamerudia: '{name}' limetoka {from} na {module_name}"))
                                .with_stage("semantic")
                                .with_span(imp.line, 1),
                        );
                    }
                } else {
                    seen_functions.insert(name.clone(), module_name.to_string());
                }
            }
        }
        for name in res.exports.constants.keys() {
            let include = names_to_import.as_ref().map_or(true, |n| n.contains(name));
            if include {
                if let Some(from) = seen_constants.get(name) {
                    if *from != "msingi" || !res.is_stdlib {
                        errors.push(
                            Diagnostic::new("SEM091", format!("jina lamerudia: '{name}' (thabiti) limetoka {from}"))
                                .with_stage("semantic")
                                .with_span(imp.line, 1),
                        );
                    }
                } else {
                    seen_constants.insert(name.clone(), module_name.to_string());
                }
            }
        }
    }
    errors
}
