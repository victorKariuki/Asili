//! Lock file management for reproducible builds

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use sha2::{Sha256, Digest};

/// Lock file for pinned dependency versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    pub version: String,
    pub locked_at: String,
    /// External dependencies
    pub dependencies: BTreeMap<String, LockedDependency>,
    /// Local workspace packages
    #[serde(default)]
    pub local_packages: BTreeMap<String, LocalPackage>,
}

/// A locked (resolved) dependency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedDependency {
    pub version: String,
    pub checksum: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub source: String,
}

/// A local workspace package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPackage {
    pub version: String,
    pub checksum: String,
    pub path: String,
}

/// One dependency whose vendored content no longer matches what `pata.lock` recorded at fetch
/// time — returned by [`LockFile::verify_content_integrity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityMismatch {
    pub name: String,
    pub expected: String,
    pub actual: String,
}

impl LockFile {
    /// Create a new lock file
    pub fn new() -> Self {
        Self {
            version: "1".to_string(),
            locked_at: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            dependencies: BTreeMap::new(),
            local_packages: BTreeMap::new(),
        }
    }

    /// Load lock file from pata.lock
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| "imeshindwa kusoma pata.lock")?;
        Self::from_str(&content)
    }

    /// Parse lock file from TOML string
    pub fn from_str(content: &str) -> Result<Self> {
        toml::from_str(content).with_context(|| "hitilafu ya kuchambua pata.lock")
    }

    /// Write lock file to pata.lock
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "hitilafu ya kubadili pata.lock kuwa TOML")?;
        fs::write(&path, content)
            .with_context(|| "imeshindwa kuandika pata.lock")
    }

    /// Add a locked dependency
    pub fn lock_dependency(&mut self, name: String, dep: LockedDependency) {
        self.dependencies.insert(name, dep);
    }

    /// Check if a dependency is locked
    pub fn is_locked(&self, name: &str) -> bool {
        self.dependencies.contains_key(name)
    }

    /// Add a local package to the lock
    pub fn lock_local_package(&mut self, name: String, pkg: LocalPackage) {
        self.local_packages.insert(name, pkg);
    }

    /// Get a local package by name
    pub fn get_local_package(&self, name: &str) -> Option<&LocalPackage> {
        self.local_packages.get(name)
    }

    /// Check if a local package is locked
    pub fn has_local_package(&self, name: &str) -> bool {
        self.local_packages.contains_key(name)
    }

    /// Get all dependencies (both external and local)
    pub fn all_dependencies(&self) -> impl Iterator<Item = &str> {
        self.dependencies
            .keys()
            .chain(self.local_packages.keys())
            .map(|s| s.as_str())
    }

    /// Re-hash every vendored (git/registry) dependency's on-disk content under `root` and
    /// compare against the checksum this lock file recorded at fetch time — the actual security
    /// property a lockfile exists for: detect a `.asili/packages/<name>/` directory that was
    /// swapped or tampered with after `pata ongeza` fetched it. Path dependencies are skipped
    /// (their checksum is a placeholder over `name@version`, not fetched content — they're live
    /// local source, not something to protect against tampering). Returns the names of every
    /// dependency whose current content hash no longer matches the lock.
    pub fn verify_content_integrity(&self, root: &Path) -> Result<Vec<IntegrityMismatch>> {
        let paths = crate::paths::Paths::new(root);
        let mut mismatches = Vec::new();
        for (name, dep) in &self.dependencies {
            if dep.path.is_some() {
                continue;
            }
            let vendor_dir = paths.package_path(name);
            if !vendor_dir.is_dir() {
                // Nothing vendored yet (e.g. a fresh checkout before the first `pata ongeza`
                // fetch) — not a tamper signal, just not fetched yet. Resolution/build will
                // surface its own "not found" error separately.
                continue;
            }
            let actual = crate::fetch::hash_dir(&vendor_dir)
                .with_context(|| format!("imeshindwa kuhesabu hashi ya {}", vendor_dir.display()))?;
            if actual != dep.checksum {
                mismatches.push(IntegrityMismatch {
                    name: name.clone(),
                    expected: dep.checksum.clone(),
                    actual,
                });
            }
        }
        Ok(mismatches)
    }

    /// Update workspace checksum (for detecting changes)
    pub fn workspace_checksum(&self) -> String {
        let mut hasher = Sha256::new();

        // Hash all dependency checksums
        for (name, dep) in &self.dependencies {
            hasher.update(name.as_bytes());
            hasher.update(&dep.checksum);
        }

        // Hash all local package checksums
        for (name, pkg) in &self.local_packages {
            hasher.update(name.as_bytes());
            hasher.update(&pkg.checksum);
        }

        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    }
}

impl Default for LockFile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_file_create() {
        let lock = LockFile::new();
        assert_eq!(lock.version, "1");
        assert!(lock.dependencies.is_empty());
    }

    #[test]
    fn test_lock_dependency() {
        let mut lock = LockFile::new();
        lock.lock_dependency(
            "stdlib".to_string(),
            LockedDependency {
                version: "1.0.0".to_string(),
                checksum: "abc123".to_string(),
                path: None,
                source: "registry".to_string(),
            },
        );
        assert!(lock.is_locked("stdlib"));
        assert!(!lock.is_locked("nonexistent"));
    }

    #[test]
    fn test_local_packages() {
        let mut lock = LockFile::new();
        lock.lock_local_package(
            "math-utils".to_string(),
            LocalPackage {
                version: "1.0.0".to_string(),
                checksum: "def456".to_string(),
                path: "lib/math-utils".to_string(),
            },
        );

        assert!(lock.has_local_package("math-utils"));
        assert!(!lock.has_local_package("nonexistent"));

        let pkg = lock.get_local_package("math-utils").unwrap();
        assert_eq!(pkg.version, "1.0.0");
        assert_eq!(pkg.path, "lib/math-utils");
    }

    #[test]
    fn test_all_dependencies() {
        let mut lock = LockFile::new();
        lock.lock_dependency(
            "uuid".to_string(),
            LockedDependency {
                version: "1.0.0".to_string(),
                checksum: "abc123".to_string(),
                path: None,
                source: "registry".to_string(),
            },
        );
        lock.lock_local_package(
            "math-utils".to_string(),
            LocalPackage {
                version: "1.0.0".to_string(),
                checksum: "def456".to_string(),
                path: "lib/math-utils".to_string(),
            },
        );

        let all_deps: Vec<_> = lock.all_dependencies().collect();
        assert_eq!(all_deps.len(), 2);
        assert!(all_deps.contains(&"uuid"));
        assert!(all_deps.contains(&"math-utils"));
    }

    #[test]
    fn test_workspace_checksum() {
        let mut lock = LockFile::new();
        lock.lock_dependency(
            "uuid".to_string(),
            LockedDependency {
                version: "1.0.0".to_string(),
                checksum: "abc123".to_string(),
                path: None,
                source: "registry".to_string(),
            },
        );

        let checksum1 = lock.workspace_checksum();
        assert!(!checksum1.is_empty());

        // Add another dependency
        lock.lock_local_package(
            "math-utils".to_string(),
            LocalPackage {
                version: "1.0.0".to_string(),
                checksum: "def456".to_string(),
                path: "lib/math-utils".to_string(),
            },
        );

        let checksum2 = lock.workspace_checksum();
        // Checksums should be different after adding a package
        assert_ne!(checksum1, checksum2);
    }
}
