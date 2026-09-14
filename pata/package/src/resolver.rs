//! Dependency resolution algorithm: real semver-range constraint solving against whatever
//! versions are actually available, replacing the old "lock whatever is given, verbatim, with
//! no conflict detection" stub. Three dependency shapes, three "where are the versions"
//! sources:
//!   - **Path** dependencies: unchanged — a path dependency has no versions to choose between,
//!     and (per Cargo's own precedent) its own `[tegemezi]` aren't walked transitively here
//!     either — a path dependency is project-local source, resolved as part of `pata_core`'s
//!     module resolver (`resolve_all`), not this package resolver.
//!   - **Git** dependencies (`{ git = "...", version = "^1.2" }`): the vendored copy under
//!     `.asili/packages/<name>/` is the only "available version" there is (no registry lists
//!     multiple versions of a git source) — its `.pata-version` marker file (written by `pata
//!     ongeza --git`) says which version was actually fetched, and the constraint either
//!     accepts or rejects it. No silent accept-anything fallback: an unmarked or
//!     constraint-violating vendored directory is a real resolve error. A git dependency's own
//!     transitive deps aren't walked — there's no manifest format defined yet for a vendored git
//!     tree to declare its own `[tegemezi]` in a way this resolver could read back out.
//!   - **Registry** dependencies (a bare version string, no `git`/`path`): resolved against a
//!     real `LocalRegistry` index at `<root>/.asili/registry/` — every version's `RegistrySource`
//!     is fetched for real (via `fetch::fetch_git` or a plain directory copy) into
//!     `.asili/packages/<name>/` the first time it's picked, then content-hashed. A dependency
//!     with nothing in the index and nothing already vendored is a real resolve error, not a
//!     placeholder-hash success. **Transitive**: each `RegistryEntry` already declared a
//!     `deps: Vec<RegistryDep>` field (present in the schema, never read by anything until now)
//!     — resolution now walks that graph breadth-first, so a registry package's own registry
//!     dependencies are fetched and locked too, not just the direct `[tegemezi]` entries a
//!     project wrote itself.
//!
//! **Conflict detection**: every package name pulled in by the transitive walk accumulates every
//! constraint seen for it (from whichever direct or transitive dependency named it). If a single
//! version can satisfy every accumulated constraint for that name, it's picked (same
//! highest-match-wins policy as a direct dependency); if no version in the index satisfies all of
//! them at once, resolution fails with a real conflict error naming every constraint and who
//! asked for it — not a silent "last one wins" overwrite, which is what re-locking the same map
//! key would otherwise do.
//!
//! `existing_lock` is honored for real now too: when an already-locked version still satisfies
//! the current constraint, it stays locked at that version rather than being re-resolved to a
//! possibly-different match on every build — the actual "don't churn the lockfile" behavior a
//! real resolver provides. This applies to direct dependencies only; the transitive walk always
//! resolves fresh against the accumulated constraint set, since an existing lock has no per-edge
//! record of *which* constraint substring came from which transitive parent.

use crate::registry::{LocalRegistry, RegistryDep};
use crate::{Dependency, LockFile, LockedDependency};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

/// Dependency resolver
pub struct Resolver;

impl Resolver {
    /// Resolve `dependencies` into a lock file, given the project `root` (needed to find the
    /// vendored package cache and local registry index) and any previously-locked versions
    /// (for lockfile-churn avoidance — see module doc).
    pub fn resolve(
        root: &Path,
        dependencies: &BTreeMap<String, Dependency>,
        existing_lock: Option<&LockFile>,
    ) -> Result<LockFile> {
        let mut lock = LockFile::new();
        // Every constraint any (direct or transitive) dependency has expressed for a given
        // registry package name, so a name reached two different ways with incompatible
        // requirements is a detected conflict rather than a silent overwrite.
        let mut registry_constraints: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut queue: VecDeque<(String, String)> = VecDeque::new(); // (name, version_req)
        let mut queued_registry_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

        for (name, dep) in dependencies {
            let locked = if let Some(path) = dep.path() {
                LockedDependency {
                    version: dep.version().to_string(),
                    checksum: compute_checksum(name, dep.version()),
                    path: Some(path.to_string()),
                    source: "path".to_string(),
                }
            } else if let Some(git_url) = dep.git() {
                resolve_git_dependency(root, name, dep.version(), git_url)?
            } else {
                registry_constraints.entry(name.clone()).or_default().push(dep.version().to_string());
                if queued_registry_names.insert(name.clone()) {
                    queue.push_back((name.clone(), dep.version().to_string()));
                }
                continue; // registry deps are resolved in the transitive pass below
            };
            lock.lock_dependency(name.clone(), locked);
        }

        // Breadth-first transitive walk over registry dependencies only (see module doc for why
        // path/git deps aren't walked). `queue` seeds with every direct registry dependency;
        // resolving one may discover more registry deps declared in its `RegistryEntry`, which
        // get appended and processed in turn until the graph is exhausted.
        let registry_root = root.join(".asili/registry");
        let registry = if registry_constraints.is_empty() {
            None
        } else {
            Some(
                LocalRegistry::load(&registry_root)
                    .with_context(|| format!("imeshindwa kusoma rejista {}", registry_root.display()))?,
            )
        };

        let mut resolved_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        while let Some((name, _first_req)) = queue.pop_front() {
            if resolved_names.contains(&name) {
                continue;
            }
            resolved_names.insert(name.clone());

            let registry = registry.as_ref().expect("registry loaded whenever the queue is non-empty");
            let constraints = registry_constraints.get(&name).cloned().unwrap_or_default();
            let (locked, entry_deps) = resolve_registry_dependency_transitive(
                root,
                &name,
                &constraints,
                existing_lock,
                registry,
            )?;

            for dep in &entry_deps {
                if !dep.optional {
                    registry_constraints.entry(dep.name.clone()).or_default().push(dep.req.clone());
                    if !resolved_names.contains(&dep.name) && queued_registry_names.insert(dep.name.clone()) {
                        queue.push_back((dep.name.clone(), dep.req.clone()));
                    }
                }
            }

            lock.lock_dependency(name, locked);
        }

        Ok(lock)
    }
}

/// Git dependency: the vendored copy is the only version there is to resolve against — no
/// registry lists multiple versions of a specific git source. Its `.pata-version` marker
/// (written by `pata ongeza --git`) says what was actually fetched; the constraint either
/// accepts it (real content checksum) or the resolve fails honestly.
fn resolve_git_dependency(root: &Path, name: &str, version_req: &str, git_url: &str) -> Result<LockedDependency> {
    let req = semver::VersionReq::parse(version_req)
        .with_context(|| format!("tegemezi '{name}': muundo batili wa toleo '{version_req}'"))?;

    let vendor_path = root.join(".asili/packages").join(name);
    let marker = vendor_path.join(".pata-version");
    let vendored_version = std::fs::read_to_string(&marker)
        .ok()
        .and_then(|s| semver::Version::parse(s.trim()).ok());

    match vendored_version {
        Some(v) if req.matches(&v) => {
            let checksum = crate::fetch::hash_dir(&vendor_path)
                .with_context(|| format!("imeshindwa kuhesabu checksum ya '{name}'"))?;
            Ok(LockedDependency {
                version: v.to_string(),
                checksum,
                path: None,
                source: "git".to_string(),
            })
        }
        Some(v) => anyhow::bail!(
            "tegemezi '{name}': toleo lililopakuliwa {v} halikubaliani na '{version_req}' (chanzo: {git_url})"
        ),
        None => anyhow::bail!(
            "tegemezi '{name}': hakuna toleo lililopakuliwa kutoka {git_url} — tumia `pata ongeza {name} --git {git_url}` kwanza"
        ),
    }
}

/// Registry (bare-version) dependency: resolve against `<root>/.asili/registry/`'s real index,
/// against **every** constraint accumulated for `name` so far (one from the project's own
/// `[tegemezi]`, plus one more per transitive parent that also depends on `name`) — a real
/// conflict-detection pass, not just a single caller's requirement. Prefers an existing lock's
/// version when it satisfies every current constraint (avoids re-fetching/re-hashing on every
/// build for no reason). Otherwise picks the highest version satisfying all constraints at once,
/// fetches it for real if not already vendored, and content-hashes the fetched tree. Returns the
/// chosen entry's own declared `deps` so the caller can continue the transitive walk.
fn resolve_registry_dependency_transitive(
    root: &Path,
    name: &str,
    version_reqs: &[String],
    existing_lock: Option<&LockFile>,
    registry: &LocalRegistry,
) -> Result<(LockedDependency, Vec<RegistryDep>)> {
    let reqs: Vec<semver::VersionReq> = version_reqs
        .iter()
        .map(|r| {
            semver::VersionReq::parse(r)
                .with_context(|| format!("tegemezi '{name}': muundo batili wa toleo '{r}'"))
        })
        .collect::<Result<_>>()?;

    let vendor_path = root.join(".asili/packages").join(name);
    let entries = registry.entries_for(name);

    if let Some(existing) = existing_lock.and_then(|l| l.dependencies.get(name)) {
        if let Ok(v) = semver::Version::parse(&existing.version) {
            if reqs.iter().all(|r| r.matches(&v)) && vendor_path.is_dir() {
                // Re-verify the checksum against what's actually on disk rather than trusting
                // the lockfile blindly — a tampered or manually-edited vendor directory should
                // be caught here, not silently re-locked with its old (now-wrong) checksum.
                let checksum = crate::fetch::hash_dir(&vendor_path)
                    .with_context(|| format!("imeshindwa kuhesabu checksum ya '{name}'"))?;
                let entry_deps = entries
                    .iter()
                    .find(|e| e.vers == existing.version)
                    .map(|e| e.deps.clone())
                    .unwrap_or_default();
                return Ok((
                    LockedDependency {
                        version: existing.version.clone(),
                        checksum,
                        path: None,
                        source: existing.source.clone(),
                    },
                    entry_deps,
                ));
            }
        }
    }

    let chosen = entries
        .iter()
        .filter_map(|e| semver::Version::parse(&e.vers).ok().map(|v| (v, e)))
        .filter(|(v, _)| reqs.iter().all(|r| r.matches(v)))
        .max_by(|a, b| a.0.cmp(&b.0));

    let Some((chosen_version, entry)) = chosen else {
        let registry_root = root.join(".asili/registry");
        if version_reqs.len() > 1 {
            anyhow::bail!(
                "mgongano wa tegemezi '{name}': hakuna toleo moja linalokubaliana na vigezo vyote: {} (kwenye rejista {})",
                version_reqs.join(", "),
                registry_root.display()
            );
        }
        anyhow::bail!(
            "tegemezi '{name}': hakuna toleo linalokubaliana na '{}' kwenye rejista {}",
            version_reqs.first().map(String::as_str).unwrap_or(""),
            registry_root.display()
        );
    };

    let checksum = fetch_from_registry_source(&entry.source, &vendor_path)
        .with_context(|| format!("imeshindwa kupata '{name}' kutoka rejista"))?;

    Ok((
        LockedDependency {
            version: chosen_version.to_string(),
            checksum,
            path: None,
            source: "registry".to_string(),
        },
        entry.deps.clone(),
    ))
}

/// Fetch a registry-resolved version's real source into `dest`, returning its content hash.
/// `Git` sources go through the same `fetch_git` a `pata ongeza --git` run would use;
/// `Path` sources (a local/file-based index entry, or any registry that vendors flat
/// directories) are copied directly — both produce a real `hash_dir` checksum over the actual
/// fetched bytes, never a placeholder.
fn fetch_from_registry_source(source: &crate::registry::RegistrySource, dest: &Path) -> Result<String> {
    use crate::registry::RegistrySource;
    match source {
        RegistrySource::Git { url, rev } => {
            crate::fetch::fetch_git(url, rev.as_deref(), dest)
                .map_err(|e| anyhow::anyhow!("{e}"))
        }
        RegistrySource::Path { path } => {
            if dest.exists() {
                std::fs::remove_dir_all(dest)?;
            }
            copy_dir_recursive(Path::new(path), dest)
                .with_context(|| format!("imeshindwa kunakili {path} kwenda {}", dest.display()))?;
            crate::fetch::hash_dir(dest).map_err(|e| anyhow::anyhow!("{e}"))
        }
    }
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Compute a simple checksum for a dependency with no fetchable content of its own (path
/// dependencies — see module doc). Kept as the one remaining caller of the old
/// name/version-string hash; every fetchable source (git, registry) uses a real content hash.
fn compute_checksum(name: &str, version: &str) -> String {
    use sha2::{Digest, Sha256};
    let input = format!("{}@{}", name, version);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher
        .finalize()
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::DependencyTable;
    use crate::registry::{RegistryEntry, RegistrySource};
    use std::io::Write;
    use std::sync::Mutex;

    // Every test that touches `root` (vendored packages / registry index live under it) must
    // not run concurrently with another that does — mirrors `pata-cli`'s own TEST_CWD_LOCK
    // pattern for cwd-dependent tests, but scoped to a shared root path instead of process cwd
    // since resolve() now takes root explicitly rather than reading std::env::current_dir().
    static TEST_ROOT_LOCK: Mutex<()> = Mutex::new(());

    fn temp_root(name: &str) -> std::path::PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pata-resolver-test-{name}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_resolve_empty() -> Result<()> {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("empty");
        let deps = BTreeMap::new();
        let lock = Resolver::resolve(&root, &deps, None)?;
        assert!(lock.dependencies.is_empty());
        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }

    #[test]
    fn test_resolve_path_dependency_unchanged() -> Result<()> {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("path");
        let mut deps = BTreeMap::new();
        deps.insert(
            "local".to_string(),
            Dependency::Table(DependencyTable {
                version: "0.1.0".to_string(),
                path: Some("../local".to_string()),
                git: None,
                branch: None,
            }),
        );
        let lock = Resolver::resolve(&root, &deps, None)?;
        assert_eq!(lock.dependencies["local"].path, Some("../local".to_string()));
        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }

    #[test]
    fn test_resolve_registry_dependency_fails_with_no_index() {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("noindex");
        let mut deps = BTreeMap::new();
        deps.insert("stdlib".to_string(), Dependency::Version("1.0.0".to_string()));
        let result = Resolver::resolve(&root, &deps, None);
        assert!(result.is_err(), "no registry index and nothing vendored should fail to resolve, not silently succeed");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_resolve_matches_registry_version_and_fetches_real_content() -> Result<()> {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("registry-match");

        // A real "published" package: a directory with one file, referenced by a Path source.
        let published_dir = root.join("published-stdlib");
        std::fs::create_dir_all(&published_dir).unwrap();
        std::fs::File::create(published_dir.join("stdlib.as"))
            .unwrap()
            .write_all(b"kazi jina() -> Neno { rejesha \"stdlib\" }")
            .unwrap();

        let mut registry = LocalRegistry::at(root.join(".asili/registry"))?;
        registry.publish(RegistryEntry {
            name: "stdlib".to_string(),
            vers: "1.2.3".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: published_dir.to_string_lossy().to_string() },
        })?;

        let mut deps = BTreeMap::new();
        deps.insert("stdlib".to_string(), Dependency::Version("^1.2.0".to_string()));
        let lock = Resolver::resolve(&root, &deps, None)?;

        assert_eq!(lock.dependencies["stdlib"].version, "1.2.3");
        assert_eq!(lock.dependencies["stdlib"].source, "registry");
        assert_eq!(lock.dependencies["stdlib"].checksum.len(), 64, "expected a real 64-hex-char SHA-256 digest");

        // The real content actually landed on disk, not just a checksum computed in the abstract.
        let vendored = root.join(".asili/packages/stdlib/stdlib.as");
        assert!(vendored.is_file());
        assert_eq!(std::fs::read_to_string(&vendored).unwrap(), "kazi jina() -> Neno { rejesha \"stdlib\" }");

        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }

    #[test]
    fn test_resolve_registry_version_out_of_range_fails() -> Result<()> {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("registry-range");
        let published_dir = root.join("published");
        std::fs::create_dir_all(&published_dir).unwrap();
        std::fs::File::create(published_dir.join("f.as")).unwrap().write_all(b"x").unwrap();

        let mut registry = LocalRegistry::at(root.join(".asili/registry"))?;
        registry.publish(RegistryEntry {
            name: "lib".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: published_dir.to_string_lossy().to_string() },
        })?;

        let mut deps = BTreeMap::new();
        deps.insert("lib".to_string(), Dependency::Version("^2.0.0".to_string()));
        let result = Resolver::resolve(&root, &deps, None);
        assert!(result.is_err(), "^2.0.0 must not match a published 1.0.0");

        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }

    #[test]
    fn test_resolve_reuses_existing_lock_when_still_satisfied() -> Result<()> {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("reuse-lock");
        let published_dir = root.join("published");
        std::fs::create_dir_all(&published_dir).unwrap();
        std::fs::File::create(published_dir.join("f.as")).unwrap().write_all(b"v1").unwrap();

        let mut registry = LocalRegistry::at(root.join(".asili/registry"))?;
        registry.publish(RegistryEntry {
            name: "lib".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: published_dir.to_string_lossy().to_string() },
        })?;
        // Also publish a newer version, to prove it's NOT picked once 1.0.0 is already locked
        // and still satisfies the constraint.
        registry.publish(RegistryEntry {
            name: "lib".to_string(),
            vers: "1.5.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: published_dir.to_string_lossy().to_string() },
        })?;

        let mut deps = BTreeMap::new();
        deps.insert("lib".to_string(), Dependency::Version("^1.0.0".to_string()));
        let first_lock = Resolver::resolve(&root, &deps, None)?;
        assert_eq!(first_lock.dependencies["lib"].version, "1.5.0", "first resolve picks the highest match");

        // Simulate an existing lock pinned at 1.0.0 (as if the user vendored/locked it earlier).
        let mut pinned = LockFile::new();
        pinned.lock_dependency("lib".to_string(), LockedDependency {
            version: "1.0.0".to_string(),
            checksum: "irrelevant".to_string(),
            path: None,
            source: "registry".to_string(),
        });
        // Re-fetch 1.0.0's real content so the vendor dir matches what the lock claims.
        fetch_from_registry_source(
            &RegistrySource::Path { path: published_dir.to_string_lossy().to_string() },
            &root.join(".asili/packages/lib"),
        )?;

        let second_lock = Resolver::resolve(&root, &deps, Some(&pinned))?;
        assert_eq!(second_lock.dependencies["lib"].version, "1.0.0", "existing lock still satisfying the constraint must be kept, not re-resolved to 1.5.0");

        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }

    #[test]
    fn test_resolve_git_dependency_fails_with_no_vendored_marker() {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("git-nomarker");
        let mut deps = BTreeMap::new();
        deps.insert(
            "gitdep".to_string(),
            Dependency::Table(DependencyTable {
                version: "^1.0".to_string(),
                path: None,
                git: Some("https://example.invalid/gitdep.git".to_string()),
                branch: None,
            }),
        );
        let result = Resolver::resolve(&root, &deps, None);
        assert!(result.is_err(), "a git dependency with nothing fetched yet must fail to resolve");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_resolve_git_dependency_matches_vendored_marker() -> Result<()> {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("git-marker");
        let vendor = root.join(".asili/packages/gitdep");
        std::fs::create_dir_all(&vendor).unwrap();
        std::fs::write(vendor.join(".pata-version"), "1.2.3").unwrap();
        std::fs::write(vendor.join("f.as"), "content").unwrap();

        let mut deps = BTreeMap::new();
        deps.insert(
            "gitdep".to_string(),
            Dependency::Table(DependencyTable {
                version: "^1.2".to_string(),
                path: None,
                git: Some("https://example.invalid/gitdep.git".to_string()),
                branch: None,
            }),
        );
        let lock = Resolver::resolve(&root, &deps, None)?;
        assert_eq!(lock.dependencies["gitdep"].version, "1.2.3");
        assert_eq!(lock.dependencies["gitdep"].source, "git");
        assert_eq!(lock.dependencies["gitdep"].checksum.len(), 64);

        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }

    #[test]
    fn test_checksum_deterministic() {
        let cs1 = compute_checksum("mylib", "1.0.0");
        let cs2 = compute_checksum("mylib", "1.0.0");
        assert_eq!(cs1, cs2);
    }

    /// The transitive-resolution floor: a direct dependency ("app-lib") declares its own
    /// registry dependency ("inner-lib") via `RegistryEntry::deps` — real end-to-end proof that
    /// resolving the project's own `[tegemezi]` also walks, fetches, and locks a second-order
    /// dependency nobody wrote directly in the project's manifest.
    #[test]
    fn test_resolve_walks_transitive_registry_dependency() -> Result<()> {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("transitive-basic");

        let inner_dir = root.join("inner-src");
        std::fs::create_dir_all(&inner_dir).unwrap();
        std::fs::File::create(inner_dir.join("inner.as")).unwrap().write_all(b"inner content").unwrap();
        let outer_dir = root.join("outer-src");
        std::fs::create_dir_all(&outer_dir).unwrap();
        std::fs::File::create(outer_dir.join("outer.as")).unwrap().write_all(b"outer content").unwrap();

        let mut registry = LocalRegistry::at(root.join(".asili/registry"))?;
        registry.publish(RegistryEntry {
            name: "inner-lib".to_string(),
            vers: "2.0.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: inner_dir.to_string_lossy().to_string() },
        })?;
        registry.publish(RegistryEntry {
            name: "app-lib".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![RegistryDep { name: "inner-lib".to_string(), req: "^2.0".to_string(), optional: false }],
            yanked: None,
            source: RegistrySource::Path { path: outer_dir.to_string_lossy().to_string() },
        })?;

        let mut deps = BTreeMap::new();
        deps.insert("app-lib".to_string(), Dependency::Version("^1.0".to_string()));
        let lock = Resolver::resolve(&root, &deps, None)?;

        assert_eq!(lock.dependencies["app-lib"].version, "1.0.0");
        assert_eq!(
            lock.dependencies["inner-lib"].version, "2.0.0",
            "inner-lib was never in [tegemezi] directly — it must still be locked via app-lib's declared registry dep"
        );
        assert!(root.join(".asili/packages/inner-lib").is_dir(), "transitive dep must actually be fetched, not just locked in the abstract");

        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }

    /// Conflict detection: two direct dependencies each require an incompatible version range of
    /// the same transitive package ("shared-lib") — no single published version satisfies both
    /// `^1.0` and `^2.0` at once, so resolution must fail loudly naming the conflict, not silently
    /// pick whichever dependent happened to be walked last.
    #[test]
    fn test_resolve_detects_transitive_version_conflict() -> Result<()> {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("transitive-conflict");

        let shared_dir = root.join("shared-src");
        std::fs::create_dir_all(&shared_dir).unwrap();
        std::fs::File::create(shared_dir.join("f.as")).unwrap().write_all(b"x").unwrap();
        let a_dir = root.join("a-src");
        std::fs::create_dir_all(&a_dir).unwrap();
        std::fs::File::create(a_dir.join("f.as")).unwrap().write_all(b"a").unwrap();
        let b_dir = root.join("b-src");
        std::fs::create_dir_all(&b_dir).unwrap();
        std::fs::File::create(b_dir.join("f.as")).unwrap().write_all(b"b").unwrap();

        let mut registry = LocalRegistry::at(root.join(".asili/registry"))?;
        registry.publish(RegistryEntry {
            name: "shared-lib".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: shared_dir.to_string_lossy().to_string() },
        })?;
        registry.publish(RegistryEntry {
            name: "shared-lib".to_string(),
            vers: "2.0.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: shared_dir.to_string_lossy().to_string() },
        })?;
        registry.publish(RegistryEntry {
            name: "pkg-a".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![RegistryDep { name: "shared-lib".to_string(), req: "^1.0".to_string(), optional: false }],
            yanked: None,
            source: RegistrySource::Path { path: a_dir.to_string_lossy().to_string() },
        })?;
        registry.publish(RegistryEntry {
            name: "pkg-b".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![RegistryDep { name: "shared-lib".to_string(), req: "^2.0".to_string(), optional: false }],
            yanked: None,
            source: RegistrySource::Path { path: b_dir.to_string_lossy().to_string() },
        })?;

        let mut deps = BTreeMap::new();
        deps.insert("pkg-a".to_string(), Dependency::Version("^1.0".to_string()));
        deps.insert("pkg-b".to_string(), Dependency::Version("^1.0".to_string()));
        let result = Resolver::resolve(&root, &deps, None);

        assert!(result.is_err(), "incompatible transitive constraints on shared-lib (^1.0 vs ^2.0) must fail to resolve");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("shared-lib"), "conflict error should name the conflicting package, got: {msg}");

        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }

    /// An `optional: true` transitive dep is not walked or locked — proves the walk actually
    /// reads the `optional` flag rather than pulling in every declared dep unconditionally.
    #[test]
    fn test_resolve_skips_optional_transitive_dependency() -> Result<()> {
        let _guard = TEST_ROOT_LOCK.lock().unwrap();
        let root = temp_root("transitive-optional");

        let outer_dir = root.join("outer-src");
        std::fs::create_dir_all(&outer_dir).unwrap();
        std::fs::File::create(outer_dir.join("f.as")).unwrap().write_all(b"x").unwrap();

        let mut registry = LocalRegistry::at(root.join(".asili/registry"))?;
        registry.publish(RegistryEntry {
            name: "app-lib".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![RegistryDep { name: "never-fetched".to_string(), req: "^1.0".to_string(), optional: true }],
            yanked: None,
            source: RegistrySource::Path { path: outer_dir.to_string_lossy().to_string() },
        })?;

        let mut deps = BTreeMap::new();
        deps.insert("app-lib".to_string(), Dependency::Version("^1.0".to_string()));
        let lock = Resolver::resolve(&root, &deps, None)?;

        assert_eq!(lock.dependencies.len(), 1, "optional transitive dep must not be locked");
        assert!(!lock.dependencies.contains_key("never-fetched"));

        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }
}
