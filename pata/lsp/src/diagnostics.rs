//! LSP diagnostics: convert Asili diagnostics and run lex/parse pipeline.

use asili_diagnostics::Diagnostic as AsiliDiagnostic;
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_options};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

pub fn asili_diagnostics_to_lsp(diags: &[AsiliDiagnostic]) -> Vec<Diagnostic> {
    diags
        .iter()
        .map(|d| {
            let range = d.span.as_ref().map(|s| {
                let line = (s.line.saturating_sub(1)) as u32;
                let col = (s.column.saturating_sub(1)) as u32;
                Range {
                    start: Position { line, character: col },
                    end: Position {
                        line,
                        character: col.saturating_add(1),
                    },
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
    if let Err(sem_errors) = semantic_check_with_options(&module, false) {
        out.extend(sem_errors);
    }
    out
}
