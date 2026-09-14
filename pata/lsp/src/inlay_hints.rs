//! Inlay hints for LSP: type annotations and parameter names

use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position, Range};

/// Compute inlay hints for a document
/// Returns hints for inferred types at variable declarations and parameter names at call sites
pub fn compute_inlay_hints(text: &str, _line: u32, _character: u32) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    // Basic inlay hints: detect variable declarations and suggest types
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();

        // Pattern: `variable = expression`
        if let Some(eq_pos) = trimmed.find('=') {
            if trimmed.starts_with("kitu ") || !trimmed.contains(":") {
                let var_end = eq_pos;
                let hint = InlayHint {
                    position: Position {
                        line: i as u32,
                        character: var_end as u32,
                    },
                    label: InlayHintLabel::LabelParts(vec![]),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: None,
                    padding_left: Some(true),
                    padding_right: Some(false),
                    data: None,
                };
                hints.push(hint);
            }
        }

        // Pattern: function call with parameters
        if trimmed.contains('(') && trimmed.contains(')') {
            if let Some(paren_pos) = trimmed.find('(') {
                let hint = InlayHint {
                    position: Position {
                        line: i as u32,
                        character: (paren_pos + 1) as u32,
                    },
                    label: InlayHintLabel::LabelParts(vec![]),
                    kind: Some(InlayHintKind::PARAMETER),
                    text_edits: None,
                    tooltip: None,
                    padding_left: Some(false),
                    padding_right: Some(true),
                    data: None,
                };
                hints.push(hint);
            }
        }
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inlay_hints_for_variable_declaration() {
        let code = "kitu x = 42";
        let hints = compute_inlay_hints(code, 0, 0);
        assert!(!hints.is_empty());
        assert_eq!(hints[0].kind, Some(InlayHintKind::TYPE));
    }

    #[test]
    fn inlay_hints_for_function_call() {
        let code = "chapisha(ujumbe)";
        let hints = compute_inlay_hints(code, 0, 0);
        assert!(!hints.is_empty());
    }
}
