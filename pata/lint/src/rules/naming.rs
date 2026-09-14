//! Naming convention lint rules

use asili_diagnostics::Diagnostic;
use asili_parser::Module;

/// Check for naming convention violations
pub fn check_naming_conventions(module: &Module) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Check function names are snake_case
    for func in &module.functions {
        if !is_snake_case(&func.name) && !is_swahili(&func.name) {
            diags.push(
                Diagnostic::new("LINT001", format!(
                    "jina la kazi '{}' linapaswa kuwa snake_case au Kiswahili",
                    func.name
                ))
                .with_stage("ukaguzi")
                .with_span(func.line, 1)
            );
        }
    }

    // Check struct names are PascalCase
    for s in &module.structs {
        if !is_pascal_case(&s.name) {
            diags.push(
                Diagnostic::new("LINT002", format!(
                    "jina la umbo '{}' linapaswa kuwa PascalCase",
                    s.name
                ))
                .with_stage("ukaguzi")
                .with_span(s.line, 1)
            );
        }
    }

    // Check constant names are UPPER_CASE
    for constant in &module.constants {
        if !is_upper_case(&constant.name) {
            diags.push(
                Diagnostic::new("LINT003", format!(
                    "jina la thabiti '{}' linapaswa kuwa UPPER_CASE",
                    constant.name
                ))
                .with_stage("ukaguzi")
                .with_span(constant.line, 1)
            );
        }
    }

    diags
}

fn is_snake_case(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_lowercase() || c == '_' || c.is_numeric())
}

fn is_pascal_case(s: &str) -> bool {
    !s.is_empty() && s.chars().next().unwrap().is_uppercase()
        && s.chars().skip(1).all(|c| c.is_alphanumeric() || c == '_')
}

fn is_upper_case(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_uppercase() || c == '_' || c.is_numeric())
}

fn is_swahili(s: &str) -> bool {
    s.len() > 0 && !s.chars().all(|c| c.is_ascii())
}

#[cfg(test)]
mod tests {
    use super::*;
    use asili_lexer::tokenize;
    use asili_parser::parse_tokens;

    fn lint(src: &str) -> Vec<Diagnostic> {
        let tokens = tokenize(src).expect("tokenize");
        let module = parse_tokens(&tokens).expect("parse");
        check_naming_conventions(&module)
    }

    #[test]
    fn lint001_flags_bad_function_name() {
        let diags = lint("kazi MyFunction() -> Tupu { rejesha Tupu }");
        assert!(diags.iter().any(|d| d.code == "LINT001"));
    }

    #[test]
    fn lint001_accepts_good_function_name() {
        let diags = lint("kazi jina_sahihi() -> Tupu { rejesha Tupu }");
        assert!(!diags.iter().any(|d| d.code == "LINT001"));
    }

    #[test]
    fn lint002_flags_bad_struct_name() {
        let diags = lint("umbo not_pascal { }");
        assert!(diags.iter().any(|d| d.code == "LINT002"));
    }

    #[test]
    fn lint002_accepts_good_struct_name() {
        let diags = lint("umbo GoodStruct { }");
        assert!(!diags.iter().any(|d| d.code == "LINT002"));
    }

    #[test]
    fn lint003_flags_bad_constant_name() {
        let diags = lint("thabiti notUpper: Namba = 5");
        assert!(diags.iter().any(|d| d.code == "LINT003"));
    }

    #[test]
    fn lint003_accepts_good_constant_name() {
        let diags = lint("thabiti MAX_SIZE: Namba = 100");
        assert!(!diags.iter().any(|d| d.code == "LINT003"));
    }
}
