//! Style lint rules

use asili_diagnostics::Diagnostic;
use asili_parser::Module;

/// Check for style issues
pub fn check_style_issues(module: &Module) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Check function length (warn if > 100 lines)
    for func in &module.functions {
        let func_lines = func.body.statements.len();
        if func_lines > 50 {
            diags.push(
                Diagnostic::new("LINT101", format!(
                    "Function '{}' has {} statements, consider breaking it up",
                    func.name, func_lines
                ))
                .with_stage("lint")
                .with_span(func.line, 1)
            );
        }
    }

    diags
}
