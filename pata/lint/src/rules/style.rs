//! Style lint rules

use asili_diagnostics::Diagnostic;
use asili_parser::Module;

/// Check for style issues
pub fn check_style_issues(module: &Module) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Check function length (warn if > 50 statements)
    for func in &module.functions {
        let func_lines = func.body.statements.len();
        if func_lines > 50 {
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
}
