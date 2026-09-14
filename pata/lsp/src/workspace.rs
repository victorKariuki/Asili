//! Minimal, self-contained multi-file project awareness for the LSP.
//!
//! Everything in the rest of this crate (diagnostics, hover, goto-definition, references,
//! semantic tokens) historically operated on exactly one file's text in isolation — it had no
//! idea a project could span multiple `.as` files. That meant a legitimate `leta mathutil`
//! (importing your own sibling module) was flagged as an undefined-function error on every
//! call into it, and goto-definition/hover silently failed for anything not in the current
//! file's own AST.
//!
//! This module fixes the diagnostics half of that: it walks a project's import graph, parsing
//! every project-local `.as` file reachable from its entrypoint, and exposes both the resolved
//! module names (to silence the "unknown module" check) and each module's exported
//! function/constant signatures (to silence "undefined function" on legitimate cross-file
//! calls). Resolution is per-project, not per-VS-Code-workspace-folder: `find_project_root`
//! walks up from whatever file is being worked on to the nearest `pata.toml`, since one opened
//! folder can (and in this very repo, does — see `examples/*/pata.toml`) contain several
//! independent Asili projects with no `pata.toml` at the folder's own root at all. The LSP
//! caches one `WorkspaceIndex` per discovered project root rather than assuming there's only
//! ever one.
//!
//! Deliberately NOT built on top of `pata-cli`'s `pipeline::{resolve, interface_registry,
//! project}` — reusing that would mean either `pata-lsp` depending on `pata-cli` (impossible:
//! `pata-cli` is a binary-only crate with no `lib.rs`, and it already depends on `pata-lsp` to
//! launch it via `pata mwalimu`, so the reverse edge would cycle) or extracting that pipeline
//! into a new shared crate — a larger structural change than this fix needs, and one that
//! risks colliding with concurrent work already in flight on those exact files. This covers
//! path-based same-project imports only: a sibling `.as` file next to the entrypoint, or a
//! `pata.toml` path dependency's own `src/<name>.as`. Version-resolved / vendored registry
//! dependencies (anything needing `pata.lock`) are out of scope here.

use asili_lexer::tokenize;
use asili_parser::{parse_tokens, ImportPath, Module};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const STDLIB_MODULES: &[&str] = &[
    "msingi", "mfumo", "majira", "matumizi", "faili", "hisabati",
    "runtime", "syscall", "kiungo", "sambamba",
];

/// One resolved project-local module: its parsed AST plus the file it came from.
#[derive(Clone)]
pub struct WorkspaceModule {
    pub path: PathBuf,
    pub module: Module,
}

/// A snapshot of every project-local `.as` file reachable (transitively) from the entrypoint —
/// keyed by module name, i.e. the name used in `leta <name>`.
#[derive(Clone, Default)]
pub struct WorkspaceIndex {
    pub modules: HashMap<String, WorkspaceModule>,
    /// Inverted import edges: `reverse_deps[B]` is every module that directly `leta`s `B`.
    /// Used to answer "who could possibly reference a symbol defined in module B" for
    /// cross-file find-references, without searching every file in the project.
    pub reverse_deps: HashMap<String, HashSet<String>>,
}

impl WorkspaceIndex {
    pub fn find(&self, name: &str) -> Option<&WorkspaceModule> {
        self.modules.get(name)
    }

    /// Every module that transitively imports `name` (direct importers, importers of those,
    /// and so on) — the search scope for "find references to a symbol defined in `name`"
    /// beyond the file it's defined in. Does not include `name` itself.
    pub fn transitive_importers(&self, name: &str) -> HashSet<String> {
        let mut seen = HashSet::new();
        let mut queue: Vec<&str> = vec![name];
        while let Some(cur) = queue.pop() {
            let Some(direct) = self.reverse_deps.get(cur) else { continue };
            for importer in direct {
                if seen.insert(importer.clone()) {
                    queue.push(importer.as_str());
                }
            }
        }
        seen
    }

    /// Names of every project-local module this index knows about — for
    /// `semantic_check_with_env_and_modules`'s `resolved_modules` set, so SEM007 stops
    /// flagging legitimate same-project imports as unknown.
    pub fn resolved_module_names(&self) -> HashSet<String> {
        self.modules.keys().cloned().collect()
    }
}

/// Very small `pata.toml` reader: only `[chanzo] kuingia = "..."` (entrypoint) and
/// `[tegemezi]` path dependencies (`name = { path = "..." }`) — enough to seed resolution.
/// Version/registry dependencies aren't understood here (see module doc); a manifest that only
/// uses those still resolves fine, it just won't pull in anything beyond the entrypoint's own
/// sibling files.
fn read_entrypoint_and_path_deps(root: &Path) -> (PathBuf, HashMap<String, PathBuf>) {
    let default_entry = root.join("src").join("kuu.as");
    let manifest = root.join("pata.toml");
    let Ok(content) = std::fs::read_to_string(&manifest) else {
        return (default_entry, HashMap::new());
    };

    let mut entry = default_entry;
    let mut deps = HashMap::new();
    let mut section = String::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(|c| c == '[' || c == ']').to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        let value = value.trim();
        match section.as_str() {
            "chanzo" if key == "kuingia" => {
                entry = root.join(value.trim_matches('"'));
            }
            "tegemezi" => {
                // name = { path = "../other" } — only the path form is understood here.
                if let Some(path_start) = value.find("path") {
                    if let Some(quote_start) = value[path_start..].find('"') {
                        let after = &value[path_start + quote_start + 1..];
                        if let Some(quote_end) = after.find('"') {
                            let rel = &after[..quote_end];
                            deps.insert(key.to_string(), root.join(rel));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    (entry, deps)
}

/// Find the file backing `leta <name>`: a sibling `.as` file next to the importing file, or a
/// path dependency's own `src/<name>.as`.
fn find_module_file(name: &str, entry_dir: &Path, path_deps: &HashMap<String, PathBuf>) -> Option<PathBuf> {
    let sibling = entry_dir.join(format!("{name}.as"));
    if sibling.is_file() {
        return Some(sibling);
    }
    if let Some(dep_root) = path_deps.get(name) {
        let candidate = dep_root.join("src").join(format!("{name}.as"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Walk up from `file_path` (a source file, not necessarily a directory) looking for the
/// nearest `pata.toml` — that directory is the file's actual project root.
///
/// A single VS Code workspace folder is not necessarily one Asili project: this monorepo, for
/// instance, has no `pata.toml` at its own root, only nested ones per example
/// (`examples/cross_package/app/pata.toml`, etc). Resolving once against the workspace-folder
/// root and stopping there would silently find nothing for any file that isn't directly under
/// a project root that happens to coincide with the opened folder — exactly the false
/// "unknown module" this whole module exists to prevent, just one level removed. Mirrors
/// `findProjectRoot` in `extensions/vscode/src/extension.ts` (used there to find where to run
/// `pata jaribu` from) — keep the two in sync if this logic changes.
pub fn find_project_root(file_path: &Path) -> Option<PathBuf> {
    let mut dir = if file_path.is_dir() {
        file_path.to_path_buf()
    } else {
        file_path.parent()?.to_path_buf()
    };
    loop {
        if dir.join("pata.toml").is_file() {
            return Some(dir);
        }
        dir = dir.parent()?.to_path_buf();
    }
}

/// Walk the import graph from `root`'s entrypoint, parsing every project-local `.as` file it
/// (transitively) `leta`s. Synchronous, real disk I/O — callers on the async LSP runtime must
/// wrap this in `tokio::task::spawn_blocking` rather than calling it inline from a handler.
pub fn resolve_workspace(root: &Path) -> WorkspaceIndex {
    let (entry, path_deps) = read_entrypoint_and_path_deps(root);
    let mut modules = HashMap::new();
    let mut reverse_deps: HashMap<String, HashSet<String>> = HashMap::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut queue: Vec<PathBuf> = vec![entry];

    while let Some(path) = queue.pop() {
        let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !visited.insert(canon) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        let Ok(tokens) = tokenize(&source) else { continue };
        let Ok(module) = parse_tokens(&tokens) else { continue };

        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let entry_dir = path.parent().unwrap_or(root).to_path_buf();
        for imp in &module.imports {
            let mod_name = match &imp.path {
                ImportPath::Full(s) => s.as_str(),
                ImportPath::Selective { module, .. } => module.as_str(),
            };
            if STDLIB_MODULES.contains(&mod_name) {
                continue;
            }
            // Record the edge regardless of whether `mod_name` was already discovered via some
            // other path — only the *queueing* (to actually go parse it) is a one-time thing;
            // recording it only on first discovery would silently drop every edge except the
            // one that happened to resolve a module first, breaking transitive_importers for
            // any module imported from more than one place.
            reverse_deps.entry(mod_name.to_string()).or_default().insert(name.clone());
            if !modules.contains_key(mod_name) {
                if let Some(dep_path) = find_module_file(mod_name, &entry_dir, &path_deps) {
                    queue.push(dep_path);
                }
            }
        }

        modules.insert(name, WorkspaceModule { path, module });
    }

    WorkspaceIndex { modules, reverse_deps }
}
