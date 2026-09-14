//! Asili linter for code style and best practices

pub mod rules;
pub mod visitor;

use asili_diagnostics::Diagnostic;
use asili_parser::parse_tokens;
use asili_lexer::tokenize;

/// Lint a source file and return warnings/suggestions
/// Returns combined linting issues (parse errors + lint warnings)
pub fn lint_source(source: &str) -> Result<Vec<Diagnostic>, String> {
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(diags) => return Err(format!("leksika imeshindwa: makosa {}", diags.len())),
    };

    let module = match parse_tokens(&tokens) {
        Ok(m) => m,
        Err(diags) => return Err(format!("uchanganuzi umeshindwa: makosa {}", diags.len())),
    };

    let mut lints = Vec::new();

    // Run all lint rules
    lints.extend(rules::naming::check_naming_conventions(&module));
    lints.extend(rules::style::check_style_issues(&module));
    lints.extend(rules::best_practices::check_best_practices(&module, source));
    lints.extend(rules::logic::check_logic_errors(&module));

    Ok(lints)
}
