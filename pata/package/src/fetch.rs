//! Real dependency fetching: git-clone a source into the vendored package cache.
//! No registry backend exists yet (see docs/design/pata-production-readiness.md) — this
//! covers git and path sources only.

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("git clone failed for {url}: {source}")]
    GitClone {
        url: String,
        #[source]
        source: git2::Error,
    },
    #[error("failed to hash fetched content at {path}: {source}")]
    Hash {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Clone `url` (optionally at `branch`, else the repo's default branch) into `dest`,
/// replacing any existing directory at `dest` first (idempotent re-fetch). Returns a
/// deterministic SHA-256 content hash over the cloned tree, computed by `hash_dir`.
pub fn fetch_git(url: &str, branch: Option<&str>, dest: &Path) -> Result<String, FetchError> {
    if dest.exists() {
        fs::remove_dir_all(dest).map_err(|e| FetchError::Hash {
            path: dest.display().to_string(),
            source: e,
        })?;
    }
    let mut builder = git2::build::RepoBuilder::new();
    if let Some(b) = branch {
        builder.branch(b);
    }
    builder
        .clone(url, dest)
        .map_err(|e| FetchError::GitClone { url: url.to_string(), source: e })?;
    // Remove .git metadata from the vendored copy
    let git_dir = dest.join(".git");
    if git_dir.exists() {
        let _ = fs::remove_dir_all(&git_dir);
    }
    hash_dir(dest).map_err(|e| FetchError::Hash { path: dest.display().to_string(), source: e })
}

/// Deterministic content hash over every regular file under `dir`, recursively.
pub fn hash_dir(dir: &Path) -> std::io::Result<String> {
    let mut files = Vec::new();
    collect_files(dir, dir, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for rel in &files {
        hasher.update(rel.as_bytes());
        hasher.update(b"\0");
        let bytes = fs::read(dir.join(rel))?;
        hasher.update(&bytes);
    }
    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect::<String>())
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else {
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.push(rel);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn hash_dir_is_deterministic() {
        let dir = std::env::temp_dir().join(format!(
            "pata-fetch-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::File::create(dir.join("a.txt")).unwrap().write_all(b"hello").unwrap();
        fs::File::create(dir.join("sub/b.txt")).unwrap().write_all(b"world").unwrap();

        let h1 = hash_dir(&dir).unwrap();
        let h2 = hash_dir(&dir).unwrap();
        assert_eq!(h1, h2);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hash_dir_changes_when_content_changes() {
        let dir = std::env::temp_dir().join(format!(
            "pata-fetch-test2-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::File::create(dir.join("a.txt")).unwrap().write_all(b"hello").unwrap();
        let h1 = hash_dir(&dir).unwrap();
        fs::File::create(dir.join("a.txt")).unwrap().write_all(b"goodbye").unwrap();
        let h2 = hash_dir(&dir).unwrap();
        assert_ne!(h1, h2);

        fs::remove_dir_all(&dir).unwrap();
    }
}
