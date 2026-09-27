//! The one implementation of "which project is this file in, and what does its `pata.toml`
//! say about my tool" — shared by `pata-fmt`, `pata-lint` and `pata-lsp`. Kept dependency-light
//! (no package manager, no network) so every tool can use it.

use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "pata.toml";

/// The nearest directory at or above `start` (a file or directory) holding a `pata.toml`.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let start_dir = if start.is_dir() {
        start
    } else {
        start.parent()?
    };
    start_dir
        .ancestors()
        .find(|dir| dir.join(MANIFEST).is_file())
        .map(Path::to_path_buf)
}

/// Deserialize the table at `path` (e.g. `["lint", "rules"]`) of `project_root/pata.toml`.
/// `Ok(None)` when there is no manifest or no such table; an error only for an unreadable or
/// malformed manifest.
pub fn load_section<T: DeserializeOwned>(
    project_root: &Path,
    path: &[&str],
) -> anyhow::Result<Option<T>> {
    let manifest = project_root.join(MANIFEST);
    if !manifest.exists() {
        return Ok(None);
    }
    let parsed: toml::Table = toml::from_str(&std::fs::read_to_string(&manifest)?)?;
    let mut table = &parsed;
    for key in path {
        match table.get(*key).and_then(|v| v.as_table()) {
            Some(t) => table = t,
            None => return Ok(None),
        }
    }
    Ok(Some(table.clone().try_into()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn finds_root_from_a_nested_file_and_reads_a_nested_section() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(MANIFEST), "[lint.rules]\nLINT001 = 1\n").unwrap();
        let nested = root.path().join("src/deep");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("a.as");
        std::fs::write(&file, "").unwrap();

        let found = find_project_root(&file).unwrap();
        assert_eq!(found, root.path());
        let rules: BTreeMap<String, i64> =
            load_section(&found, &["lint", "rules"]).unwrap().unwrap();
        assert_eq!(rules["LINT001"], 1);
        assert!(load_section::<BTreeMap<String, i64>>(&found, &["fmt"])
            .unwrap()
            .is_none());
    }
}
