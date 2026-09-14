//! Style lint rules

use asili_diagnostics::Diagnostic;
use asili_parser::Module;

/// Default LINT101 threshold: functions with more statements than this get flagged. Overridable
/// via `[lint.rules.LINT101] options.line_limit` in `pata.toml` — see `check_style_issues_with_limit`.
pub const DEFAULT_LINE_LIMIT: usize = 50;

/// Check for style issues, using the default statement-count threshold for LINT101.
pub fn check_style_issues(module: &Module) -> Vec<Diagnostic> {
    check_style_issues_with_limit(module, DEFAULT_LINE_LIMIT)
}

/// Check for style issues with a caller-supplied LINT101 threshold (from `LintConfig::option_int
/// ("LINT101", "line_limit")`, when set).
pub fn check_style_issues_with_limit(module: &Module, line_limit: usize) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    for func in &module.functions {
        let func_lines = func.body.statements.len();
        if func_lines > line_limit {
            diags.push(
                Diagnostic::new("LINT101", format!(
                    "kazi '{}' ina kauli {} — fikiria kuigawanya",
                    func.name, func_lines
                ))
                .with_stage("ukaguzi")
                .with_span(func.line, 1)
            );
        }
    }

    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use asili_lexer::tokenize;
    use asili_parser::parse_tokens;

    fn lint(src: &str) -> Vec<Diagnostic> {
        let tokens = tokenize(src).expect("tokenize");
        let module = parse_tokens(&tokens).expect("parse");
        check_style_issues(&module)
    }

    #[test]
    fn lint101_flags_long_function() {
        let mut src = String::from("kazi ndefu() -> Tupu { ");
        for _ in 0..51 {
            src.push_str("weka x = 1\n");
        }
        src.push_str("rejesha Tupu }");
        let diags = lint(&src);
        assert!(diags.iter().any(|d| d.code == "LINT101"));
    }

    #[test]
    fn lint101_accepts_short_function() {
        let src = "kazi fupi() -> Tupu { weka x = 1\nrejesha Tupu }";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT101"));
    }

    #[test]
    fn lint101_accepts_boundary_function() {
        let mut src = String::from("kazi boundary() -> Tupu { ");
        for _ in 0..49 {
            src.push_str("weka x = 1\n");
        }
        src.push_str("rejesha Tupu }");
        let diags = lint(&src);
        assert!(!diags.iter().any(|d| d.code == "LINT101"));
    }
}
