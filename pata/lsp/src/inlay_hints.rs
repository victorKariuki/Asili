//! Inlay hints for LSP: inferred types on un-annotated `weka`/`thabiti` bindings.
//!
//! Built on `SemanticAnalyzer::inlay_type_hints` (see `semantic.rs`), which does real inference
//! via the same `infer_expr_type` walk that drives semantic-token scope resolution — not a
//! text/regex scan. A binding only gets a hint when its inferred type is concrete (not
//! `ValueType::Unknown`); an already-annotated `weka x: Namba = ...` never gets one, since the
//! annotation is already visible in the source.

use asili_lexer::tokenize;
use asili_parser::parse_tokens;
use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position};

use crate::semantic::SemanticAnalyzer;

/// Compute inlay hints for a document: `: <Aina>` after every un-annotated `weka`/`thabiti`
/// binding name. Returns an empty list (not an error) for unparseable source — same
/// fails-quiet convention as `hover`/`semantic_tokens` elsewhere in this crate.
pub fn compute_inlay_hints(text: &str) -> Vec<InlayHint> {
    let Ok(tokens) = tokenize(text) else { return Vec::new() };
    let Ok(module) = parse_tokens(&tokens) else { return Vec::new() };

    let mut analyzer = SemanticAnalyzer::new(module);
    analyzer.analyze();

    analyzer
        .inlay_type_hints()
        .iter()
        .map(|(line, column, value_type)| InlayHint {
            position: Position {
                line: *line as u32,
                character: *column as u32,
            },
            label: InlayHintLabel::String(format!(": {value_type}")),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: None,
            padding_left: Some(true),
            padding_right: Some(false),
            data: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_un_annotated_number_binding() {
        let code = "kazi kuu() -> Tupu {\n    weka x = 42\n}\n";
        let hints = compute_inlay_hints(code);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].kind, Some(InlayHintKind::TYPE));
        match &hints[0].label {
            InlayHintLabel::String(s) => assert_eq!(s, ": Namba"),
            _ => panic!("expected a string label"),
        }
        // Position should land right after `x` (line 1, 0-based), not at the start of the line.
        assert_eq!(hints[0].position.line, 1);
    }

    #[test]
    fn no_hint_for_explicitly_annotated_binding() {
        let code = "kazi kuu() -> Tupu {\n    weka x: Namba = 42\n}\n";
        let hints = compute_inlay_hints(code);
        assert!(hints.is_empty());
    }

    #[test]
    fn no_hint_for_unparseable_source() {
        let code = "weka ===";
        let hints = compute_inlay_hints(code);
        assert!(hints.is_empty());
    }

    #[test]
    fn hints_string_binding() {
        let code = "kazi kuu() -> Tupu {\n    weka jina = \"Asili\"\n}\n";
        let hints = compute_inlay_hints(code);
        assert_eq!(hints.len(), 1);
        match &hints[0].label {
            InlayHintLabel::String(s) => assert_eq!(s, ": Neno"),
            _ => panic!("expected a string label"),
        }
    }
}
