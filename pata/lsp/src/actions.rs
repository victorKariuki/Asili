//! LSP code actions (quick fixes)

use tower_lsp::lsp_types::{CodeAction, CodeActionKind, Url};
use asili_diagnostics::Diagnostic;

/// Generate code actions for a diagnostic
pub fn code_actions_for_diagnostic(diagnostic: &Diagnostic, _file_path: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Pattern: missing doc comment (LINT202)
    if diagnostic.code == "LINT202" {
        let action = CodeAction {
            title: "Add doc comment".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: None,
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        };
        actions.push(action);
    }

    // Pattern: unused imports (LINT203)
    if diagnostic.code == "LINT203" {
        let action = CodeAction {
            title: "Remove unused imports".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: None,
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        };
        actions.push(action);
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_for_missing_doc() {
        let diag = Diagnostic::new("LINT202", "missing doc".to_string());
        let diag_with_span = diag.with_span(5, 0);
        let actions = code_actions_for_diagnostic(&diag_with_span, "test.as");
        assert!(actions.iter().any(|a| a.title.contains("doc")));
    }
}
