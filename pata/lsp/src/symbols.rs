//! Symbol extraction: parse a Module into LSP-ready symbol/completion data.

use asili_lexer::tokenize;
use asili_parser::{parse_tokens, Module};
use tower_lsp::lsp_types::{
    Command, CodeLens, CompletionItem, CompletionItemKind, DocumentSymbol, FoldingRange,
    FoldingRangeKind, Location, Position, Range, SymbolInformation, SymbolKind, Url,
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
///
/// NOTE: end.character must stay within LSP's uinteger bound (0..=i32::MAX per the spec
/// and vscode-languageclient's `uinteger.is`). u32::MAX (4294967295) fails that check, which
/// makes vscode-languageclient misdetect the whole documentSymbol response as
/// SymbolInformation[] instead of DocumentSymbol[] and crash reading `.location.range`
/// (undefined) in asSymbolInformation. Do not restore u32::MAX here.
fn line_range(line: usize) -> Range {
    let l = line.saturating_sub(1) as u32;
    Range {
        start: Position { line: l, character: 0 },
        end: Position { line: l, character: i32::MAX as u32 },
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

/// Push every top-level declaration in `module` matching `q` (already-lowercased) into `out`,
/// located at `uri`. Shared by both `workspace_symbols` (open documents, needs to parse their
/// live text) and `workspace_symbols_from_index` (the resolved workspace's already-parsed
/// modules, no re-parsing needed) so the two stay in sync.
fn push_module_symbols(module: &Module, uri: &Url, q: &str, out: &mut Vec<SymbolInformation>) {
    let mut push = |name: &str, kind: SymbolKind, line: usize| {
        if q.is_empty() || name.to_lowercase().contains(q) {
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

/// Build SymbolInformation list for all open documents matching `query`.
/// `docs` is an iterator of (uri_string, source_text) pairs.
pub fn workspace_symbols(
    docs: impl Iterator<Item = (String, String)>,
    query: &str,
) -> Vec<SymbolInformation> {
    let q = query.to_lowercase();
    let mut out = Vec::new();

    for (uri_str, source) in docs {
        let Ok(uri) = uri_str.parse::<Url>() else { continue };
        let Some(module) = parse_module(&source) else { continue };
        push_module_symbols(&module, &uri, &q, &mut out);
    }

    out
}

/// Like `workspace_symbols`, but sourced from the resolved workspace index's already-parsed
/// modules instead of open documents' live text — this is what makes `workspace/symbol` find a
/// struct/function in a project file the user hasn't opened as an editor tab yet, which the
/// open-documents-only version above can never do. `skip_paths` excludes files already covered
/// by an open-document pass (so a currently-edited file's live buffer wins over the workspace
/// index's last-resolved-from-disk snapshot, rather than appearing twice with possibly
/// different content).
pub fn workspace_symbols_from_index(
    index: &crate::workspace::WorkspaceIndex,
    query: &str,
    skip_paths: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<SymbolInformation> {
    let q = query.to_lowercase();
    let mut out = Vec::new();

    for wm in index.modules.values() {
        if skip_paths.contains(&wm.path) {
            continue;
        }
        let Ok(uri) = Url::from_file_path(&wm.path) else { continue };
        push_module_symbols(&wm.module, &uri, &q, &mut out);
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

/// Whether `word` is a function/struct/trait/enum exported (`umma`/`is_public`) from `module`,
/// or a module-level constant (which the language has no privacy concept for — see
/// `WorkspaceIndex::extern_env`, which exposes all of them the same way). Used to decide
/// whether a references search is worth extending beyond the current file at all: a purely
/// local variable can never legitimately be referenced from another module, so there's no
/// point paying for a cross-file search for one.
pub fn is_exported_declaration(module: &Module, word: &str) -> bool {
    module.functions.iter().any(|f| f.name == word && f.is_public)
        || module.structs.iter().any(|s| s.name == word && s.is_public)
        || module.traits.iter().any(|t| t.name == word && t.is_public)
        || module.enums.iter().any(|e| e.name == word && e.is_public)
        || module.constants.iter().any(|c| c.name == word)
}

/// Every token-occurrence of `word` in `source`, located at `uri` — the cross-file half of
/// `find_references`: that function locates the word under the cursor in one file and (via
/// `include_declaration`) can exclude its own declaration line; called once per file this
/// symbol could be referenced from (see `WorkspaceIndex::transitive_importers`), this just
/// finds every occurrence unconditionally, since "is this the declaration line" only makes
/// sense relative to the file that actually declares it.
pub fn find_references_in(word: &str, uri: &Url, source: &str) -> Vec<Location> {
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    tokens
        .iter()
        .filter(|t| t.lexeme == word)
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

/// Like `word_at`, but also returns the token's own range — for `textDocument/prepareRename`,
/// which needs to tell the client exactly what span it's about to let the user retype.
pub fn word_at_range(source: &str, line_0: u32, char_0: u32) -> Option<(String, Range)> {
    let tokens = tokenize(source).ok()?;
    let line_1 = (line_0 as usize) + 1;
    let col_1 = (char_0 as usize) + 1;
    let t = tokens.iter().find(|t| {
        t.line == line_1 && t.column <= col_1 && col_1 <= t.column + t.lexeme.len()
    })?;
    let l = (t.line - 1) as u32;
    let sc = (t.column - 1) as u32;
    Some((
        t.lexeme.clone(),
        Range {
            start: Position { line: l, character: sc },
            end: Position { line: l, character: sc + t.lexeme.len() as u32 },
        },
    ))
}

/// Whether `word` is something a rename should actually be offered for — a real identifier,
/// not a keyword, builtin type, number, string, or bit of punctuation the tokenizer happened
/// to hand back. Used by `prepareRename` to reject renaming e.g. `kazi` or `42`.
pub fn can_rename(word: &str) -> bool {
    let mut chars = word.chars();
    let starts_ident = matches!(chars.next(), Some(c) if c.is_alphabetic() || c == '_');
    starts_ident
        && word.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !KEYWORDS.contains(&word)
        && !BUILTIN_TYPES.contains(&word)
}

/// Folding ranges for every `{ ... }` block spanning more than one line — function/struct/
/// trait/impl bodies, if/while/for/match blocks, and so on.
///
/// Computed by bracket-matching directly on the raw source text (skipping brace look-alikes
/// inside string/char literals and comments) rather than from the AST: `Block` carries no
/// end-line/span today, so getting this from the AST would mean adding one everywhere `Block`
/// appears — a bigger change for the same result. This is less precise on pathological input
/// (an odd number of quotes inside a comment could throw off the string/char tracking for the
/// rest of the file) but correct for any real Asili source.
pub fn folding_ranges(source: &str) -> Vec<FoldingRange> {
    let chars: Vec<char> = source.chars().collect();
    let mut ranges = Vec::new();
    let mut open_lines: Vec<u32> = Vec::new();
    let mut line = 0u32;
    let mut in_string = false;
    let mut in_char = false;
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            // An unterminated string/char literal shouldn't be allowed to swallow the rest of
            // the file's braces — the lexer itself rejects those, so treat newline as a reset.
            in_string = false;
            in_char = false;
            i += 1;
            continue;
        }
        if in_string || in_char {
            if c == '\\' {
                i += 2; // skip the escaped character too
                continue;
            }
            if (in_string && c == '"') || (in_char && c == '\'') {
                in_string = false;
                in_char = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => in_string = true,
            '\'' => in_char = true,
            '#' if chars.get(i + 1) != Some(&'[') => {
                // Line comment (not a `#[attribute]`) — skip to end of line.
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            '{' => open_lines.push(line),
            '}' => {
                if let Some(start_line) = open_lines.pop() {
                    if line > start_line {
                        ranges.push(FoldingRange {
                            start_line,
                            start_character: None,
                            end_line: line,
                            end_character: None,
                            kind: Some(FoldingRangeKind::Region),
                            collapsed_text: None,
                        });
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    ranges
}

/// A "▶ Run Test" code lens above every `#[jaribio]`-attributed function — clicking it runs
/// `asili.runTest`, a command the client (`extension.ts`) registers to shell out to
/// `pata jaribu --filter <name>` in an integrated terminal. `uri` is threaded through as the
/// command's first argument so the client knows which file's project to run the test from.
pub fn test_code_lenses(source: &str, uri: &Url) -> Vec<CodeLens> {
    let module = match parse_module(source) {
        Some(m) => m,
        None => return vec![],
    };

    module
        .functions
        .iter()
        .filter(|f| f.is_test)
        .map(|f| CodeLens {
            range: line_range(f.line),
            command: Some(Command {
                title: "▶ Endesha Jaribio".to_string(),
                command: "asili.runTest".to_string(),
                arguments: Some(vec![
                    serde_json::Value::String(uri.to_string()),
                    serde_json::Value::String(f.name.clone()),
                ]),
            }),
            data: None,
        })
        .collect()
}
