//! A resolved-shape dependency reference: either a semver-ish version string (resolved against
//! the vendored package cache) or a local filesystem path. Used by `resolve::find_module_file`
//! to locate a module's source — a deliberately smaller type than `pata_package::Dependency`
//! (which also carries git/branch manifest fields not relevant to "where is this module's
//! source on disk").

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Dependency {
    Version(String),
    Path(PathBuf),
}

impl From<&str> for Dependency {
    fn from(s: &str) -> Self {
        Dependency::Version(s.to_string())
    }
}

impl std::fmt::Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dependency::Version(v) => write!(f, "\"{}\"", v),
            Dependency::Path(p) => write!(f, "{{ path = \"{}\" }}", p.display()),
        }
    }
}
