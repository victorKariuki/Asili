//! Single source of truth for interface data (builtins and .asi files). Parse once, retrieve everywhere.

use crate::builtin_modules;
use crate::Error;
use asili_parser::{
    parse_value_type, FnContract, Param, TraitDecl, TraitMethodSig, TypeExpr, ValueType,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// One required method signature inside a `.asi`-declared `sifa` — mirrors
/// `asili_parser::TraitMethodSig` but keeps this module's own plain-string param typing
/// (`.asi` has no `Param`/`TypeExpr` AST to reuse without pulling in the real parser).
#[derive(Clone, Debug, PartialEq)]
pub struct TraitMethodStub {
    pub name: String,
    pub params: Vec<ValueType>,
    pub ret: ValueType,
}

/// A `sifa` declared in a `.asi` interface stub: name + required method signatures.
#[derive(Clone, Debug)]
pub struct TraitStub {
    pub name: String,
    pub methods: Vec<TraitMethodStub>,
}

impl TraitStub {
    /// Convert into a real `TraitDecl` (line/column 0 — `.asi` stubs have no meaningful source
    /// location of their own in the consuming project) so callers can reuse the same
    /// completeness-checking logic (`check_trait_completeness`) that already walks
    /// `Module.traits`/`ImplDecl`, instead of a second parallel comparison.
    pub fn to_trait_decl(&self) -> TraitDecl {
        TraitDecl {
            name: self.name.clone(),
            methods: self
                .methods
                .iter()
                .map(|m| TraitMethodSig {
                    name: m.name.clone(),
                    params: m
                        .params
                        .iter()
                        .enumerate()
                        .map(|(i, ty)| Param {
                            name: format!("_{i}").into(),
                            ty: TypeExpr {
                                name: ty.to_string(),
                            },
                            line: 0,
                            column: 0,
                        })
                        .collect(),
                    return_type: TypeExpr {
                        name: m.ret.to_string(),
                    },
                    line: 0,
                })
                .collect(),
            line: 0,
            column: 0,
            attrs: vec![],
            is_public: true,
        }
    }
}

/// Where the interface came from (builtin or .asi file).
#[derive(Clone, Debug)]
pub enum InterfaceSource {
    Builtin,
    #[allow(dead_code)]
    File(PathBuf),
}

/// One module's interface: name, function/constant/trait exports, source, fingerprint.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ModuleInterface {
    pub name: String,
    pub functions: HashMap<String, FnContract>,
    pub constants: HashMap<String, ValueType>,
    pub traits: HashMap<String, TraitStub>,
    pub source: InterfaceSource,
    pub fingerprint: u64,
}

impl ModuleInterface {
    /// Build an ExportTable for use in ResolvedModule (caller uses resolve::ExportTable).
    pub fn to_export_table(&self) -> (HashMap<String, FnContract>, HashMap<String, ValueType>) {
        (self.functions.clone(), self.constants.clone())
    }

    /// Trait declarations this interface exports, as real `TraitDecl`s ready to drop into a
    /// `Module.traits` list (see `TraitStub::to_trait_decl`).
    pub fn trait_decls(&self) -> Vec<TraitDecl> {
        self.traits.values().map(TraitStub::to_trait_decl).collect()
    }
}

/// Stdlib environment: merged function and constant types from all registry modules (for semantic).
#[derive(Default, Clone, Debug)]
pub struct StdlibEnv {
    pub functions: HashMap<String, FnContract>,
    pub constants: HashMap<String, ValueType>,
}

/// Registry of module interfaces. Only place that parses .asi content.
pub struct InterfaceRegistry {
    root: PathBuf,
    modules: HashMap<String, Arc<ModuleInterface>>,
}

/// One `kazi` line of an `.asi` file — the signature syntax builtin tables use too.
fn parse_fn_sahihi(line: &str) -> Option<(String, FnContract)> {
    line.starts_with("kazi ").then_some(())?;
    asili_parser::builtins::parse_signature(line, &asili_parser::builtins::resolve_builtin_type)
}

fn parse_thabiti_line(line: &str) -> Option<(String, ValueType)> {
    let tail = line.strip_prefix("thabiti ")?.trim();
    let (name, type_str) = tail.split_once(':')?;
    let name = name.trim().to_string();
    let ty = parse_value_type(type_str.trim());
    Some((name, ty))
}

/// Parse one `kazi name(params) -> Ret` line found inside a `sifa { }` body — no bodies allowed,
/// same shape as a top-level function signature stub.
fn parse_trait_method_sahihi(line: &str) -> Option<TraitMethodStub> {
    let (name, contract) = parse_fn_sahihi(line)?;
    Some(TraitMethodStub {
        name,
        params: contract.params,
        ret: contract.ret,
    })
}

// TODO: parse_asi_content is a line-by-line text parser for .asi stub files, not a real AST parser.
// It cannot handle multi-line sahihi, generic constraints (kazi foo<T: Sifa>(...)), doc comments,
// or attribute lines (#[ndani]). Any .asi stub spanning multiple lines (other than the `sifa { }`
// block handled explicitly below) will be silently misparsed.
// Fix: run the real lexer+parser on .asi files and extract FnContract from the parsed Function nodes.
fn parse_asi_content(
    content: &str,
) -> (
    HashMap<String, FnContract>,
    HashMap<String, ValueType>,
    HashMap<String, TraitStub>,
) {
    let mut functions = HashMap::new();
    let mut constants = HashMap::new();
    let mut traits = HashMap::new();

    let mut lines = content.lines();
    // `#` lines right above a `kazi` are its description (LSP hover).
    let mut doc: Vec<&str> = Vec::new();
    while let Some(raw) = lines.next() {
        let line = raw.trim();
        if let Some(comment) = line.strip_prefix('#') {
            doc.push(comment.trim());
            continue;
        }
        let above = std::mem::take(&mut doc);
        if line.starts_with("kazi ") {
            if let Some((name, mut sig)) = parse_fn_sahihi(line) {
                sig.doc = above.join(" ");
                functions.insert(name, sig);
            }
        } else if line.starts_with("thabiti ") {
            if let Some((name, ty)) = parse_thabiti_line(line) {
                constants.insert(name, ty);
            }
        } else if let Some(rest) = line.strip_prefix("sifa ") {
            let name = rest.split('{').next().unwrap_or(rest).trim().to_string();
            if name.is_empty() {
                continue;
            }
            let mut methods = Vec::new();
            // A `sifa` with no `{` on its declaration line (forward-declared/empty trait, same
            // convention as the real parser's `parse_trait_method_sigs`) has no body to scan.
            if line.contains('{') {
                for body_raw in lines.by_ref() {
                    let body_line = body_raw.trim();
                    if body_line == "}" || body_line.starts_with('}') {
                        break;
                    }
                    if let Some(m) = parse_trait_method_sahihi(body_line) {
                        methods.push(m);
                    }
                }
            }
            traits.insert(name.clone(), TraitStub { name, methods });
        }
    }
    (functions, constants, traits)
}

fn fingerprint(content: &str) -> u64 {
    let mut h = DefaultHasher::new();
    content.hash(&mut h);
    h.finish()
}

impl InterfaceRegistry {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            modules: HashMap::new(),
        }
    }

    /// Register all builtin modules (msingi, mfumo, majira, matumizi, faili, hisabati). Call before load_stdlib.
    pub fn register_builtins(&mut self) {
        for name in builtin_modules::BUILTIN_MODULE_NAMES {
            if self.modules.contains_key(*name) {
                continue;
            }
            let table =
                builtin_modules::builtin_module_exports(name).expect("builtin export table");
            let iface = ModuleInterface {
                name: (*name).to_string(),
                functions: table.functions,
                constants: table.constants,
                traits: HashMap::new(),
                source: InterfaceSource::Builtin,
                fingerprint: 0,
            };
            self.modules.insert((*name).to_string(), Arc::new(iface));
        }
    }

    /// Scan lib/std for *.asi and parse each; skip names already in registry (builtins win).
    pub fn load_stdlib(&mut self) -> Result<(), Error> {
        let std_path = self.root.join("lib/std");
        if !std_path.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&std_path)
            .map_err(|e| Error::new(format!("imeshindwa kusoma {}: {e}", std_path.display())))?
        {
            let entry = entry.map_err(|e| Error::new(format!("hitilafu ya kiingilio: {e}")))?;
            let path = entry.path();
            if path.extension().map(|e| e == "asi").unwrap_or(false) {
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if name.is_empty() || self.modules.contains_key(name) {
                    continue;
                }
                let content = fs::read_to_string(&path).map_err(|e| {
                    Error::new(format!("imeshindwa kusoma {}: {e}", path.display()))
                })?;
                let (functions, constants, traits) = parse_asi_content(&content);
                let fp = fingerprint(&content);
                let iface = ModuleInterface {
                    name: name.to_string(),
                    functions,
                    constants,
                    traits,
                    source: InterfaceSource::File(path.clone()),
                    fingerprint: fp,
                };
                self.modules.insert(name.to_string(), Arc::new(iface));
            }
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<ModuleInterface>> {
        self.modules.get(name).cloned()
    }

    /// Load and parse an .asi file if not already in registry; return cached or new entry.
    pub fn get_or_load(&mut self, name: &str, path: &Path) -> Result<Arc<ModuleInterface>, Error> {
        if let Some(iface) = self.get(name) {
            return Ok(iface);
        }
        let content = fs::read_to_string(path)
            .map_err(|e| Error::new(format!("imeshindwa kusoma {}: {e}", path.display())))?;
        let (functions, constants, traits) = parse_asi_content(&content);
        let fp = fingerprint(&content);
        let iface = ModuleInterface {
            name: name.to_string(),
            functions,
            constants,
            traits,
            source: InterfaceSource::File(path.to_path_buf()),
            fingerprint: fp,
        };
        let arc = Arc::new(iface);
        self.modules.insert(name.to_string(), arc.clone());
        Ok(arc)
    }

    /// Build a single StdlibEnv by merging all registered modules (for duplicate checks).
    #[allow(dead_code)]
    pub fn stdlib_env(&self) -> StdlibEnv {
        let mut env = StdlibEnv::default();
        for iface in self.modules.values() {
            for (k, v) in &iface.functions {
                env.functions.insert(k.clone(), v.clone());
            }
            for (k, v) in &iface.constants {
                env.constants.insert(k.clone(), v.clone());
            }
        }
        env
    }

    /// Builtin stdlib environment (every builtin module except the opt-in ones). User and
    /// third-party modules remain explicit imports.
    pub fn prelude_env(&self) -> StdlibEnv {
        let mut env = StdlibEnv::default();
        for name in builtin_modules::ambient_module_names() {
            if let Some(iface) = self.modules.get(name) {
                for (k, v) in &iface.functions {
                    env.functions.insert(k.clone(), v.clone());
                }
                for (k, v) in &iface.constants {
                    env.constants.insert(k.clone(), v.clone());
                }
            }
        }
        env
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asili_parser::builtins::{render_stub, BUILTIN_MODULES};
    use std::path::Path;

    /// `lib/std/<m>.asi` is generated from the builtin table (`core/parser/src/builtins.rs`), so
    /// it cannot drift: this test fails when a file differs from what the table renders, and
    /// `ASILI_GOLDEN=write` rewrites them. A stub for a module that no longer exists fails too.
    #[test]
    fn stdlib_stubs_are_generated() {
        let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib/std");
        let write = std::env::var_os("ASILI_GOLDEN").is_some_and(|v| v == "write");
        let mut problems = Vec::new();
        for m in BUILTIN_MODULES {
            let path = std_dir.join(format!("{}.asi", m.name));
            let want = render_stub(m);
            if write {
                fs::write(&path, &want).expect("write stub");
            } else if fs::read_to_string(&path).ok().as_deref() != Some(want.as_str()) {
                problems.push(format!("lib/std/{}.asi", m.name));
            }
            // What the stub says is what the compiler knows.
            let (functions, constants, _) = parse_asi_content(&want);
            let table = builtin_modules::builtin_module_exports(m.name).expect("table");
            assert_eq!(functions.len(), table.functions.len(), "{}", m.name);
            for (name, real) in &table.functions {
                let stub = &functions[name];
                assert_eq!(
                    (
                        &stub.params,
                        &stub.ret,
                        &stub.names,
                        stub.optional,
                        stub.variadic,
                        &stub.doc
                    ),
                    (
                        &real.params,
                        &real.ret,
                        &real.names,
                        real.optional,
                        real.variadic,
                        &real.doc
                    ),
                    "{}::{name}",
                    m.name
                );
            }
            assert_eq!(constants, table.constants, "{}", m.name);
        }
        for entry in fs::read_dir(&std_dir).expect("lib/std") {
            let path = entry.expect("entry").path();
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if path.extension().is_some_and(|e| e == "asi")
                && !BUILTIN_MODULES.iter().any(|m| m.name == stem)
            {
                problems.push(format!("{} (no such builtin module)", path.display()));
            }
        }
        assert!(
            problems.is_empty(),
            "lib/std stubs differ from core/parser/src/builtins.rs; run \
             `ASILI_GOLDEN=write cargo test -p pata-core stdlib_stubs`:\n{}",
            problems.join("\n")
        );
    }
}
