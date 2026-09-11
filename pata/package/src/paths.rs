//! Path management for workspace and package directories.

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Directory structure management
#[derive(Debug, Clone)]
pub struct Paths {
    /// Root of the workspace
    pub root: PathBuf,
    /// .asili/ directory for package manager state
    pub asili_dir: PathBuf,
    /// .asili/packages/ for external packages
    pub packages_dir: PathBuf,
    /// .asili/kilele/ for build cache
    pub build_cache_dir: PathBuf,
    /// kilele/ for final build artifacts
    pub target_dir: PathBuf,
    /// lib/ for local packages
    pub lib_dir: PathBuf,
    /// src/ for main source
    pub src_dir: PathBuf,
}

impl Paths {
    /// Create path manager for a workspace root
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let asili_dir = root.join(".asili");
        let packages_dir = asili_dir.join("packages");
        let build_cache_dir = asili_dir.join("kilele");
        let target_dir = root.join("kilele");
        let lib_dir = root.join("lib");
        let src_dir = root.join("src");

        Self {
            root,
            asili_dir,
            packages_dir,
            build_cache_dir,
            target_dir,
            lib_dir,
            src_dir,
        }
    }

    /// Initialize all required directories
    pub fn init(&self) -> Result<()> {
        // Create .asili/ structure
        std::fs::create_dir_all(&self.asili_dir)?;
        std::fs::create_dir_all(&self.packages_dir)?;
        std::fs::create_dir_all(&self.build_cache_dir)?;

        // Create target/
        std::fs::create_dir_all(&self.target_dir)?;

        // Create lib/ if it doesn't exist (optional)
        if !self.lib_dir.exists() {
            std::fs::create_dir_all(&self.lib_dir)?;
        }

        // Create src/ if it doesn't exist (optional)
        if !self.src_dir.exists() {
            std::fs::create_dir_all(&self.src_dir)?;
        }

        Ok(())
    }

    /// Get path to an external package
    pub fn package_path(&self, name: &str) -> PathBuf {
        self.packages_dir.join(name)
    }

    /// Get path to an external package's source
    pub fn package_src_path(&self, name: &str) -> PathBuf {
        self.package_path(name).join("src")
    }

    /// Get path to build artifact
    pub fn artifact_path(&self, name: &str, release: bool) -> PathBuf {
        let mode = if release { "release" } else { "debug" };
        self.target_dir.join(mode).join(format!("{}.asb", name))
    }

    /// Get path to build cache for a package
    pub fn build_cache_path(&self, package_name: &str) -> PathBuf {
        self.build_cache_dir.join(package_name)
    }

    /// Create .gitignore content for workspace
    pub fn gitignore_content() -> &'static str {
        r#"# Kilele za ujenzi
/kilele/
/.asili/packages/
/.asili/kilele/

# Faili ya lock (hiari: fuatilia kwa ujenzi unaoweza kurudiwa)
# Asili.lock

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db
"#
    }

    /// Create default .gitignore
    pub fn create_gitignore(&self) -> Result<()> {
        let gitignore_path = self.root.join(".gitignore");
        if !gitignore_path.exists() {
            std::fs::write(&gitignore_path, Self::gitignore_content())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_paths_creation() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::new(temp.path());

        assert_eq!(paths.root, temp.path());
        assert_eq!(paths.asili_dir, temp.path().join(".asili"));
        assert_eq!(paths.packages_dir, temp.path().join(".asili/packages"));
        assert_eq!(paths.target_dir, temp.path().join("kilele"));
    }

    #[test]
    fn test_paths_init() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::new(temp.path());

        paths.init().unwrap();

        assert!(paths.asili_dir.exists());
        assert!(paths.packages_dir.exists());
        assert!(paths.build_cache_dir.exists());
        assert!(paths.target_dir.exists());
    }

    #[test]
    fn test_package_paths() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::new(temp.path());

        let pkg_path = paths.package_path("math-utils");
        assert!(pkg_path.to_string_lossy().contains("math-utils"));

        let src_path = paths.package_src_path("math-utils");
        assert!(src_path.to_string_lossy().contains("src"));
    }

    #[test]
    fn test_artifact_paths() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::new(temp.path());

        let debug_artifact = paths.artifact_path("app", false);
        assert!(debug_artifact.to_string_lossy().contains("debug"));
        assert!(debug_artifact.to_string_lossy().contains("app.asb"));

        let release_artifact = paths.artifact_path("app", true);
        assert!(release_artifact.to_string_lossy().contains("release"));
    }

    #[test]
    fn test_gitignore_creation() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::new(temp.path());

        paths.create_gitignore().unwrap();

        let gitignore = temp.path().join(".gitignore");
        assert!(gitignore.exists());

        let content = std::fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains("kilele/"));
        assert!(content.contains(".asili/packages/"));
    }
}
