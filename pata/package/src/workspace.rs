//! Workspace management for multi-package projects.

use crate::manifest::{Manifest, WorkspaceConfig};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A workspace containing one or more packages
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Root directory of the workspace
    pub root: PathBuf,
    /// Workspace configuration from Asili.toml
    pub config: WorkspaceConfig,
    /// All member packages (name -> manifest)
    pub members: BTreeMap<String, Manifest>,
    /// Root package (if this is also a package)
    pub root_package: Option<Manifest>,
}

impl Workspace {
    /// Load workspace from root directory
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let manifest_path = root.join("Asili.toml");

        let config = WorkspaceConfig::load(&manifest_path)
            .with_context(|| "imeshindwa kupakia workspace")?;

        // Validate members exist
        config.validate_members(&root)
            .with_context(|| "Mtaa haupo au hauna Asili.toml")?;

        // Load all members
        let members_vec = config.load_members(&root)
            .with_context(|| "imeshindwa kupakia wanachama wa workspace")?;

        let members: BTreeMap<String, Manifest> = members_vec.into_iter().collect();

        // Check if root is also a package
        let root_package = config.package.as_ref().map(|_| {
            Manifest {
                package: config.package.clone().unwrap(),
                dependencies: config.dependencies.clone(),
                dev_dependencies: config.dev_dependencies.clone(),
            }
        });

        Ok(Self {
            root,
            config,
            members,
            root_package,
        })
    }

    /// Check if this is a workspace (has [workspace] section)
    pub fn is_workspace(&self) -> bool {
        self.config.is_workspace()
    }

    /// Get member by name
    pub fn get_member(&self, name: &str) -> Option<&Manifest> {
        self.members.get(name)
    }

    /// Get all member names
    pub fn member_names(&self) -> impl Iterator<Item = &String> {
        self.members.keys()
    }

    /// Get all members
    pub fn all_members(&self) -> impl Iterator<Item = (&String, &Manifest)> {
        self.members.iter()
    }

    /// Get total number of members
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get member path relative to root
    pub fn member_path(&self, member_name: &str) -> Option<PathBuf> {
        self.config.members().and_then(|members| {
            members.iter().find(|m| m.contains(member_name))
                .map(|m| self.root.join(m))
        })
    }

    /// Collect all dependencies from root and members
    pub fn all_dependencies(&self) -> BTreeMap<String, String> {
        let mut deps = BTreeMap::new();

        // Add root package dependencies
        if let Some(root) = &self.root_package {
            for (name, dep) in root.all_dependencies() {
                deps.insert(name.clone(), dep.version().to_string());
            }
        }

        // Add member dependencies (overwrite if duplicate)
        for (_, manifest) in &self.members {
            for (name, dep) in manifest.all_dependencies() {
                deps.insert(name.clone(), dep.version().to_string());
            }
        }

        deps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_open() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create workspace structure
        fs::write(
            root.join("Asili.toml"),
            r#"
[workspace]
members = ["lib/math-utils"]

[package]
name = "my-app"
version = "1.0.0"
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("lib/math-utils")).unwrap();
        fs::write(
            root.join("lib/math-utils/Asili.toml"),
            r#"
[package]
name = "math-utils"
version = "1.0.0"
"#,
        )
        .unwrap();

        let ws = Workspace::open(root).unwrap();
        assert!(ws.is_workspace());
        assert_eq!(ws.member_count(), 1);
        // Members are keyed by their path, not package name
        assert!(ws.get_member("lib/math-utils").is_some());
        assert!(ws.root_package.is_some());
    }

    #[test]
    fn test_member_not_found() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::write(
            root.join("Asili.toml"),
            r#"
[workspace]
members = ["lib/nonexistent"]
"#,
        )
        .unwrap();

        let result = Workspace::open(root);
        assert!(result.is_err());
    }
}
