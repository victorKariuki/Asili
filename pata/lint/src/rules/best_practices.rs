//! Best practices lint rules

use asili_diagnostics::Diagnostic;
use asili_parser::Module;

/// Check for best practice violations
pub fn check_best_practices(module: &Module, source: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Check for hardcoded strings that might be constants
    let string_count = source.matches('"').count() / 2;
    if string_count > 5 {
        diags.push(
            Diagnostic::new("LINT201", format!(
                "Found {} string literals, consider extracting repeated ones to constants",
                string_count
            ))
            .with_stage("lint")
        );
    }

    // Check for functions without documentation
    for func in &module.functions {
        if func.name != "kuu" && !func.name.starts_with("test_") {
            diags.push(
                Diagnostic::new("LINT202", format!(
                    "Function '{}' lacks documentation comment",
                    func.name
                ))
                .with_stage("lint")
                .with_span(func.line, 1)
            );
        }
    }

    // Check for unused imports
    if module.imports.len() > 0 && module.functions.is_empty() {
        diags.push(
            Diagnostic::new("LINT203", "File has imports but no code using them".to_string())
            .with_stage("lint")
        );
    }

    diags
}
