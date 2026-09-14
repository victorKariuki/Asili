//! LSP code actions (quick fixes): pure diagnostic → edit logic, unit-testable independent of
//! the `tower_lsp::LanguageServer` trait plumbing. `lib.rs`'s `code_action` handler calls
//! `action_for_diagnostic` once per diagnostic the client reports back for the requested range.

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, Diagnostic as LspDiagnostic, NumberOrString, Position, Range,
    TextEdit, Url, WorkspaceEdit,
};

/// Build the quick-fix `CodeAction` for one diagnostic, if this crate knows a fix for its code.
/// Returns `None` for unrecognized codes, or when a recognized code's diagnostic doesn't carry
/// the shape the fix needs (e.g. an unexpected message format) — callers should skip, not error.
pub fn action_for_diagnostic(diag: &LspDiagnostic, uri: &Url) -> Option<CodeAction> {
    let code = match &diag.code {
        Some(NumberOrString::String(c)) => c.as_str(),
        _ => return None,
    };

    match code {
        "LINT202" => lint202_add_doc_comment(diag, uri),
        "LINT203" => lint203_remove_unused_import(diag, uri),
        _ => None,
    }
}

/// LINT202 ("kazi 'x' haina maelezo (doc comment)"): insert a stub `#` comment line directly
/// above the flagged function, satisfying `pata/lint/src/rules/best_practices.rs`'s check (a
/// `#` line immediately preceding the `kazi`/attribute block). The function name is parsed out
/// of the diagnostic's own message rather than threaded separately — matches the real message
/// text the rule emits (Swahili, `"kazi '<name>' haina maelezo..."`), not a placeholder English
/// string.
fn lint202_add_doc_comment(diag: &LspDiagnostic, uri: &Url) -> Option<CodeAction> {
    let name = diag
        .message
        .strip_prefix("kazi '")
        .and_then(|rest| rest.split('\'').next())?;

    let insert_line = diag.range.start.line;
    let indent = " ".repeat(diag.range.start.character as usize);
    let edit = TextEdit {
        range: Range {
            start: Position { line: insert_line, character: 0 },
            end: Position { line: insert_line, character: 0 },
        },
        new_text: format!("{indent}# TODO: eleza {name}.\n"),
    };
    let mut changes = std::collections::HashMap::new();
    changes.insert(uri.clone(), vec![edit]);

    Some(CodeAction {
        title: format!("Ongeza maelezo kwa '{name}'"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// LINT203 ("'x' imeletwa lakini haitumiki popote"): delete the whole `leta module::{...}` line
/// the diagnostic's span points at. Correct as long as the rule only ever flags a *whole*
/// selective-import line (true today — see `best_practices.rs`'s doc comment on
/// `name_used_outside_import_line`); doesn't yet narrow to just the unused name within a mixed
/// `{used, unused}` import, since the diagnostic's span is the import line, not a sub-range.
fn lint203_remove_unused_import(diag: &LspDiagnostic, uri: &Url) -> Option<CodeAction> {
    let delete_line = diag.range.start.line;
    let edit = TextEdit {
        range: Range {
            start: Position { line: delete_line, character: 0 },
            end: Position { line: delete_line + 1, character: 0 },
        },
        new_text: String::new(),
    };
    let mut changes = std::collections::HashMap::new();
    changes.insert(uri.clone(), vec![edit]);

    Some(CodeAction {
        title: "Ondoa leta isiyotumika".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::DiagnosticSeverity;

    fn diag(code: &str, message: &str, line: u32, character: u32) -> LspDiagnostic {
        LspDiagnostic {
            range: Range {
                start: Position { line, character },
                end: Position { line, character: character + 1 },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(code.to_string())),
            code_description: None,
            source: None,
            message: message.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn test_uri() -> Url {
        Url::parse("file:///test.as").unwrap()
    }

    #[test]
    fn lint202_produces_a_real_insert_edit() {
        let d = diag("LINT202", "kazi 'jumlisha' haina maelezo (doc comment)", 4, 0);
        let action = action_for_diagnostic(&d, &test_uri()).expect("action");
        assert_eq!(action.title, "Ongeza maelezo kwa 'jumlisha'");
        let edit = action.edit.expect("edit must be Some, not inert");
        let changes = edit.changes.expect("changes map");
        let edits = changes.get(&test_uri()).expect("edit for this uri");
        assert_eq!(edits.len(), 1);
        assert!(edits[0].new_text.contains("jumlisha"));
        // Inserted before the flagged line, not replacing it.
        assert_eq!(edits[0].range.start, edits[0].range.end);
        assert_eq!(edits[0].range.start.line, 4);
    }

    #[test]
    fn lint202_indents_the_inserted_comment_to_match() {
        let d = diag("LINT202", "kazi 'ndani' haina maelezo (doc comment)", 2, 4);
        let action = action_for_diagnostic(&d, &test_uri()).expect("action");
        let edits = action.edit.unwrap().changes.unwrap().remove(&test_uri()).unwrap();
        assert!(edits[0].new_text.starts_with("    #"));
    }

    #[test]
    fn lint203_produces_a_real_line_deletion_edit() {
        let d = diag("LINT203", "'soma' imeletwa lakini haitumiki popote", 0, 0);
        let action = action_for_diagnostic(&d, &test_uri()).expect("action");
        assert_eq!(action.title, "Ondoa leta isiyotumika");
        let edits = action.edit.unwrap().changes.unwrap().remove(&test_uri()).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "");
        // Spans the whole line (start of line 0 through start of line 1).
        assert_eq!(edits[0].range.start, Position { line: 0, character: 0 });
        assert_eq!(edits[0].range.end, Position { line: 1, character: 0 });
    }

    #[test]
    fn unrecognized_code_yields_no_action() {
        let d = diag("LINT101", "kazi ni ndefu mno", 0, 0);
        assert!(action_for_diagnostic(&d, &test_uri()).is_none());
    }

    #[test]
    fn lint202_with_unexpected_message_shape_yields_no_action() {
        // Guards against silently regenerating the original bug (matching an English prefix
        // against a Swahili message) — a message that doesn't start with "kazi '" must not
        // produce a bogus action instead of being skipped.
        let d = diag("LINT202", "something else entirely", 0, 0);
        assert!(action_for_diagnostic(&d, &test_uri()).is_none());
    }
}
