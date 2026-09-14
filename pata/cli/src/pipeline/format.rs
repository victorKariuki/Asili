//! `pata jenga`/`pata nadhifu`/`pata thibitisha`'s file-collection and format-check/write logic.
//! Formatting itself delegates to `pata-fmt`'s real token-stream printer
//! (`pata_fmt::canonical_format_with_indent`) — this module previously carried its own inlined
//! copy of a much weaker line-based text transform (blind string-replace on `{`/`}`/`,`,
//! including inside string literals — see this file's own former `TODO`/`HACK` comments, and
//! GitHub issue #18) that `pata nadhifu` and `pata thibitisha`'s format-compliance gate were
//! both actively running against real user source. `pata-fmt` gained a `[lib]` target this
//! session specifically so this crate (and `pata-lsp`) could depend on the one real
//! implementation instead of maintaining separate, drifting copies.

use crate::commands::CliError;
use pata_fmt::config::FormatterConfig;
use std::fs;
use std::path::{Path, PathBuf};

pub fn collect_asili_files(root: &Path) -> Result<Vec<PathBuf>, CliError> {
    let mut out = Vec::new();
    walk(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), CliError> {
    for entry in fs::read_dir(dir)
        .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", dir.display()), 1))?
    {
        let entry = entry.map_err(|e| CliError::new(format!("hitilafu ya kusoma kiingilio: {e}"), 1))?;
        let path = entry.path();
        if path.file_name().map(|n| n == "kilele").unwrap_or(false) {
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

/// Format `input` using the real canonical formatter, honoring `[fmt]` indent settings from the
/// nearest ancestor `pata.toml` above `file_path` (when given).
pub fn canonical_format(input: &str, file_path: Option<&Path>) -> String {
    let config = file_path
        .and_then(|p| FormatterConfig::find_and_load(p).ok())
        .unwrap_or_default();
    pata_fmt::canonical_format_with_indent(input, &config.indent_unit())
}

pub fn check_or_write(files: &[PathBuf], check_only: bool) -> Result<(usize, usize), CliError> {
    let mut changed = 0usize;
    for file in files {
        let original = fs::read_to_string(file)
            .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", file.display()), 1))?;
        let formatted = canonical_format(&original, Some(file));
        if formatted != original {
            changed += 1;
            if !check_only {
                fs::write(file, formatted).map_err(|e| {
                    CliError::new(format!("imeshindwa kuandika {}: {e}", file.display()), 1)
                })?;
            }
        }
    }
    Ok((files.len(), changed))
}

#[cfg(test)]
mod tests {
    use super::canonical_format;

    #[test]
    fn format_is_idempotent() {
        let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    chapisha(\"x\")\n}\n";
        let a = canonical_format(src, None);
        let b = canonical_format(&a, None);
        assert_eq!(a, b);
    }

    /// The whole point of the pata-fmt delegation: a string literal containing `{`/`}`/`,`
    /// must survive formatting byte-for-byte, not get corrupted by blind brace/comma spacing —
    /// the exact GitHub issue #18 bug this rewrite fixes.
    #[test]
    fn format_preserves_string_literal_contents() {
        let src = r#"chapisha("a, b {c}")"#;
        let formatted = canonical_format(src, None);
        assert!(
            formatted.contains(r#""a, b {c}""#),
            "string literal contents must survive formatting verbatim, got: {formatted}"
        );
    }
}
