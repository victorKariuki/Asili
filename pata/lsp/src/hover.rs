//! Hover response: resolve position to token and module symbol.

use asili_lexer::tokenize;
use asili_parser::parse_tokens;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkedString, Position, Range};

const KEYWORDS: &[&str] = &[
    "leta", "kazi", "umbo", "sifa", "shughuli", "ya", "weka", "thabiti", "rejesha", "ikiwa",
    "vinginevyo", "kwa", "wakati", "linganisha", "vunja", "endelea", "lebo", "tupa", "jaribu",
    "kama", "azima", "azima_tenda", "umma", "katika", "kutoka", "au_ikiwa", "chapisha", "paparika",
];

// TODO(Phase II): compute_hover only shows "Keyword", "Function", or "Identifier" labels.
// Should include: inferred type for variables/params, full function signature with param types
// and return type, struct field list for umbo names, trait method list for sifa names.
// Requires passing the semantic analysis result (typed scopes) alongside the parsed module.

/// Compute hover at (line, character) in LSP 0-based coordinates. Returns None on parse/lex error or no token.
pub fn compute_hover(text: &str, line_0: u32, character_0: u32) -> Option<Hover> {
    let tokens = tokenize(text).ok()?;
    let module = parse_tokens(&tokens).ok()?;
    let line_1 = (line_0 as usize).saturating_add(1);
    let character_1 = (character_0 as usize).saturating_add(1);

    for tok in &tokens {
        if tok.line == line_1
            && tok.column <= character_1
            && character_1 <= tok.column + tok.lexeme.len()
        {
            let content = if KEYWORDS.contains(&tok.lexeme.as_str()) {
                format!("**Keyword:** `{}`", tok.lexeme)
            } else if let Some(f) = module.functions.iter().find(|f| f.name == tok.lexeme) {
                format!("**Function:** `{}`", f.name)
            } else {
                format!("**Identifier:** `{}`", tok.lexeme)
            };
            let start_char = (tok.column.saturating_sub(1)) as u32;
            let end_char = start_char + (tok.lexeme.len() as u32);
            return Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(content)),
                range: Some(Range {
                    start: Position { line: line_0, character: start_char },
                    end: Position { line: line_0, character: end_char },
                }),
            });
        }
    }
    None
}
