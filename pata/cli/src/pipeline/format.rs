use crate::commands::CliError;
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

// TODO: canonical_format is a line-level text transform — it does NOT parse the AST.
// Known limitations:
//   - Indentation is stripped entirely (all lines are left-aligned after formatting)
//   - Brace/comma spacing is inserted blindly, including inside string literals
//   - No operator spacing (a+b stays a+b, not a + b)
//   - No alignment of struct fields or match arms
// To fix: format by re-printing the parsed AST with a pretty-printer visitor, not regex on raw text.
pub fn canonical_format(input: &str) -> String {
    let mut out = String::new();
    let mut last_blank = false;
    for raw in input.lines() {
        let trimmed_end = raw.trim_end();
        let is_blank = trimmed_end.trim().is_empty();
        if is_blank {
            if !last_blank {
                out.push('\n');
            }
            last_blank = true;
            continue;
        }

        last_blank = false;
        // HACK: brace and comma spacing via string replace can corrupt string literals
        // containing { } or , characters. Must be replaced with a token-aware formatter.
        let mut line = trimmed_end.replace("{", " { ");
        line = line.replace("}", " } ");
        line = line.replace(",", ", ");
        while line.contains("  ") {
            line = line.replace("  ", " ");
        }
        out.push_str(line.trim());
        out.push('\n');
    }

    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn check_or_write(files: &[PathBuf], check_only: bool) -> Result<(usize, usize), CliError> {
    let mut changed = 0usize;
    for file in files {
        let original = fs::read_to_string(file)
            .map_err(|e| CliError::new(format!("imeshindwa kusoma {}: {e}", file.display()), 1))?;
        let formatted = canonical_format(&original);
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
        let src = "kazi kuu(hoja: Orodha<Neno>) -> Tupu{\\n    chapisha(\\\"x\\\")\\n}\\n";
        let a = canonical_format(src);
        let b = canonical_format(&a);
        assert_eq!(a, b);
    }
}
