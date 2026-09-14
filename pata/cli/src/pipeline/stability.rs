//! Type-stability checking against git-tag baselines

use asili_parser::Module;
use crate::commands::CliError;
use std::process::Command;

/// Check type stability against a git tag (e.g., `v1.0.0`)
pub fn check_type_stability(baseline_tag: &str, current: &Module) -> Result<(), CliError> {
    // Attempt to fetch the baseline from git
    let baseline = get_baseline_module(baseline_tag)?;

    // Compare public signatures
    let mut breaking_changes = Vec::new();

    for func in &current.functions {
        // Skip private/test functions
        if func.name.starts_with("test_") {
            continue;
        }

        let baseline_func = baseline.functions.iter().find(|f| f.name == func.name);
        match baseline_func {
            None => {
                // New public function: not breaking
            }
            Some(old_func) => {
                // Check parameter count (arity change)
                if func.params.len() != old_func.params.len() {
                    breaking_changes.push(format!(
                        "kazi '{}': muundo wa hoja umegeuka ({} → {})",
                        func.name, old_func.params.len(), func.params.len()
                    ));
                }
                // Check return type consistency would require full type comparison
                // Placeholder: arity check is the quick win
            }
        }
    }

    // Check for removed public functions
    for old_func in &baseline.functions {
        if old_func.name.starts_with("test_") {
            continue;
        }
        if !current.functions.iter().any(|f| f.name == old_func.name) {
            breaking_changes.push(format!("kazi '{}' iliondolewa", old_func.name));
        }
    }

    if !breaking_changes.is_empty() {
        let msg = format!(
            "mabadiliko ya kukatika aiki yanayolingana:\n{}",
            breaking_changes.join("\n")
        );
        return Err(CliError::new(msg, 1));
    }

    Ok(())
}

/// Fetch the source code from a git tag and parse it
fn get_baseline_module(tag: &str) -> Result<Module, CliError> {
    // Use git show to fetch the main source file from the tag
    let output = Command::new("git")
        .arg("show")
        .arg(format!("{}:src/kuu.as", tag))
        .output()
        .map_err(|e| {
            CliError::new(
                format!("imeshindwa kupata baseline kutoka kwa tag '{}': {}", tag, e),
                1,
            )
        })?;

    if !output.status.success() {
        return Err(CliError::new(
            format!("tag '{}' haipo au src/kuu.as haipo ndani yake", tag),
            1,
        ));
    }

    let source = String::from_utf8_lossy(&output.stdout).to_string();
    let tokens = asili_lexer::tokenize(&source)
        .map_err(|_| CliError::new("imeshindwa kupiga tokenize baseline", 1))?;
    let module = asili_parser::parse_tokens(&tokens)
        .map_err(|_| CliError::new("imeshindwa kusimbua baseline", 1))?;

    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stability_detects_arity_change() {
        // Placeholder: real test would need Module fixtures
        // Verify that parameter count mismatches are detected
    }
}
