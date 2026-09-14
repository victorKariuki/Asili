//! A resolved-shape dependency reference: a semver-ish version string (resolved against a
//! registry index), a local filesystem path, or a git source. Used by `resolve::find_module_file`
//! to locate a module's source — a deliberately smaller type than `pata_package::Dependency`
//! (which also carries a `branch` field and keeps `version`/`git`/`path` as independent optional
//! table fields rather than one enum, since a real `pata.toml`/`Asili.toml` table can combine
//! them in ways this simpler "where do I find the source" type doesn't need to distinguish).
//!
//! `Git` was added alongside `pata ongeza --git`'s real-fetch wiring: without it, a
//! `--git`-fetched dependency had no way to record in `pata.toml` that it came from a git URL
//! (only a plain version string), so `pata_package::Resolver::resolve` — once it started doing
//! real constraint solving instead of accepting anything — had no way to tell a fetched git
//! dependency apart from an unfetched registry one, and incorrectly treated every version
//! dependency as registry-sourced.

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Dependency {
    Version(String),
    Path(PathBuf),
    /// A dependency fetched via `pata ongeza --git <url>` — `version` is the semver constraint
    /// given at `ongeza` time (e.g. `^0.1`), not necessarily the exact fetched version (that's
    /// recorded in `pata.lock`, and in the vendored `.pata-version` marker `fetch_git`'s caller
    /// writes).
    Git { url: String, version: String },
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
            Dependency::Git { url, version } => write!(f, "{{ git = \"{}\", version = \"{}\" }}", url, version),
        }
    }
}
