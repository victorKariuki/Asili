//! LSP diagnostics: convert Asili diagnostics and run lex/parse pipeline.

use asili_diagnostics::Diagnostic as AsiliDiagnostic;
use asili_lexer::tokenize;
use asili_parser::{extern_env_from_imports, merge_modules, parse_tokens, semantic_check_with_env_and_modules};
use pata_lint::{config::LintConfig, lint_source_with_config};
use std::path::Path;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use crate::workspace::WorkspaceIndex;

/// Extract a symbol name from a diagnostic message of the form "prefix: name".
fn extract_symbol_from_message(msg: &str) -> Option<&str> {
    msg.rfind(": ").map(|i| msg[i + 2..].trim())
}

/// Find the byte-column span of `symbol` on `line_text`, searching left-to-right.
/// Returns `(start_char, end_char)` as 0-based character indices, or None if not found.
fn find_symbol_on_line(line_text: &str, symbol: &str) -> Option<(u32, u32)> {
    if symbol.is_empty() {
        return None;
    }
    let mut search = line_text;
    let mut offset = 0usize;
    while let Some(pos) = search.find(symbol) {
        let abs = offset + pos;
        let before = abs == 0 || !line_text.as_bytes()[abs - 1].is_ascii_alphanumeric() && line_text.as_bytes()[abs - 1] != b'_';
        let after_idx = abs + symbol.len();
        let after = after_idx >= line_text.len() || !line_text.as_bytes()[after_idx].is_ascii_alphanumeric() && line_text.as_bytes()[after_idx] != b'_';
        if before && after {
            return Some((abs as u32, after_idx as u32));
        }
        offset += pos + 1;
        search = &search[pos + 1..];
    }
    None
}

pub fn asili_diagnostics_to_lsp_with_source(diags: &[AsiliDiagnostic], source: &str) -> Vec<Diagnostic> {
    let source_lines: Vec<&str> = source.lines().collect();
    diags
        .iter()
        .map(|d| {
            let range = d.span.as_ref().map(|s| {
                let line_idx = (s.line.saturating_sub(1)) as u32;
                let col = (s.column.saturating_sub(1)) as u32;

                // Try to get a precise span from the source text when col is 0 (parser placeholder).
                let (start_col, end_col) = if col == 0 {
                    if let Some(line_text) = source_lines.get(line_idx as usize) {
                        // Extract symbol from the diagnostic message and locate it on the line.
                        let symbol = extract_symbol_from_message(&d.message).unwrap_or("");
                        if let Some((sc, ec)) = find_symbol_on_line(line_text, symbol) {
                            (sc, ec)
                        } else {
                            // Fall back: highlight the first identifier token on the line.
                            let trimmed_start = line_text.len() - line_text.trim_start().len();
                            let rest = line_text.trim_start();
                            let word_len = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
                            (trimmed_start as u32, (trimmed_start + word_len) as u32)
                        }
                    } else {
                        (col, col + 1)
                    }
                } else {
                    (col, col + 1)
                };

                Range {
                    start: Position { line: line_idx, character: start_col },
                    end: Position { line: line_idx, character: end_col },
                }
            }).unwrap_or(Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 1 },
            });
            Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(tower_lsp::lsp_types::NumberOrString::String(d.code.to_string())),
                code_description: None,
                source: Some(d.stage.to_string()),
                message: d.message.clone(),
                related_information: None,
                tags: None,
                data: None,
            }
        })
        .collect()
}


/// `workspace` is the project's resolved cross-file index (see `crate::workspace`), when one
/// is available — `None` degrades to the old stdlib-only behavior (e.g. before the first
/// workspace scan completes, or for a file outside any known project). Without it, a call into
/// your own sibling `.as` file falsely reports as an undefined function/unknown module, and a
/// struct/trait defined there is unrecognized entirely: the stdlib-only extern env and
/// `resolved_modules` set have no way to know that module exists.
///
/// `file_path`, when given, is used to load the project's `[lint.rules]` `pata.toml` table
/// (walking upward via `LintConfig::find_and_load`) so LSP-published lint diagnostics respect
/// the same per-rule severity/options `pata-lint`'s CLI does — `None` (e.g. an unsaved buffer
/// with no on-disk path) lints with every rule at its default settings.
pub fn run_lex_parse(text: &str, workspace: Option<&WorkspaceIndex>, file_path: Option<&Path>) -> Vec<AsiliDiagnostic> {
    let mut out = Vec::new();
    let tokens = match tokenize(text) {
        Ok(t) => t,
        Err(lex_errors) => {
            out.extend(lex_errors);
            return out;
        }
    };
    let module = match parse_tokens(&tokens) {
        Ok(m) => m,
        Err(parse_errors) => {
            out.extend(parse_errors);
            return out;
        }
    };
    let (extern_fns, extern_consts) = extern_env_from_imports(&module);

    // Structs/traits/impls have no extern-signature equivalent in the semantic checker (unlike
    // functions/constants, there's no HashMap<String, ...> parameter for them) — so a
    // project-local struct defined in another file can't be recognized the way a project-local
    // *function* can just by feeding its signature in. Instead, mirror exactly what
    // `pata-cli`'s own compile pipeline does for this (`pipeline/compile.rs`'s
    // `merged_for_eval` checks): merge every module this file actually `leta`s into one Module
    // via `merge_modules`, and semantic-check *that* — cross-file structs/traits/impls are then
    // just structs/traits/impls already sitting in the module being checked. Building the merge
    // from the current file's own `imports` (not unconditionally from every resolved module)
    // also fixes a real gap the previous function/constant-only merge had: it used to expose
    // every project-local function to every file regardless of whether that file actually
    // imported it, so a missing `leta` was never caught.
    let (semantic_module, resolved_modules) = match workspace {
        Some(ws) => {
            let module_map = ws.modules.iter().map(|(k, v)| (k.clone(), v.module.clone())).collect();
            (merge_modules(&module, &module_map), ws.resolved_module_names())
        }
        None => (module, Default::default()),
    };
    if let Err(sem_errors) = semantic_check_with_env_and_modules(&semantic_module, false, extern_fns, extern_consts, resolved_modules) {
        out.extend(sem_errors);
    }

    // Run linting to provide style and best-practice warnings
    let lint_config = file_path
        .and_then(|p| LintConfig::find_and_load(p).ok())
        .unwrap_or_default();
    if let Ok(lint_diags) = lint_source_with_config(text, &lint_config) {
        out.extend(lint_diags);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn long_function_source() -> String {
        let mut src = String::from("kazi ndefu() -> Tupu { ");
        for _ in 0..51 {
            src.push_str("weka x = 1\n");
        }
        src.push_str("rejesha Tupu }");
        src
    }

    #[test]
    fn run_lex_parse_with_no_file_path_uses_default_lint_settings() {
        let src = long_function_source();
        let diags = run_lex_parse(&src, None, None);
        assert!(diags.iter().any(|d| d.code == "LINT101"));
    }

    #[test]
    fn run_lex_parse_respects_project_pata_toml_lint_rules() {
        let root = std::env::temp_dir()
            .join(format!("pata-lsp-test-diag-config-{}", std::process::id()));
        let src_dir = root.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            root.join("pata.toml"),
            "[lint.rules.LINT101]\nseverity = \"ignore\"\n",
        )
        .unwrap();
        let file_path = src_dir.join("kuu.as");
        let src = long_function_source();
        std::fs::write(&file_path, &src).unwrap();

        let diags = run_lex_parse(&src, None, Some(&file_path));
        assert!(
            !diags.iter().any(|d| d.code == "LINT101"),
            "pata.toml's [lint.rules.LINT101] severity=\"ignore\" should suppress LINT101"
        );

        std::fs::remove_dir_all(&root).ok();
    }
}
