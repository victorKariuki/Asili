//! LSP diagnostics: convert Asili diagnostics and run lex/parse pipeline.

use asili_diagnostics::Diagnostic as AsiliDiagnostic;
use asili_lexer::tokenize;
use asili_parser::{extern_env_from_imports, parse_tokens, semantic_check_with_env};
use pata_lint::lint_source;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

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


pub fn run_lex_parse(text: &str) -> Vec<AsiliDiagnostic> {
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
    if let Err(sem_errors) = semantic_check_with_env(&module, false, extern_fns, extern_consts) {
        out.extend(sem_errors);
    }

    // Run linting to provide style and best-practice warnings
    if let Ok(lint_diags) = lint_source(text) {
        out.extend(lint_diags);
    }

    out
}
