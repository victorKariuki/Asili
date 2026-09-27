//! Module loader: resolve imports, build export tables, detect cycles.
//
// Note: build_export_table exports all module-level `thabiti` constants (there is no visibility
// modifier on constants yet, so every one is treated as exported) and merge_for_eval merges
// structs/traits/impls from imported modules in addition to functions — both were once TODOs
// here but are already implemented below.

use crate::dependency::Dependency;
use crate::interface_registry::{InterfaceRegistry, StdlibEnv};
use asili_diagnostics::Diagnostic;
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, parse_value_type, FnContract, ImportPath, Module, ValueType};
use std::collections::{BTreeMap, HashMap};
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

/// Build export table from a module AST (public functions and constants).
pub fn build_export_table(module: &Module) -> ExportTable {
    let mut functions = HashMap::new();
    let mut constants = HashMap::new();
    for f in &module.functions {
        if f.is_public {
            let params = f
                .params
                .iter()
                .map(|p| parse_value_type(&p.ty.name))
                .collect();
            let ret = parse_value_type(&f.return_type.name);
            functions.insert(f.name.clone(), FnContract { params, ret });
        }
    }
    for c in &module.constants {
        let ty = parse_value_type(&c.ty.name);
        constants.insert(c.name.clone(), ty);
    }
    ExportTable {
        functions,
        constants,
    }
}

/// Search path order: root, root/lib, then root/lib/std (stdlib .asi). Returns (path, true if stdlib .asi).
pub fn find_module_file(
    name: &str,
    root: &Path,
    dependencies: &BTreeMap<String, Dependency>,
) -> Option<(PathBuf, bool)> {
    // Check path-based dependencies first.
    if let Some(Dependency::Path(dep_path)) = dependencies.get(name) {
        let abs_path = if dep_path.is_absolute() {
            dep_path.clone()
        } else {
            root.join(dep_path)
        };
        // Expect a project structure within the dependency path.
        let entrypoint = abs_path.join("src").join(format!("{name}.as"));
        if entrypoint.is_file() {
            return Some((entrypoint, false));
        }
    }

    // A version or git dependency resolves against the vendored package cache
    // (.asili/packages/<name>/), populated by `pata ongeza --git` (real fetch) or the registry
    // resolver (real fetch from a RegistrySource) — see pata_package::Resolver::resolve. Two
    // vendored layouts are checked: a real project layout (`src/<name>.as`, matching path
    // dependencies' own convention) and a bare single-file source directly at the vendor root
    // (`<name>.as`) — a git repo whose only content is the module file itself, with no `src/`
    // subdirectory, is a legitimate shape for a small dependency and shouldn't require one. If
    // neither is vendored, resolution falls through to the generic candidates below and
    // ultimately reports RES002 with the full searched-path list.
    if matches!(
        dependencies.get(name),
        Some(Dependency::Version(_)) | Some(Dependency::Git { .. })
    ) {
        let vendor_root = pata_package::Paths::new(root).package_path(name);
        let candidates = [
            vendor_root.join("src").join(format!("{name}.as")),
            vendor_root.join(format!("{name}.as")),
        ];
        for candidate in &candidates {
            if candidate.is_file() {
                return Some((candidate.clone(), false));
            }
        }
    }

    let candidates = [
        (root.join(format!("{name}.as")), false),
        (root.join("lib").join(format!("{name}.as")), false),
        (root.join("lib").join(name).join("mod.as"), false),
        (
            root.join("lib").join("std").join(format!("{name}.asi")),
            true,
        ),
    ];
    for (p, is_asi) in &candidates {
        if p.is_file() {
            return Some((p.clone(), *is_asi));
        }
    }
    None
}

/// Resolve a single module by name: load file, parse, recursively resolve its imports.
/// `loading` is the stack of modules currently being resolved (outermost first) and
/// `import_line` the line of the `leta` that asked for `name`, so a cycle can be reported as
/// the actual chain at the import that closes it.
#[allow(clippy::too_many_arguments)]
fn resolve_one(
    name: &str,
    import_line: usize,
    root: &Path,
    dependencies: &BTreeMap<String, Dependency>,
    resolved: &mut HashMap<String, ResolvedModule>,
    loading: &mut Vec<String>,
    errors: &mut Vec<Diagnostic>,
    registry: &mut InterfaceRegistry,
) {
    if resolved.contains_key(name) {
        return;
    }
    if let Some(start) = loading.iter().position(|m| m == name) {
        errors.push(cycle_diagnostic(&loading[start..], name).with_span(import_line, 1));
        return;
    }
    if let Some(iface) = registry.get(name) {
        let (functions, constants) = iface.to_export_table();
        let exports = ExportTable {
            functions,
            constants,
        };
        resolved.insert(
            name.to_string(),
            ResolvedModule {
                module: Module {
                    imports: vec![],
                    constants: vec![],
                    enums: vec![],
                    functions: vec![],
                    structs: vec![],
                    traits: iface.trait_decls(),
                    impls: vec![],
                },
                exports,
                is_stdlib: true,
            },
        );
        return;
    }
    let (path, is_asi) = match find_module_file(name, root, dependencies) {
        Some(p) => p,
        None => {
            let searched = format!(
                "{}; {}; {}; {}",
                root.join(format!("{name}.as")).display(),
                root.join("lib").join(format!("{name}.as")).display(),
                root.join("lib").join(name).join("mod.as").display(),
                root.join("lib")
                    .join("std")
                    .join(format!("{name}.asi"))
                    .display()
            );
            errors.push(
                Diagnostic::new(
                    "RES002",
                    format!("moduli '{name}' haikupatikana: {searched}"),
                )
                .with_stage("utatuzi"),
            );
            return;
        }
    };
    loading.push(name.to_string());

    if is_asi {
        loading.pop();
        match registry.get_or_load(name, &path) {
            Ok(iface) => {
                let (functions, constants) = iface.to_export_table();
                let exports = ExportTable {
                    functions,
                    constants,
                };
                let module = Module {
                    imports: vec![],
                    constants: vec![],
                    enums: vec![],
                    functions: vec![],
                    structs: vec![],
                    traits: iface.trait_decls(),
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
                errors.push(Diagnostic::new("RES003", e.message.clone()).with_stage("utatuzi"));
            }
        }
        return;
    }

    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            errors.push(
                Diagnostic::new(
                    "RES003",
                    format!("imeshindwa kusoma {}: {e}", path.display()),
                )
                .with_stage("utatuzi"),
            );
            loading.pop();
            return;
        }
    };
    let tokens = match tokenize(&source) {
        Ok(t) => t,
        Err(lex_errs) => {
            for d in lex_errs {
                errors.push(d);
            }
            loading.pop();
            return;
        }
    };
    let module = match parse_tokens(&tokens) {
        Ok(m) => m,
        Err(parse_errs) => {
            for d in parse_errs {
                errors.push(d);
            }
            loading.pop();
            return;
        }
    };
    for imp in &module.imports {
        let dep_name = match &imp.path {
            ImportPath::Full(n) => n.as_str(),
            ImportPath::Selective { module: n, .. } => n.as_str(),
        };
        resolve_one(
            dep_name,
            imp.line,
            root,
            dependencies,
            resolved,
            loading,
            errors,
            registry,
        );
    }
    loading.pop();
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
    dependencies: &BTreeMap<String, Dependency>,
    registry: &mut InterfaceRegistry,
) -> Result<ResolvedProgram, Vec<Diagnostic>> {
    let mut resolved = HashMap::new();
    let mut loading = Vec::new();
    let mut errors = Vec::new();
    for imp in &entrypoint.imports {
        let name = match &imp.path {
            ImportPath::Full(n) => n.as_str(),
            ImportPath::Selective { module: n, .. } => n.as_str(),
        };
        resolve_one(
            name,
            imp.line,
            root,
            dependencies,
            &mut resolved,
            &mut loading,
            &mut errors,
            registry,
        );
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
/// Delegates to `asili_parser::merge_modules`, shared with `driver/wasm`'s in-memory bundler so
/// the merge rules live in exactly one place.
fn merge_for_eval(entrypoint: &Module, resolved: &HashMap<String, ResolvedModule>) -> Module {
    let modules: HashMap<String, Module> = resolved
        .iter()
        .map(|(name, res)| (name.clone(), res.module.clone()))
        .collect();
    asili_parser::merge_modules(entrypoint, &modules)
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
            ImportPath::Selective {
                module: name,
                names,
            } => (name.as_str(), Some(names.clone())),
        };
        let Some(res) = resolved.get(module_name) else {
            continue;
        };
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

/// RES001: the modules in `chain` import each other in a loop back to `closing`.
fn cycle_diagnostic(chain: &[String], closing: &str) -> Diagnostic {
    let mut path: Vec<&str> = chain.iter().map(String::as_str).collect();
    path.push(closing);
    Diagnostic::new(
        "RES001",
        format!("mzunguko wa moduli: {}", path.join(" → ")),
    )
    .with_stage("utatuzi")
}

/// Resolved module names in dependency order (dependencies first). `resolve_all` already rejects
/// import cycles (RES001); if one reaches here anyway it is reported, never a panic.
pub fn dependency_order(
    resolved: &HashMap<String, ResolvedModule>,
) -> Result<Vec<String>, Diagnostic> {
    let imports_of = |name: &str| -> Vec<&str> {
        resolved[name]
            .module
            .imports
            .iter()
            .map(|imp| match &imp.path {
                ImportPath::Full(n) => n.as_str(),
                ImportPath::Selective { module: n, .. } => n.as_str(),
            })
            .collect()
    };
    let mut remaining: Vec<String> = resolved.keys().cloned().collect();
    remaining.sort(); // deterministic order among independent modules
    let mut order = Vec::new();
    while !remaining.is_empty() {
        let ready = remaining.iter().position(|name| {
            imports_of(name)
                .iter()
                .all(|d| !remaining.iter().any(|r| r == d))
        });
        let Some(i) = ready else {
            return Err(cycle_diagnostic(&remaining, &remaining[0]));
        };
        order.push(remaining.remove(i));
    }
    Ok(order)
}

/// Marker for names that come from the ambient builtin prelude rather than an explicit `leta`.
const PRELUDE: &str = "msingi";

/// Check for duplicate import names when merging: the same name brought in by two explicit
/// imports. Imports may shadow ambient prelude names.
pub fn check_duplicate_imports(
    entrypoint: &Module,
    resolved: &HashMap<String, ResolvedModule>,
    prelude: &StdlibEnv,
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let mut seen_functions: HashMap<String, String> = HashMap::new();
    let mut seen_constants: HashMap<String, String> = HashMap::new();
    for name in prelude.functions.keys() {
        seen_functions.insert(name.clone(), PRELUDE.to_string());
    }
    for name in prelude.constants.keys() {
        seen_constants.insert(name.clone(), PRELUDE.to_string());
    }
    for imp in &entrypoint.imports {
        let (module_name, names_to_import) = match &imp.path {
            ImportPath::Full(name) => (name.as_str(), None as Option<Vec<String>>),
            ImportPath::Selective {
                module: name,
                names,
            } => (name.as_str(), Some(names.clone())),
        };
        let Some(res) = resolved.get(module_name) else {
            continue;
        };
        for name in res.exports.functions.keys() {
            let include = names_to_import.as_ref().is_none_or(|n| n.contains(name));
            if include {
                // Ambient (prelude) names are shadowable, like Rust's prelude: an import of the
                // same name simply wins. Only two explicit imports of one name clash.
                if let Some(from) = seen_functions.get(name).filter(|from| *from != PRELUDE) {
                    errors.push(
                        Diagnostic::new(
                            "SEM090",
                            format!("jina lamerudia: '{name}' limetoka {from} na {module_name}"),
                        )
                        .with_stage("semantiki")
                        .with_span(imp.line, 1),
                    );
                } else {
                    seen_functions.insert(name.clone(), module_name.to_string());
                }
            }
        }
        for name in res.exports.constants.keys() {
            let include = names_to_import.as_ref().is_none_or(|n| n.contains(name));
            if include {
                // Ambient (prelude) names are shadowable, like Rust's prelude: an import of the
                // same name simply wins. Only two explicit imports of one name clash.
                if let Some(from) = seen_constants.get(name).filter(|from| *from != PRELUDE) {
                    errors.push(
                        Diagnostic::new(
                            "SEM091",
                            format!("jina lamerudia: '{name}' (thabiti) limetoka {from}"),
                        )
                        .with_stage("semantiki")
                        .with_span(imp.line, 1),
                    );
                } else {
                    seen_constants.insert(name.clone(), module_name.to_string());
                }
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(src: &str) -> Module {
        asili_parser::parse_tokens(&asili_lexer::tokenize(src).expect("lex")).expect("parse")
    }

    fn exporting_constant(name: &str) -> ResolvedModule {
        let mut exports = ExportTable::default();
        exports.constants.insert(name.to_string(), ValueType::Namba);
        ResolvedModule {
            module: module(""),
            exports,
            is_stdlib: false,
        }
    }

    #[test]
    fn import_cycle_is_reported_as_the_chain_at_the_closing_import() {
        let root = std::env::temp_dir().join(format!("pata-core-cycle-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("a.as"),
            "leta b\numma kazi fa() -> Namba { rejesha 1 }",
        )
        .unwrap();
        fs::write(
            root.join("b.as"),
            "\nleta a\numma kazi fb() -> Namba { rejesha 2 }",
        )
        .unwrap();
        let entry = module("leta a\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { }");
        let mut registry = InterfaceRegistry::new(root.clone());
        let errors = match resolve_all(&entry, &root, &BTreeMap::new(), &mut registry) {
            Err(errors) => errors,
            Ok(_) => panic!("a cycle must not resolve"),
        };
        fs::remove_dir_all(&root).ok();
        let cycle = errors.iter().find(|d| d.code == "RES001").expect("RES001");
        assert!(cycle.message.contains("a → b → a"), "{}", cycle.message);
        assert_eq!(
            cycle.span.as_ref().map(|s| s.line),
            Some(2),
            "points at b.as's `leta a`"
        );
    }

    #[test]
    fn dependency_order_reports_a_cycle_instead_of_panicking() {
        let with_import = |dep: &str| ResolvedModule {
            module: module(&format!("leta {dep}\numma kazi f() -> Tupu {{ }}")),
            exports: ExportTable::default(),
            is_stdlib: false,
        };
        let resolved: HashMap<String, ResolvedModule> = [
            ("x".to_string(), with_import("y")),
            ("y".to_string(), with_import("x")),
        ]
        .into_iter()
        .collect();
        let err = dependency_order(&resolved).expect_err("cycle");
        assert_eq!(err.code, "RES001");
    }

    #[test]
    fn an_import_shadows_an_ambient_name_but_two_imports_clash() {
        let mut prelude = StdlibEnv::default();
        prelude.constants.insert("PI".to_string(), ValueType::Namba);
        let resolved: HashMap<String, ResolvedModule> = [
            ("a".to_string(), exporting_constant("PI")),
            ("b".to_string(), exporting_constant("PI")),
        ]
        .into_iter()
        .collect();

        let one = module("leta a\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { }");
        assert!(check_duplicate_imports(&one, &resolved, &prelude).is_empty());

        let two = module("leta a\nleta b\nkazi kuu(hoja: Orodha<Neno>) -> Tupu { }");
        let errors = check_duplicate_imports(&two, &resolved, &prelude);
        assert!(errors.iter().any(|d| d.code == "SEM091"), "{errors:?}");
    }
}
