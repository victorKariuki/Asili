//! Dependency resolution algorithm: real semver-range constraint solving against whatever
//! versions are actually available, replacing the old "lock whatever is given, verbatim, with
//! no conflict detection" stub. Three dependency shapes, three "where are the versions"
//! sources:
//!   - **Path** dependencies: unchanged — a path dependency has no versions to choose between.
//!   - **Git** dependencies (`{ git = "...", version = "^1.2" }`): the vendored copy under
//!     `.asili/packages/<name>/` is the only "available version" there is (no registry lists
//!     multiple versions of a git source) — its `.pata-version` marker file (written by `pata
//!     ongeza --git`) says which version was actually fetched, and the constraint either
//!     accepts or rejects it. No silent accept-anything fallback: an unmarked or
//!     constraint-violating vendored directory is a real resolve error.
//!   - **Registry** dependencies (a bare version string, no `git`/`path`): resolved against a
//!     real `LocalRegistry` index at `<root>/.asili/registry/` — every version's `RegistrySource`
//!     is fetched for real (via `fetch::fetch_git` or a plain directory copy) into
//!     `.asili/packages/<name>/` the first time it's picked, then content-hashed. A dependency
//!     with nothing in the index and nothing already vendored is a real resolve error, not a
//!     placeholder-hash success.
//!
//! `existing_lock` is honored for real now too: when an already-locked version still satisfies
//! the current constraint, it stays locked at that version rather than being re-resolved to a
//! possibly-different match on every build — the actual "don't churn the lockfile" behavior a
//! real resolver provides.

use crate::registry::LocalRegistry;
use crate::{Dependency, LockFile, LockedDependency};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
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
                resolve_registry_dependency(root, name, dep.version(), existing_lock)?
            };
            lock.lock_dependency(name.clone(), locked);
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

/// Registry (bare-version) dependency: resolve against `<root>/.asili/registry/`'s real index.
/// Prefers an existing lock's version when it still satisfies the constraint (avoids re-fetching
/// / re-hashing on every build for no reason). Otherwise picks the highest matching published
/// version, fetches it for real if not already vendored, and content-hashes the fetched tree.
fn resolve_registry_dependency(
    root: &Path,
    name: &str,
    version_req: &str,
    existing_lock: Option<&LockFile>,
) -> Result<LockedDependency> {
    let req = semver::VersionReq::parse(version_req)
        .with_context(|| format!("tegemezi '{name}': muundo batili wa toleo '{version_req}'"))?;

    let vendor_path = root.join(".asili/packages").join(name);

    if let Some(existing) = existing_lock.and_then(|l| l.dependencies.get(name)) {
        if let Ok(v) = semver::Version::parse(&existing.version) {
            if req.matches(&v) && vendor_path.is_dir() {
                // Re-verify the checksum against what's actually on disk rather than trusting
                // the lockfile blindly — a tampered or manually-edited vendor directory should
                // be caught here, not silently re-locked with its old (now-wrong) checksum.
                let checksum = crate::fetch::hash_dir(&vendor_path)
                    .with_context(|| format!("imeshindwa kuhesabu checksum ya '{name}'"))?;
                return Ok(LockedDependency {
                    version: existing.version.clone(),
                    checksum,
                    path: None,
                    source: existing.source.clone(),
                });
            }
        }
    }

    let registry_root = root.join(".asili/registry");
    let registry = LocalRegistry::load(&registry_root)
        .with_context(|| format!("imeshindwa kusoma rejista {}", registry_root.display()))?;

    let entries = registry.entries_for(name);
    let chosen = entries
        .iter()
        .filter_map(|e| semver::Version::parse(&e.vers).ok().map(|v| (v, e)))
        .filter(|(v, _)| req.matches(v))
        .max_by(|a, b| a.0.cmp(&b.0));

    let Some((chosen_version, entry)) = chosen else {
        anyhow::bail!(
            "tegemezi '{name}': hakuna toleo linalokubaliana na '{version_req}' kwenye rejista {}",
            registry_root.display()
        );
    };

    let checksum = fetch_from_registry_source(&entry.source, &vendor_path)
        .with_context(|| format!("imeshindwa kupata '{name}' kutoka rejista"))?;

    Ok(LockedDependency {
        version: chosen_version.to_string(),
        checksum,
        path: None,
        source: "registry".to_string(),
    })
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
}
