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
//! File-finding now delegates to `pata_core::find_module_file` — the same lookup `pata-cli`'s
//! own compile pipeline uses, covering path dependencies, version dependencies resolved against
//! the vendored `.asili/packages/<name>/` cache, and stdlib `.asi` interface stubs. Before the
//! `pata-core` extraction, this module carried its own smaller reimplementation (sibling-file
//! and path-dependency lookup only) because `pata-lsp` couldn't depend on `pata-cli` (a
//! binary-only crate with no `lib.rs`, which itself depends on `pata-lsp` to launch `pata
//! mwalimu` — the reverse edge would have cycled) and extracting a shared crate was deferred.
//! `WorkspaceIndex`'s own shape (flat name→module map, reverse-dependency edges for
//! cross-file find-references) stays LSP-specific rather than adopting `pata_core::
//! ResolvedProgram`'s shape — the two serve different queries (this module answers "what
//! imports what," `ResolvedProgram` answers "give me one merged module ready to evaluate") and
//! forcing one into the other would touch every LSP feature that already depends on this type
//! (signature help, diagnostics, symbols, rename) for no behavioral gain.

use asili_lexer::tokenize;
use asili_parser::{parse_tokens, ImportPath, Module};
use pata_core::Dependency;
use std::collections::{BTreeMap, HashMap, HashSet};
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

/// Per-project-root cache of already-parsed modules, keyed by canonicalized file path — the
/// incremental-resolution fix for issue #25: `resolve_workspace` previously re-lexed and
/// re-parsed *every* file in a project's import graph on every call, even files that didn't
/// change. Held by `Backend` (`server.rs`) alongside its `WorkspaceIndex` cache and passed into
/// `resolve_workspace`, which checks a file's current content hash against the cached one before
/// doing any tokenize/parse work — a hit skips straight to reusing the cached `WorkspaceModule`.
/// No explicit "invalidate this file" call is needed: a changed file's new content simply won't
/// hash-match its cache entry on the next walk, and an unchanged file's will — this is naturally
/// immune to the staleness problem `DocStore::invalidate` exists for (a *derived* diagnostics
/// result that depends on other files' content too), since each entry here is keyed by its own
/// file's own content only.
#[derive(Clone, Default)]
pub struct ModuleCache {
    entries: HashMap<PathBuf, (u64, WorkspaceModule)>,
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
/// `[tegemezi]` entries (both `name = "1.2.3"` version strings and `name = { path = "..." }`
/// path tables) — enough to seed resolution. Produces a `BTreeMap<String, pata_core::Dependency>`
/// directly so the actual file-finding below can delegate to `pata_core::find_module_file`
/// (covering vendored version deps and stdlib `.asi` stubs too, not just path deps as before the
/// `pata-core` extraction).
fn read_entrypoint_and_deps(root: &Path) -> (PathBuf, BTreeMap<String, Dependency>) {
    let default_entry = root.join("src").join("kuu.as");
    let manifest = root.join("pata.toml");
    let Ok(content) = std::fs::read_to_string(&manifest) else {
        return (default_entry, BTreeMap::new());
    };

    let mut entry = default_entry;
    let mut deps: BTreeMap<String, Dependency> = BTreeMap::new();
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
                if let Some(path_start) = value.find("path") {
                    // name = { path = "../other" }
                    if let Some(quote_start) = value[path_start..].find('"') {
                        let after = &value[path_start + quote_start + 1..];
                        if let Some(quote_end) = after.find('"') {
                            let rel = &after[..quote_end];
                            deps.insert(key.to_string(), Dependency::Path(root.join(rel)));
                        }
                    }
                } else if value.starts_with('"') {
                    // name = "1.2.3" (or "^1.2", etc.) — a version dependency, resolved by
                    // pata_core::find_module_file against the vendored package cache.
                    let version = value.trim_matches('"').to_string();
                    deps.insert(key.to_string(), Dependency::Version(version));
                }
            }
            _ => {}
        }
    }
    (entry, deps)
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

/// The distinct set of project roots that `changed_paths` (a `workspace/didChangeWatchedFiles`
/// event's file list) could possibly affect — the incremental-invalidation scope for
/// `Backend::did_change_watched_files`: only these roots' cached `WorkspaceIndex` entries need
/// dropping, not the whole cache. A changed path outside any discoverable project (no `pata.toml`
/// anywhere above it) contributes nothing — correct, since nothing cached could depend on it.
pub fn affected_project_roots(changed_paths: &[PathBuf]) -> HashSet<PathBuf> {
    changed_paths
        .iter()
        .filter_map(|path| find_project_root(path))
        .collect()
}

/// Walk the import graph from `root`'s entrypoint, parsing every project-local `.as` file it
/// (transitively) `leta`s. Synchronous, real disk I/O — callers on the async LSP runtime must
/// wrap this in `tokio::task::spawn_blocking` rather than calling it inline from a handler.
///
/// `cache` (issue #25) is checked before tokenizing/parsing each file: a file whose current
/// on-disk content still hashes to what's cached is reused as-is, so a re-walk after one file
/// changes only actually re-parses that file (plus anything whose own AST changed as a result of
/// walking its imports again) rather than every file in the project. `cache` is updated in place
/// with every file this call touches (both hits and fresh parses), so the caller's persisted copy
/// stays current for the next call.
pub fn resolve_workspace(root: &Path, cache: &mut ModuleCache) -> WorkspaceIndex {
    let (entry, path_deps) = read_entrypoint_and_deps(root);
    let mut modules = HashMap::new();
    let mut reverse_deps: HashMap<String, HashSet<String>> = HashMap::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut queue: Vec<PathBuf> = vec![entry];

    while let Some(path) = queue.pop() {
        let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !visited.insert(canon.clone()) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        let hash = crate::doc_store::hash_text(&source);

        let wm = match cache.entries.get(&canon) {
            Some((cached_hash, cached_wm)) if *cached_hash == hash => cached_wm.clone(),
            _ => {
                #[cfg(test)]
                REPARSE_COUNT.with(|c| c.set(c.get() + 1));
                let Ok(tokens) = tokenize(&source) else { continue };
                let Ok(module) = parse_tokens(&tokens) else { continue };
                let wm = WorkspaceModule { path: path.clone(), module };
                cache.entries.insert(canon, (hash, wm.clone()));
                wm
            }
        };

        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        for imp in &wm.module.imports {
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
                // Always resolved from the project root, matching pata_core::resolve_one's own
                // (and therefore pata-cli's own) behavior exactly — not relative to whichever
                // file happens to be doing the importing. Covers path deps, version deps
                // resolved against the vendored `.asili/packages/<name>/` cache, and stdlib
                // `.asi` interface stubs, none of which the old per-file-relative lookup saw.
                if let Some((dep_path, _is_asi)) = pata_core::find_module_file(mod_name, root, &path_deps) {
                    queue.push(dep_path);
                }
            }
        }

        modules.insert(name, wm);
    }

    WorkspaceIndex { modules, reverse_deps }
}

// `#[cfg(test)]`-only counter of how many times `resolve_workspace`'s tokenize/parse branch
// actually ran (a real cache miss) — mirrors `DocStore::recompute_count`'s exact pattern, since
// a cache hit and a cache miss otherwise return externally-indistinguishable `WorkspaceModule`
// values (no `PartialEq`, no other observable side effect of a fresh parse).
#[cfg(test)]
thread_local! {
    static REPARSE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reparse_count() -> usize {
        REPARSE_COUNT.with(|c| c.get())
    }

    fn temp_project(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-lsp-ws-test-{name}-{stamp}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("pata.toml"),
            "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n",
        ).unwrap();
        std::fs::write(dir.join("src/kuu.as"), "kazi kuu() -> Tupu { }").unwrap();
        dir
    }

    /// The actual point of issue #25's fix: a file whose content hasn't changed since the last
    /// `resolve_workspace` call is served from `ModuleCache`'s existing entry rather than being
    /// re-tokenized/re-parsed. `resolve_workspace` still reads the file's bytes each call (that's
    /// unavoidable — computing a fresh content hash to compare against the cache is exactly how
    /// a change is detected at all, the same tradeoff `DocStore::diagnostics_for` makes for the
    /// same reason); the observable win is skipping the tokenize/parse work itself, proven here
    /// directly via `REPARSE_COUNT` (mirroring `DocStore::recompute_count`'s exact pattern) —
    /// two calls with byte-identical content must show exactly one real parse, not two.
    #[test]
    fn resolve_workspace_reuses_cached_module_without_reparsing_unchanged_file() {
        REPARSE_COUNT.with(|c| c.set(0));
        let project = temp_project("cache-reuse");
        let mut cache = ModuleCache::default();

        let first = resolve_workspace(&project, &mut cache);
        assert!(first.modules.contains_key("kuu"), "kuu.as should resolve on the first (real parse) call");
        assert_eq!(reparse_count(), 1, "the first call must be a real parse");

        let second = resolve_workspace(&project, &mut cache);
        assert!(second.modules.contains_key("kuu"), "kuu.as must still resolve on the second call");
        assert_eq!(reparse_count(), 1, "unchanged content on the second call must be a cache hit, not a second real parse");

        std::fs::remove_dir_all(&project).ok();
    }

    /// The other half of the same fix: a file whose content genuinely changed *must* be
    /// re-parsed, not served stale from the cache — proven by changing kuu.as to import a second
    /// module that didn't exist in the first call, and confirming the second call's
    /// `WorkspaceIndex` actually reflects the new import edge.
    #[test]
    fn resolve_workspace_reparses_a_file_whose_content_actually_changed() {
        REPARSE_COUNT.with(|c| c.set(0));
        let project = temp_project("cache-invalidate");
        // find_module_file (pata-core) resolves a plain sibling `leta msaidizi` against
        // `<root>/msaidizi.as`, not `<root>/src/msaidizi.as` — the project root, not next to
        // the entrypoint.
        std::fs::write(project.join("msaidizi.as"), "umma kazi f() -> Tupu { }").unwrap();
        let mut cache = ModuleCache::default();

        let first = resolve_workspace(&project, &mut cache);
        assert!(!first.modules.contains_key("msaidizi"), "msaidizi.as isn't imported yet, so it shouldn't be resolved");
        assert_eq!(reparse_count(), 1, "only kuu.as (the entrypoint) should be parsed on the first call");

        std::fs::write(project.join("src/kuu.as"), "leta msaidizi\nkazi kuu() -> Tupu { }").unwrap();
        let second = resolve_workspace(&project, &mut cache);
        assert!(
            second.modules.contains_key("msaidizi"),
            "kuu.as's real content change (adding `leta msaidizi`) must be picked up, not served from a stale cache entry"
        );
        assert_eq!(
            reparse_count(), 3,
            "kuu.as's changed content (1 more real parse) plus msaidizi.as being newly discovered and parsed for the first time (1 more) = 3 total real parses across both calls"
        );

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn affected_project_roots_finds_the_one_real_root_for_a_changed_file() {
        let project = temp_project("single");
        let changed = vec![project.join("src/kuu.as")];
        let roots = affected_project_roots(&changed);
        assert_eq!(roots.len(), 1);
        assert!(roots.contains(&project));
        std::fs::remove_dir_all(&project).ok();
    }

    /// The actual point of scoped invalidation: two independent projects changing shouldn't be
    /// conflated into one root, and a project nobody touched shouldn't appear in the result at
    /// all — proving the incremental fix's real behavior (only affected roots, not everything).
    #[test]
    fn affected_project_roots_distinguishes_two_independent_projects() {
        let project_a = temp_project("a");
        let project_b = temp_project("b");
        let untouched = temp_project("untouched");

        let changed = vec![project_a.join("src/kuu.as"), project_b.join("src/kuu.as")];
        let roots = affected_project_roots(&changed);

        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&project_a));
        assert!(roots.contains(&project_b));
        assert!(!roots.contains(&untouched), "a project with no changed files must not appear as affected");

        std::fs::remove_dir_all(&project_a).ok();
        std::fs::remove_dir_all(&project_b).ok();
        std::fs::remove_dir_all(&untouched).ok();
    }

    #[test]
    fn affected_project_roots_deduplicates_multiple_files_in_the_same_project() {
        let project = temp_project("dedup");
        std::fs::write(project.join("src/other.as"), "kazi f() -> Tupu { }").unwrap();

        let changed = vec![project.join("src/kuu.as"), project.join("src/other.as"), project.join("pata.toml")];
        let roots = affected_project_roots(&changed);

        assert_eq!(roots.len(), 1, "three changed files in one project should collapse to one root, got: {roots:?}");
        assert!(roots.contains(&project));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn affected_project_roots_ignores_a_file_outside_any_project() {
        let outside = std::env::temp_dir().join(format!("pata-lsp-ws-test-outside-{}", std::process::id()));
        std::fs::create_dir_all(&outside).unwrap();
        let stray_file = outside.join("stray.as");
        std::fs::write(&stray_file, "kazi f() -> Tupu { }").unwrap();

        let roots = affected_project_roots(&[stray_file]);
        assert!(roots.is_empty(), "a file with no pata.toml anywhere above it must not produce a root");

        std::fs::remove_dir_all(&outside).ok();
    }
}
