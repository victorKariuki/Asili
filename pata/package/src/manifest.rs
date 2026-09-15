//! Dependency specification, shared by every manifest source `pata-cli` reads (a project's own
//! `pata.toml` `[tegemezi]` table, and each transitively-resolved `RegistryEntry`'s own declared
//! deps — see `resolver.rs`). Previously also held `Manifest`/`WorkspaceConfig`/`PackageMetadata`,
//! an entire second, English-keyed manifest schema for a separate `Asili.toml` workspace file —
//! removed once `pata.toml` itself gained a `[eneo-kazi]` table (real TOML parsing, in
//! `pata-cli`'s own `pipeline::project::load_project_config`) as the one unified way to declare
//! workspace membership, retiring `Asili.toml` entirely. See
//! `docs/design/package-manager-design.md` for the design history.

use serde::{Deserialize, Serialize};

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    /// Full dependency spec (must come first to try table first)
    Table(DependencyTable),
    /// Version requirement (e.g., "1.2.3" or "^1.2.0")
    Version(String),
}

/// Full dependency specification table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyTable {
    pub version: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub git: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

impl Dependency {
    /// Get version requirement
    pub fn version(&self) -> &str {
        match self {
            Dependency::Version(v) => v,
            Dependency::Table(t) => &t.version,
        }
    }

    /// Get path if this is a path dependency
    pub fn path(&self) -> Option<&str> {
        match self {
            Dependency::Version(_) => None,
            Dependency::Table(t) => t.path.as_deref(),
        }
    }

    /// Get git URL if this is a git dependency
    pub fn git(&self) -> Option<&str> {
        match self {
            Dependency::Version(_) => None,
            Dependency::Table(t) => t.git.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_accessors() {
        let dep1 = Dependency::Version("1.2.3".to_string());
        assert_eq!(dep1.version(), "1.2.3");
        assert_eq!(dep1.path(), None);

        let dep2 = Dependency::Table(DependencyTable {
            version: "0.1.0".to_string(),
            path: Some("./lib".to_string()),
            git: None,
            branch: None,
        });
        assert_eq!(dep2.version(), "0.1.0");
        assert_eq!(dep2.path(), Some("./lib"));
    }
}
