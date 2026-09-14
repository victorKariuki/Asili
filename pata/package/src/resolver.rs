//! Dependency resolution algorithm

use crate::{Dependency, LockFile, LockedDependency};
use anyhow::Result;
use std::collections::BTreeMap;

/// Dependency resolver
pub struct Resolver;

impl Resolver {
    /// Resolve dependencies into a lock file
    /// Currently implements a simple algorithm that treats each dependency as resolved
    pub fn resolve(
        dependencies: &BTreeMap<String, Dependency>,
        _existing_lock: Option<&LockFile>,
    ) -> Result<LockFile> {
        let mut lock = LockFile::new();

        for (name, dep) in dependencies {
            let locked = LockedDependency {
                version: dep.version().to_string(),
                checksum: compute_checksum(name, dep.version()),
                path: dep.path().map(|s| s.to_string()),
                source: dep.git().map(|_| "git").unwrap_or("registry").to_string(),
            };
            lock.lock_dependency(name.clone(), locked);
        }

        Ok(lock)
    }
}

/// Compute a simple checksum for a dependency
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

    #[test]
    fn test_resolve_empty() -> Result<()> {
        let deps = BTreeMap::new();
        let lock = Resolver::resolve(&deps, None)?;
        assert!(lock.dependencies.is_empty());
        Ok(())
    }

    #[test]
    fn test_resolve_simple() -> Result<()> {
        let mut deps = BTreeMap::new();
        deps.insert("stdlib".to_string(), Dependency::Version("1.0.0".to_string()));

        let lock = Resolver::resolve(&deps, None)?;
        assert_eq!(lock.dependencies.len(), 1);
        assert!(lock.is_locked("stdlib"));
        Ok(())
    }

    #[test]
    fn test_checksum_deterministic() {
        let cs1 = compute_checksum("mylib", "1.0.0");
        let cs2 = compute_checksum("mylib", "1.0.0");
        assert_eq!(cs1, cs2);
    }
}
