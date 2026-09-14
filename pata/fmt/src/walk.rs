use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Collect all .as and .asi files recursively from a directory.
pub fn collect_asili_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("imeshindwa kusoma {}", dir.display()))? {
        let entry = entry.with_context(|| "hitilafu ya kusoma entry")?;
        let path = entry.path();

        // Skip target directories
        if path.file_name().map(|n| n == "kilele").unwrap_or(false) {
            continue;
        }

        // Skip hidden directories
        if path.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with('.')).unwrap_or(false) {
            continue;
        }

        if path.is_dir() {
            walk(&path, out)?;
        } else if let Some(ext) = path.extension() {
            if ext == "as" || ext == "asi" {
                out.push(path);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_collect_asili_files() -> Result<()> {
        let tmpdir = TempDir::new()?;
        let path = tmpdir.path();

        // Create test files
        let mut file = fs::File::create(path.join("test.as"))?;
        file.write_all(b"kazi foo() -> Tupu {}")?;

        let mut file = fs::File::create(path.join("test.asi"))?;
        file.write_all(b"kazi bar() -> Tupu {}")?;

        let files = collect_asili_files(path)?;
        assert_eq!(files.len(), 2);
        Ok(())
    }
}
