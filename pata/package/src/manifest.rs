//! Manifest parsing for Asili.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Workspace configuration (top-level Asili.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub workspace: Option<WorkspaceMetadata>,
    #[serde(default)]
    pub package: Option<PackageMetadata>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, Dependency>,
}

/// Workspace metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    #[serde(default)]
    pub members: Vec<String>,
}

/// Package manifest metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: PackageMetadata,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, Dependency>,
}

/// Package metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
}

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

impl WorkspaceConfig {
    /// Load workspace config from Asili.toml
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| "imeshindwa kusoma Asili.toml")?;
        Self::from_str(&content)
    }

    /// Parse workspace config from TOML string
    pub fn from_str(content: &str) -> Result<Self> {
        toml::from_str(content).with_context(|| "hitilafu ya kuchambua Asili.toml")
    }

    /// Write workspace config to Asili.toml
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "hitilafu ya kubadili manifest kuwa TOML")?;
        fs::write(&path, content)
            .with_context(|| "imeshindwa kuandika Asili.toml")
    }

    /// Check if this is a workspace root (has [workspace] section)
    pub fn is_workspace(&self) -> bool {
        self.workspace.is_some()
    }

    /// Get workspace members if this is a workspace
    pub fn members(&self) -> Option<&[String]> {
        self.workspace.as_ref().map(|w| w.members.as_slice())
    }

    /// Validate workspace members paths exist
    pub fn validate_members(&self, root: &Path) -> Result<()> {
        if let Some(members) = self.members() {
            for member in members {
                let member_path = root.join(member);
                if !member_path.exists() {
                    anyhow::bail!("Mtaa {} haupo: {}", member, member_path.display());
                }
                let manifest_path = member_path.join("Asili.toml");
                if !manifest_path.exists() {
                    anyhow::bail!("Manifest haipo kwa {}: {}", member, manifest_path.display());
                }
            }
        }
        Ok(())
    }

    /// Load all member manifests
    pub fn load_members(&self, root: &Path) -> Result<Vec<(String, Manifest)>> {
        let mut members = Vec::new();
        if let Some(member_paths) = self.members() {
            for member_path_str in member_paths {
                let member_path = root.join(member_path_str);
                let manifest_path = member_path.join("Asili.toml");
                let manifest = Manifest::load(&manifest_path)?;
                members.push((member_path_str.clone(), manifest));
            }
        }
        Ok(members)
    }
}

impl Manifest {
    /// Load manifest from pata.toml
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("imeshindwa kusoma pata.toml"))?;
        Self::from_str(&content)
    }

    /// Parse manifest from TOML string
    pub fn from_str(content: &str) -> Result<Self> {
        toml::from_str(content).with_context(|| "hitilafu ya kuchambua pata.toml")
    }

    /// Write manifest to pata.toml
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "hitilafu ya kubadili manifest kuwa TOML")?;
        fs::write(&path, content)
            .with_context(|| format!("imeshindwa kuandika pata.toml"))
    }

    /// Get all dependencies including dev dependencies
    pub fn all_dependencies(&self) -> impl Iterator<Item = (&String, &Dependency)> {
        self.dependencies.iter().chain(self.dev_dependencies.iter())
    }
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
    fn test_parse_manifest() {
        let content = r#"
[package]
name = "myapp"
version = "0.1.0"
description = "My app"

[dependencies]
stdlib = "1.0.0"
other = "2.0.0"
"#;
        let manifest = Manifest::from_str(content).unwrap();
        assert_eq!(manifest.package.name, "myapp");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.dependencies.len(), 2);
    }

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

    #[test]
    fn test_parse_workspace_config() {
        let content = r#"
[workspace]
members = ["lib/math-utils", "lib/string-lib"]

[package]
name = "my-app"
version = "1.0.0"

[dependencies]
uuid = "1.0"
"#;
        let config = WorkspaceConfig::from_str(content).unwrap();
        assert!(config.is_workspace());
        assert_eq!(config.members(), Some(&["lib/math-utils".to_string(), "lib/string-lib".to_string()][..]));
        assert_eq!(config.package.as_ref().unwrap().name, "my-app");
        assert_eq!(config.dependencies.len(), 1);
    }

    #[test]
    fn test_non_workspace_config() {
        let content = r#"
[package]
name = "single-package"
version = "1.0.0"
"#;
        let config = WorkspaceConfig::from_str(content).unwrap();
        assert!(!config.is_workspace());
        assert_eq!(config.members(), None);
    }
}
