//! Package registry: a minimal, self-hostable index — a directory of one JSON file per package
//! (`<index_root>/<name>.json`, containing every published version as a `RegistryEntry`) plus a
//! real fetchable `source` per version, instead of a hosted API server. Modeled on Cargo's
//! alternative-registry RFC (a JSON index + fetchable location per version, no API server
//! required) and the Zig precedent of content-hash-pinned git/tarball sources with no central
//! registry — see `docs/design/pata-production-readiness.md`. `Resolver::resolve` reads this
//! index (when a dependency isn't a path/git source) to discover which versions exist and where
//! to fetch each one from.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Package metadata summary — not itself the index's on-disk unit (that's `Vec<RegistryEntry>`
/// per package, one JSON file), but a convenient merged view over it (`LocalRegistry::lookup`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name
    pub name: String,
    /// Available versions (semantic versions)
    pub versions: Vec<String>,
    /// Latest version
    pub latest: String,
    /// Brief description
    pub description: Option<String>,
}

/// Where a specific published version's real source lives, fetchable by
/// `pata_package::fetch_git` (`Git`) or a plain filesystem copy (`Path` — the source for a
/// `file://`-style local index entry, or any registry that vendors flat directories instead of
/// git repos).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum RegistrySource {
    Git { url: String, rev: Option<String> },
    Path { path: String },
}

/// Registry index entry (compatible with Cargo/crates.io format), extended with a `source` so
/// the index is actually fetchable — crates.io's own format omits this because its API server
/// derives the download URL from `name`+`vers` implicitly, which a server-less static index has
/// no equivalent for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Package name (lowercase)
    pub name: String,
    /// Version number
    pub vers: String,
    /// Dependencies with version constraints
    #[serde(default)]
    pub deps: Vec<RegistryDep>,
    /// Package yanked/deprecated?
    pub yanked: Option<bool>,
    /// Where to fetch this exact version's real source from.
    pub source: RegistrySource,
}

/// Dependency entry in registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryDep {
    /// Dependency name
    pub name: String,
    /// Version requirement (semver constraint)
    pub req: String,
    /// Is it optional?
    #[serde(default)]
    pub optional: bool,
}

/// A registry index backed by a real directory on disk: `<root>/<name>.json` holds
/// `Vec<RegistryEntry>` — every published, non-yanked version of `<name>` plus where to fetch
/// each one. `LocalRegistry::load` reads every `*.json` file under `root` eagerly (a static
/// index is expected to be small enough for this — the same assumption Cargo's sparse index
/// makes per-crate, just applied to the whole index at once here since there's no HTTP fetch
/// step to make lazy loading worthwhile for a local/file:// index).
pub struct LocalRegistry {
    root: Option<PathBuf>,
    packages: BTreeMap<String, PackageMetadata>,
    /// Every version's full entry (including its `source`), keyed by package name — `packages`
    /// alone (versions as bare strings) isn't enough to answer "where do I fetch this from."
    entries: BTreeMap<String, Vec<RegistryEntry>>,
}

impl LocalRegistry {
    /// Create an empty, in-memory-only registry (no `root` — `publish`/`save` have nothing to
    /// write to; use `LocalRegistry::load` or `LocalRegistry::at` for a real on-disk index).
    pub fn new() -> Self {
        Self {
            root: None,
            packages: BTreeMap::new(),
            entries: BTreeMap::new(),
        }
    }

    /// An empty registry backed by `root` on disk (created if it doesn't exist yet) — `publish`
    /// writes real `<root>/<name>.json` files from here on.
    pub fn at(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)
            .with_context(|| format!("imeshindwa kuunda saraka ya rejista {}", root.display()))?;
        Ok(Self {
            root: Some(root),
            packages: BTreeMap::new(),
            entries: BTreeMap::new(),
        })
    }

    /// Load every package's index file (`*.json`) from a real directory on disk. Returns an
    /// empty registry (not an error) if `root` doesn't exist yet — a project with no local
    /// registry configured should resolve as "nothing found there," not fail outright.
    pub fn load(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut reg = Self { root: Some(root.clone()), packages: BTreeMap::new(), entries: BTreeMap::new() };
        if !root.is_dir() {
            return Ok(reg);
        }
        for entry in std::fs::read_dir(&root)
            .with_context(|| format!("imeshindwa kusoma rejista {}", root.display()))?
        {
            let entry = entry.with_context(|| "hitilafu ya kiingilio cha rejista")?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("imeshindwa kusoma {}", path.display()))?;
            let versions: Vec<RegistryEntry> = serde_json::from_str(&content)
                .with_context(|| format!("muundo batili wa JSON: {}", path.display()))?;
            if let Some(first) = versions.first() {
                reg.index_package(first.name.clone(), versions);
            }
        }
        Ok(reg)
    }

    fn index_package(&mut self, name: String, mut versions: Vec<RegistryEntry>) {
        versions.retain(|v| !v.yanked.unwrap_or(false));
        let version_strings: Vec<String> = versions.iter().map(|v| v.vers.clone()).collect();
        let latest = versions
            .iter()
            .filter_map(|v| semver::Version::parse(&v.vers).ok())
            .max()
            .map(|v| v.to_string())
            .unwrap_or_default();
        self.packages.insert(
            name.clone(),
            PackageMetadata {
                name: name.clone(),
                versions: version_strings,
                latest,
                description: None,
            },
        );
        self.entries.insert(name, versions);
    }

    /// Register a package summary directly (in-memory only — does not write to disk; use
    /// `publish` for a real on-disk-index entry with a fetchable source).
    pub fn register(&mut self, metadata: PackageMetadata) -> Result<()> {
        self.packages.insert(metadata.name.clone(), metadata);
        Ok(())
    }

    /// Publish one real version entry: adds it to the in-memory index and, when this registry
    /// was opened via `at`/`load` (has a real `root`), writes/updates `<root>/<name>.json` on
    /// disk so it's discoverable by a later `LocalRegistry::load` — the actual "make a version
    /// available to resolve against" operation for a file-based index.
    pub fn publish(&mut self, entry: RegistryEntry) -> Result<()> {
        let name = entry.name.clone();
        let mut versions = self.entries.remove(&name).unwrap_or_default();
        versions.retain(|v| v.vers != entry.vers); // replace, don't duplicate, on re-publish
        versions.push(entry);
        self.index_package(name.clone(), versions.clone());

        if let Some(root) = &self.root {
            let path = root.join(format!("{name}.json"));
            let content = serde_json::to_string_pretty(&versions)
                .with_context(|| "imeshindwa kubadili rejista kuwa JSON")?;
            std::fs::write(&path, content)
                .with_context(|| format!("imeshindwa kuandika {}", path.display()))?;
        }
        Ok(())
    }

    /// Look up a package's version summary.
    pub fn lookup(&self, name: &str) -> Option<&PackageMetadata> {
        self.packages.get(name)
    }

    /// Every published (non-yanked) version entry for `name`, including each one's fetchable
    /// `source` — what `Resolver::resolve` actually needs beyond the bare version list
    /// `lookup`/`PackageMetadata` gives.
    pub fn entries_for(&self, name: &str) -> &[RegistryEntry] {
        self.entries.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// List all packages
    pub fn list_packages(&self) -> impl Iterator<Item = &PackageMetadata> {
        self.packages.values()
    }
}

impl Default for LocalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lookup() {
        let mut reg = LocalRegistry::new();
        let meta = PackageMetadata {
            name: "jitu".to_string(),
            versions: vec!["1.0.0".to_string()],
            latest: "1.0.0".to_string(),
            description: Some("Math library".to_string()),
        };
        reg.register(meta).unwrap();

        let found = reg.lookup("jitu");
        assert!(found.is_some());
        assert_eq!(found.unwrap().latest, "1.0.0");
    }

    #[test]
    fn registry_list() {
        let mut reg = LocalRegistry::new();
        reg.register(PackageMetadata {
            name: "jitu".to_string(),
            versions: vec!["1.0.0".to_string()],
            latest: "1.0.0".to_string(),
            description: None,
        })
        .unwrap();

        assert_eq!(reg.list_packages().count(), 1);
    }

    #[test]
    fn publish_writes_a_real_index_file_and_load_reads_it_back() {
        let dir = std::env::temp_dir().join(format!("pata-registry-test-{}", std::process::id()));
        let mut reg = LocalRegistry::at(&dir).expect("open registry at root");
        reg.publish(RegistryEntry {
            name: "jitu".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: "/tmp/jitu-1.0.0".to_string() },
        })
        .expect("publish");

        assert!(dir.join("jitu.json").is_file(), "publish must write a real index file");

        let loaded = LocalRegistry::load(&dir).expect("load");
        let meta = loaded.lookup("jitu").expect("jitu should be indexed");
        assert_eq!(meta.latest, "1.0.0");
        let entries = loaded.entries_for("jitu");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, RegistrySource::Path { path: "/tmp/jitu-1.0.0".to_string() });

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn publish_multiple_versions_and_yanked_are_excluded_from_metadata() {
        let dir = std::env::temp_dir().join(format!("pata-registry-test-yank-{}", std::process::id()));
        let mut reg = LocalRegistry::at(&dir).expect("open registry");
        reg.publish(RegistryEntry {
            name: "jitu".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: "/tmp/jitu-1.0.0".to_string() },
        }).unwrap();
        reg.publish(RegistryEntry {
            name: "jitu".to_string(),
            vers: "1.1.0".to_string(),
            deps: vec![],
            yanked: Some(true),
            source: RegistrySource::Path { path: "/tmp/jitu-1.1.0".to_string() },
        }).unwrap();
        reg.publish(RegistryEntry {
            name: "jitu".to_string(),
            vers: "1.2.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: "/tmp/jitu-1.2.0".to_string() },
        }).unwrap();

        let loaded = LocalRegistry::load(&dir).expect("load");
        let meta = loaded.lookup("jitu").expect("indexed");
        // 1.1.0 is yanked and must not appear as a resolvable/latest version.
        assert_eq!(meta.latest, "1.2.0");
        assert!(!meta.versions.contains(&"1.1.0".to_string()));
        assert!(meta.versions.contains(&"1.0.0".to_string()));
        assert!(meta.versions.contains(&"1.2.0".to_string()));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_from_nonexistent_root_returns_empty_registry_not_error() {
        let dir = std::env::temp_dir().join(format!("pata-registry-test-missing-{}", std::process::id()));
        let reg = LocalRegistry::load(&dir).expect("load on missing dir must not error");
        assert_eq!(reg.list_packages().count(), 0);
    }

    #[test]
    fn republishing_the_same_version_replaces_not_duplicates() {
        let dir = std::env::temp_dir().join(format!("pata-registry-test-repub-{}", std::process::id()));
        let mut reg = LocalRegistry::at(&dir).expect("open registry");
        reg.publish(RegistryEntry {
            name: "jitu".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: "/tmp/first".to_string() },
        }).unwrap();
        reg.publish(RegistryEntry {
            name: "jitu".to_string(),
            vers: "1.0.0".to_string(),
            deps: vec![],
            yanked: None,
            source: RegistrySource::Path { path: "/tmp/second".to_string() },
        }).unwrap();

        let entries = reg.entries_for("jitu");
        assert_eq!(entries.len(), 1, "re-publishing the same version must replace, not duplicate");
        assert_eq!(entries[0].source, RegistrySource::Path { path: "/tmp/second".to_string() });

        std::fs::remove_dir_all(&dir).ok();
    }
}
