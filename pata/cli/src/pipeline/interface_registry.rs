//! Single source of truth for interface data (builtins and .asi files). Parse once, retrieve everywhere.

use crate::commands::CliError;
use crate::pipeline::builtin_modules;
use asili_parser::{parse_value_type, FnContract, ValueType};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where the interface came from (builtin or .asi file).
#[derive(Clone, Debug)]
pub enum InterfaceSource {
    Builtin,
    #[allow(dead_code)]
    File(PathBuf),
}

/// One module's interface: name, function/constant exports, source, fingerprint.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ModuleInterface {
    pub name: String,
    pub functions: HashMap<String, FnContract>,
    pub constants: HashMap<String, ValueType>,
    pub source: InterfaceSource,
    pub fingerprint: u64,
}

impl ModuleInterface {
    /// Build an ExportTable for use in ResolvedModule (caller uses resolve::ExportTable).
    pub fn to_export_table(&self) -> (HashMap<String, FnContract>, HashMap<String, ValueType>) {
        (self.functions.clone(), self.constants.clone())
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

fn parse_fn_sahihi(line: &str) -> Option<(String, FnContract)> {
    let tail = line.strip_prefix("kazi ")?;
    let open = tail.find('(')?;
    let close = tail.rfind(')')?;
    let name = tail[..open].trim().to_string();
    let params_raw = &tail[open + 1..close];
    let mut params = Vec::new();
    for p in params_raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let ty = if let Some((_, t)) = p.split_once(':') {
            parse_value_type(t.trim())
        } else {
            ValueType::Unknown
        };
        params.push(ty);
    }
    let ret = if let Some((_, r)) = tail[close + 1..].split_once("->") {
        parse_value_type(r.trim())
    } else {
        ValueType::Tupu
    };
    Some((name, FnContract { params, ret }))
}

fn parse_thabiti_line(line: &str) -> Option<(String, ValueType)> {
    let tail = line.strip_prefix("thabiti ")?.trim();
    let (name, type_str) = tail.split_once(':')?;
    let name = name.trim().to_string();
    let ty = parse_value_type(type_str.trim());
    Some((name, ty))
}

// TODO: parse_asi_content is a line-by-line text parser for .asi stub files, not a real AST parser.
// It cannot handle multi-line sahihi, generic constraints (kazi foo<T: Sifa>(...)), doc comments,
// or attribute lines (#[ndani]). Any .asi stub spanning multiple lines will be silently misparsed.
// Fix: run the real lexer+parser on .asi files and extract FnContract from the parsed Function nodes.
fn parse_asi_content(content: &str) -> (HashMap<String, FnContract>, HashMap<String, ValueType>) {
    let mut functions = HashMap::new();
    let mut constants = HashMap::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("kazi ") {
            if let Some((name, sig)) = parse_fn_sahihi(line) {
                functions.insert(name, sig);
            }
        } else if line.starts_with("thabiti ") {
            if let Some((name, ty)) = parse_thabiti_line(line) {
                constants.insert(name, ty);
            }
        }
    }
    (functions, constants)
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
            let table = builtin_modules::builtin_module_exports(name).expect("builtin export table");
            let iface = ModuleInterface {
                name: (*name).to_string(),
                functions: table.functions,
                constants: table.constants,
                source: InterfaceSource::Builtin,
                fingerprint: 0,
            };
            self.modules.insert((*name).to_string(), Arc::new(iface));
        }
    }

    /// Scan lib/std for *.asi and parse each; skip names already in registry (builtins win).
    pub fn load_stdlib(&mut self) -> Result<(), CliError> {
        let std_path = self.root.join("lib/std");
        if !std_path.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&std_path)
            .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", std_path.display()), 1))?
        {
            let entry = entry.map_err(|e| CliError::new(format!("hitilafu ya kiingilio: {e}"), 1))?;
            let path = entry.path();
            if path.extension().map(|e| e == "asi").unwrap_or(false) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if name.is_empty() || self.modules.contains_key(name) {
                    continue;
                }
                let content = fs::read_to_string(&path).map_err(|e| {
                    CliError::new(format!("imeshindwa kusoma {}: {e}", path.display()), 1)
                })?;
                let (functions, constants) = parse_asi_content(&content);
                let fp = fingerprint(&content);
                let iface = ModuleInterface {
                    name: name.to_string(),
                    functions,
                    constants,
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
    pub fn get_or_load(&mut self, name: &str, path: &Path) -> Result<Arc<ModuleInterface>, CliError> {
        if let Some(iface) = self.get(name) {
            return Ok(iface);
        }
        let content = fs::read_to_string(path).map_err(|e| {
            CliError::new(format!("imeshindwa kusoma {}: {e}", path.display()), 1)
        })?;
        let (functions, constants) = parse_asi_content(&content);
        let fp = fingerprint(&content);
        let iface = ModuleInterface {
            name: name.to_string(),
            functions,
            constants,
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

    /// Prelude only (msingi). Use as initial scope for semantic: prelude + imported modules.
    pub fn prelude_env(&self) -> StdlibEnv {
        let mut env = StdlibEnv::default();
        if let Some(iface) = self.modules.get("msingi") {
            for (k, v) in &iface.functions {
                env.functions.insert(k.clone(), v.clone());
            }
            for (k, v) in &iface.constants {
                env.constants.insert(k.clone(), v.clone());
            }
        }
        env
    }
}
