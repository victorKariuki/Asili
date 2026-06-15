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
                    "Function name '{}' should be snake_case or Swahili",
                    func.name
                ))
                .with_stage("lint")
                .with_span(func.line, 1)
            );
        }
    }

    // Check struct names are PascalCase
    for s in &module.structs {
        if !is_pascal_case(&s.name) {
            diags.push(
                Diagnostic::new("LINT002", format!(
                    "Struct name '{}' should be PascalCase",
                    s.name
                ))
                .with_stage("lint")
                .with_span(s.line, 1)
            );
        }
    }

    // Check constant names are UPPER_CASE
    for constant in &module.constants {
        if !is_upper_case(&constant.name) {
            diags.push(
                Diagnostic::new("LINT003", format!(
                    "Constant name '{}' should be UPPER_CASE",
                    constant.name
                ))
                .with_stage("lint")
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
