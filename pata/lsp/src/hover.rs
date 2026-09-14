//! Hover response: resolve position to token and module symbol with type information.

use asili_lexer::tokenize;
use asili_parser::parse_tokens;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkedString, Position, Range};
use crate::semantic::SemanticAnalyzer;
use crate::hover_format::{
    format_keyword_hover, format_function_hover, format_variable_hover, format_struct_hover,
    format_trait_hover, format_identifier_hover,
};

const KEYWORDS: &[&str] = &[
    "leta", "kazi", "umbo", "sifa", "shughuli", "ya", "weka", "thabiti", "rejesha", "ikiwa",
    "vinginevyo", "kwa", "wakati", "linganisha", "vunja", "endelea", "lebo", "tupa", "jaribu",
    "kama", "azima", "azima_tenda", "umma", "katika", "kutoka", "au_ikiwa", "chapisha", "paparika",
];

/// Compute hover at (line, character) in LSP 0-based coordinates with semantic analysis.
/// Returns None on parse/lex error or no token.
pub fn compute_hover(text: &str, line_0: u32, character_0: u32) -> Option<Hover> {
    let tokens = tokenize(text).ok()?;
    let module = parse_tokens(&tokens).ok()?;
    let line_1 = (line_0 as usize).saturating_add(1);
    let character_1 = (character_0 as usize).saturating_add(1);

    let mut analyzer = SemanticAnalyzer::new(module.clone());
    let _semantic_tokens = analyzer.analyze();

    for tok in &tokens {
        if tok.line == line_1
            && tok.column <= character_1
            && character_1 <= tok.column + tok.lexeme.len()
        {
            let content = if KEYWORDS.contains(&tok.lexeme.as_str()) {
                format_keyword_hover(&tok.lexeme)
            } else if let Some(hover_info) = analyzer.get_hover_info(&tok.lexeme) {
                use crate::types::HoverInfo;
                match hover_info {
                    HoverInfo::Keyword(word) => format_keyword_hover(&word),
                    HoverInfo::Variable { name, type_ } => format_variable_hover(&name, &type_),
                    HoverInfo::Function {
                        name,
                        params,
                        return_type,
                    } => {
                        let param_types = params.iter().map(|(_, t)| t.clone()).collect();
                        let func = module.functions.iter().find(|f| f.name == name)?;
                        format_function_hover(func, param_types, &return_type)
                    }
                    HoverInfo::Struct { name, fields } => {
                        let struct_decl = module.structs.iter().find(|s| s.name == name)?;
                        let field_types = fields.iter().map(|(_, t)| t.clone()).collect();
                        format_struct_hover(struct_decl, field_types)
                    }
                    HoverInfo::Trait { name, .. } => {
                        let trait_decl = module.traits.iter().find(|t| t.name == name)?;
                        format_trait_hover(trait_decl)
                    }
                    HoverInfo::Identifier(name) => format_identifier_hover(&name),
                }
            } else if let Some(f) = module.functions.iter().find(|f| f.name == tok.lexeme) {
                let return_type = analyzer.get_hover_info(&f.name).and_then(|h| {
                    use crate::types::HoverInfo;
                    if let HoverInfo::Function {
                        return_type, ..
                    } = h
                    {
                        Some(return_type)
                    } else {
                        None
                    }
                });
                if let Some(return_type) = return_type {
                    let param_types = f.params.iter()
                        .map(|p| crate::semantic::type_expr_to_value_type(&p.ty))
                        .collect();
                    format_function_hover(f, param_types, &return_type)
                } else {
                    format!("**Function:** `{}`", f.name)
                }
            } else {
                format_identifier_hover(&tok.lexeme)
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
