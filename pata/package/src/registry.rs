//! Package registry interface (minimal for Asili-compatible registries)

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Package metadata from a registry index
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

/// Registry index entry (compatible with Cargo/crates.io format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Package name (lowercase)
    pub name: String,
    /// Version number
    pub vers: String,
    /// Dependencies with version constraints
    pub deps: Vec<RegistryDep>,
    /// Package yanked/deprecated?
    pub yanked: Option<bool>,
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

/// Simple in-memory registry for testing/local use
pub struct LocalRegistry {
    packages: BTreeMap<String, PackageMetadata>,
}

impl LocalRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            packages: BTreeMap::new(),
        }
    }

    /// Register a package
    pub fn register(&mut self, metadata: PackageMetadata) -> anyhow::Result<()> {
        self.packages.insert(metadata.name.clone(), metadata);
        Ok(())
    }

    /// Look up a package
    pub fn lookup(&self, name: &str) -> Option<&PackageMetadata> {
        self.packages.get(name)
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
}
