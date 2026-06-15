//! Symbol extraction: parse a Module into LSP-ready symbol/completion data.

use asili_lexer::tokenize;
use asili_parser::{parse_tokens, Module};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, DocumentSymbol, Location, Position, Range,
    SymbolInformation, SymbolKind, Url,
};
use crate::types::format_type;
use crate::semantic::type_expr_to_value_type;

// ── Keywords always offered in completion ──────────────────────────────────────

const KEYWORDS: &[&str] = &[
    "leta", "kazi", "umbo", "sifa", "shughuli", "ya", "weka", "thabiti", "rejesha",
    "ikiwa", "au_ikiwa", "vinginevyo", "kwa", "katika", "kutoka", "hadi", "wakati",
    "milele", "linganisha", "vunja", "endelea", "lebo", "tupa", "jaribu", "kama",
    "azima", "azima_tenda", "umma", "siyo", "na", "au", "kweli", "si_kweli",
];

const BUILTIN_TYPES: &[&str] = &[
    "Namba", "Neno", "Ukweli", "Herufi", "Tupu", "Hamna",
    "Orodha", "Kamusi", "Jozi", "Chaguo", "Tokeo",
    "Biti8", "Biti16", "Biti32", "Biti64",
    "uBiti8", "uBiti16", "uBiti32", "uBiti64",
];

const BUILTIN_FUNCTIONS: &[&str] = &[
    "chapisha", "paparika", "onyo", "makosa", "omba",
    "orodha", "kamusi", "kamusi_tupu", "jozi", "tokeo", "kosa", "chaguo",
];

// ── Parse helpers ─────────────────────────────────────────────────────────────

/// Try to parse source into a Module. Returns None on lex/parse failure.
pub fn parse_module(source: &str) -> Option<Module> {
    let tokens = tokenize(source).ok()?;
    parse_tokens(&tokens).ok()
}

/// Single 0-based line → LSP Range spanning that whole line.
fn line_range(line: usize) -> Range {
    let l = line.saturating_sub(1) as u32;
    Range {
        start: Position { line: l, character: 0 },
        end: Position { line: l, character: u32::MAX },
    }
}

// ── Completion ────────────────────────────────────────────────────────────────

/// Build completion items for `source`. Returns keyword + type + function items.
pub fn completion_items(source: &str) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = Vec::new();

    // Keywords
    for kw in KEYWORDS {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }

    // Built-in types
    for ty in BUILTIN_TYPES {
        items.push(CompletionItem {
            label: ty.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            ..Default::default()
        });
    }

    // Built-in functions
    for bf in BUILTIN_FUNCTIONS {
        items.push(CompletionItem {
            label: bf.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("(builtin)".to_string()),
            ..Default::default()
        });
    }

    // Module-level symbols from parsed source
    if let Some(module) = parse_module(source) {
        for f in &module.functions {
            let params: Vec<String> = f.params.iter()
                .map(|p| format!("{}: {}", p.name, p.ty.name))
                .collect();
            items.push(CompletionItem {
                label: f.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("kazi {}({}) -> {}", f.name, params.join(", "), f.return_type.name)),
                ..Default::default()
            });
        }
        for s in &module.structs {
            items.push(CompletionItem {
                label: s.name.clone(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some(format!("umbo {}", s.name)),
                ..Default::default()
            });
            // fields as property completions
            for (fname, ftype) in &s.fields {
                let ty_str = ftype.as_ref().map(|t| t.name.as_str()).unwrap_or("?");
                items.push(CompletionItem {
                    label: format!("{}.{}", s.name, fname),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(format!("{}: {}", fname, ty_str)),
                    ..Default::default()
                });
            }
        }
        for t in &module.traits {
            items.push(CompletionItem {
                label: t.name.clone(),
                kind: Some(CompletionItemKind::INTERFACE),
                detail: Some(format!("sifa {}", t.name)),
                ..Default::default()
            });
        }
        for c in &module.constants {
            let ty = format_type(&type_expr_to_value_type(&c.ty));
            items.push(CompletionItem {
                label: c.name.clone(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some(format!("thabiti {}: {}", c.name, ty)),
                ..Default::default()
            });
        }
    }

    items
}

// ── Document symbols (file outline) ──────────────────────────────────────────

/// Build an outline of top-level declarations for the document symbols request.
pub fn document_symbols(source: &str) -> Vec<DocumentSymbol> {
    let module = match parse_module(source) {
        Some(m) => m,
        None => return vec![],
    };

    let mut syms: Vec<DocumentSymbol> = Vec::new();

    for f in &module.functions {
        let range = line_range(f.line);
        #[allow(deprecated)]
        syms.push(DocumentSymbol {
            name: f.name.clone(),
            detail: Some(format!("-> {}", f.return_type.name)),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        });
    }

    for s in &module.structs {
        let range = line_range(s.line);
        let children: Vec<DocumentSymbol> = s.fields.iter().map(|(fname, ftype)| {
            let ty = ftype.as_ref().map(|t| t.name.clone()).unwrap_or_default();
            #[allow(deprecated)]
            DocumentSymbol {
                name: fname.clone(),
                detail: Some(ty),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                range,
                selection_range: range,
                children: None,
            }
        }).collect();
        #[allow(deprecated)]
        syms.push(DocumentSymbol {
            name: s.name.clone(),
            detail: None,
            kind: SymbolKind::STRUCT,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: Some(children),
        });
    }

    for t in &module.traits {
        let range = line_range(t.line);
        #[allow(deprecated)]
        syms.push(DocumentSymbol {
            name: t.name.clone(),
            detail: None,
            kind: SymbolKind::INTERFACE,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        });
    }

    for c in &module.constants {
        let range = line_range(c.line);
        #[allow(deprecated)]
        syms.push(DocumentSymbol {
            name: c.name.clone(),
            detail: Some(c.ty.name.clone()),
            kind: SymbolKind::CONSTANT,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        });
    }

    for e in &module.enums {
        let range = line_range(e.line);
        let children: Vec<DocumentSymbol> = e.variants.iter().map(|v| {
            #[allow(deprecated)]
            DocumentSymbol {
                name: v.name.clone(),
                detail: v.data.as_ref().map(|t| t.name.clone()),
                kind: SymbolKind::ENUM_MEMBER,
                tags: None,
                deprecated: None,
                range: line_range(v.line),
                selection_range: line_range(v.line),
                children: None,
            }
        }).collect();
        #[allow(deprecated)]
        syms.push(DocumentSymbol {
            name: e.name.clone(),
            detail: None,
            kind: SymbolKind::ENUM,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: Some(children),
        });
    }

    syms
}

// ── Workspace symbols ─────────────────────────────────────────────────────────

/// Build SymbolInformation list for all open documents matching `query`.
/// `docs` is an iterator of (uri_string, source_text) pairs.
pub fn workspace_symbols<'a>(
    docs: impl Iterator<Item = (String, String)>,
    query: &str,
) -> Vec<SymbolInformation> {
    let q = query.to_lowercase();
    let mut out = Vec::new();

    for (uri_str, source) in docs {
        let Ok(uri) = uri_str.parse::<Url>() else { continue };
        let module = match parse_module(&source) {
            Some(m) => m,
            None => continue,
        };

        let mut push = |name: &str, kind: SymbolKind, line: usize| {
            if q.is_empty() || name.to_lowercase().contains(&q) {
                #[allow(deprecated)]
                out.push(SymbolInformation {
                    name: name.to_string(),
                    kind,
                    tags: None,
                    deprecated: None,
                    location: Location { uri: uri.clone(), range: line_range(line) },
                    container_name: None,
                });
            }
        };

        for f in &module.functions { push(&f.name, SymbolKind::FUNCTION, f.line); }
        for s in &module.structs   { push(&s.name, SymbolKind::STRUCT, s.line); }
        for t in &module.traits    { push(&t.name, SymbolKind::INTERFACE, t.line); }
        for e in &module.enums     { push(&e.name, SymbolKind::ENUM, e.line); }
        for c in &module.constants { push(&c.name, SymbolKind::CONSTANT, c.line); }
    }

    out
}

// ── Goto Definition ───────────────────────────────────────────────────────────

/// Find the declaration location of the word at `(line_0, char_0)` in `source`.
/// Returns a Location in the same document if found.
pub fn goto_definition(source: &str, uri: &Url, line_0: u32, char_0: u32) -> Option<Location> {
    let tokens = tokenize(source).ok()?;
    let module = parse_tokens(&tokens).ok()?;

    // Find which token the cursor is on.
    let line_1 = (line_0 as usize) + 1;
    let col_1  = (char_0 as usize) + 1;
    let word = tokens.iter().find(|t| {
        t.line == line_1 && t.column <= col_1 && col_1 <= t.column + t.lexeme.len()
    }).map(|t| t.lexeme.as_str())?;

    // Search module-level declarations for that name.
    let decl_line = module.functions.iter().find(|f| f.name == word).map(|f| f.line)
        .or_else(|| module.structs.iter().find(|s| s.name == word).map(|s| s.line))
        .or_else(|| module.traits.iter().find(|t| t.name == word).map(|t| t.line))
        .or_else(|| module.enums.iter().find(|e| e.name == word).map(|e| e.line))
        .or_else(|| module.constants.iter().find(|c| c.name == word).map(|c| c.line))?;

    Some(Location {
        uri: uri.clone(),
        range: line_range(decl_line),
    })
}

// ── References ────────────────────────────────────────────────────────────────

/// Find all occurrences of the word under `(line_0, char_0)` in `source`.
pub fn find_references(
    source: &str,
    uri: &Url,
    line_0: u32,
    char_0: u32,
    include_declaration: bool,
) -> Vec<Location> {
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let line_1 = (line_0 as usize) + 1;
    let col_1  = (char_0 as usize) + 1;

    let word = match tokens.iter().find(|t| {
        t.line == line_1 && t.column <= col_1 && col_1 <= t.column + t.lexeme.len()
    }) {
        Some(t) => t.lexeme.clone(),
        None => return vec![],
    };

    // If we don't want the declaration, find it to exclude.
    let decl_line = if !include_declaration {
        if let Ok(module) = parse_tokens(&tokens) {
            module.functions.iter().find(|f| f.name == word).map(|f| f.line)
                .or_else(|| module.structs.iter().find(|s| s.name == word).map(|s| s.line))
                .or_else(|| module.traits.iter().find(|t| t.name == word).map(|t| t.line))
                .or_else(|| module.enums.iter().find(|e| e.name == word).map(|e| e.line))
                .or_else(|| module.constants.iter().find(|c| c.name == word).map(|c| c.line))
        } else {
            None
        }
    } else {
        None
    };

    tokens.iter()
        .filter(|t| {
            t.lexeme == word && (include_declaration || decl_line != Some(t.line))
        })
        .map(|t| {
            let l = (t.line.saturating_sub(1)) as u32;
            let sc = (t.column.saturating_sub(1)) as u32;
            Location {
                uri: uri.clone(),
                range: Range {
                    start: Position { line: l, character: sc },
                    end: Position { line: l, character: sc + word.len() as u32 },
                },
            }
        })
        .collect()
}

// ── Rename ───────────────────────────────────────────────────────────────────

/// Collect all token spans for `word` in `source` as LSP Ranges (for workspace edits).
pub fn rename_locations(source: &str, word: &str) -> Vec<Range> {
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    tokens.iter()
        .filter(|t| t.lexeme == word)
        .map(|t| {
            let l = (t.line.saturating_sub(1)) as u32;
            let sc = (t.column.saturating_sub(1)) as u32;
            Range {
                start: Position { line: l, character: sc },
                end: Position { line: l, character: sc + word.len() as u32 },
            }
        })
        .collect()
}

/// Extract the word under `(line_0, char_0)` from `source`.
pub fn word_at(source: &str, line_0: u32, char_0: u32) -> Option<String> {
    let tokens = tokenize(source).ok()?;
    let line_1 = (line_0 as usize) + 1;
    let col_1  = (char_0 as usize) + 1;
    tokens.iter().find(|t| {
        t.line == line_1 && t.column <= col_1 && col_1 <= t.column + t.lexeme.len()
    }).map(|t| t.lexeme.clone())
}
