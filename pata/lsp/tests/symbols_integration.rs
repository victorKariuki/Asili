//! Integration tests for completion, goto-definition, references, document symbols,
//! workspace symbols, and rename.

use pata_lsp::symbols::{
    completion_items, document_symbols, find_references, goto_definition,
    rename_locations, word_at, workspace_symbols,
};
use tower_lsp::lsp_types::{SymbolKind, Url};

// Note: struct fields must be comma-separated (parser requirement).
const SOURCE: &str = r#"
kazi salamu(jina: Neno) -> Neno {
  rejesha jina
}

umbo Mtu { jina: Neno, umri: Namba }

thabiti IDADI: Namba = 42
"#;

fn uri() -> Url {
    "file:///test.as".parse().unwrap()
}

// ── Completion ─────────────────────────────────────────────────────────────────

#[test]
fn completion_includes_keywords() {
    let items = completion_items(SOURCE);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"kazi"), "should contain keyword 'kazi'");
    assert!(labels.contains(&"ikiwa"), "should contain keyword 'ikiwa'");
    assert!(labels.contains(&"rejesha"), "should contain keyword 'rejesha'");
}

#[test]
fn completion_includes_user_function() {
    let items = completion_items(SOURCE);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"salamu"), "should contain user function 'salamu'");
}

#[test]
fn completion_includes_user_struct() {
    let items = completion_items(SOURCE);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"Mtu"), "should contain user struct 'Mtu'");
}

#[test]
fn completion_includes_builtin_types() {
    let items = completion_items(SOURCE);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"Namba"), "should contain builtin type 'Namba'");
    assert!(labels.contains(&"Neno"), "should contain builtin type 'Neno'");
}

// ── Document symbols ───────────────────────────────────────────────────────────

#[test]
fn document_symbols_finds_function() {
    let syms = document_symbols(SOURCE);
    let found = syms.iter().any(|s| s.name == "salamu" && s.kind == SymbolKind::FUNCTION);
    assert!(found, "should find function 'salamu'");
}

#[test]
fn document_symbols_finds_struct_with_fields() {
    let syms = document_symbols(SOURCE);
    let s = syms.iter().find(|s| s.name == "Mtu" && s.kind == SymbolKind::STRUCT);
    assert!(s.is_some(), "should find struct 'Mtu'");
    let children = s.unwrap().children.as_ref().expect("struct should have field children");
    assert!(children.iter().any(|c| c.name == "jina"), "should have field 'jina'");
    assert!(children.iter().any(|c| c.name == "umri"), "should have field 'umri'");
}

#[test]
fn document_symbols_finds_constant() {
    let syms = document_symbols(SOURCE);
    let found = syms.iter().any(|s| s.name == "IDADI" && s.kind == SymbolKind::CONSTANT);
    assert!(found, "should find constant 'IDADI'");
}

// ── Goto definition ────────────────────────────────────────────────────────────

#[test]
fn goto_definition_finds_function() {
    // "salamu" appears on line 2 (1-indexed), col 6 (1-indexed) → LSP 0-based: line=1, char=5
    let loc = goto_definition(SOURCE, &uri(), 1, 5);
    assert!(loc.is_some(), "should find definition of 'salamu'");
}

#[test]
fn goto_definition_unknown_returns_none() {
    let loc = goto_definition(SOURCE, &uri(), 0, 0);
    // line 0 in the source is the blank line — no identifier there
    assert!(loc.is_none());
}

// ── References ─────────────────────────────────────────────────────────────────

#[test]
fn find_references_finds_all_occurrences() {
    let src = "weka x = 1\nx + x";
    // "x" appears 3 times: col 5, then col 0 and col 4 on next line (1-indexed).
    // LSP line 0 = "weka x = 1", char 5
    let locs = find_references(src, &uri(), 0, 5, true);
    assert!(!locs.is_empty(), "should find references to 'x'");
    assert!(locs.len() >= 3, "should find all 3 occurrences of 'x', got {}", locs.len());
}

#[test]
fn find_references_excludes_declaration() {
    // "salamu" declared on line 1, called inside two other functions.
    let src = "kazi salamu(x: Namba) -> Namba { rejesha x }\nkazi a() -> Namba { rejesha salamu(1) }\nkazi b() -> Namba { rejesha salamu(2) }";
    // LSP line 0 col 5 = "salamu" in the function declaration
    let with_decl = find_references(src, &uri(), 0, 5, true);
    let without_decl = find_references(src, &uri(), 0, 5, false);
    assert!(
        with_decl.len() > without_decl.len(),
        "excluding declaration should return fewer results (got with={}, without={})",
        with_decl.len(), without_decl.len()
    );
    assert_eq!(without_decl.len(), 2, "should find 2 call-site references");
}

// ── Workspace symbols ──────────────────────────────────────────────────────────

#[test]
fn workspace_symbols_finds_across_docs() {
    let docs = vec![
        ("file:///a.as".to_string(), SOURCE.to_string()),
        ("file:///b.as".to_string(), "kazi hesabu() -> Namba { rejesha 0 }".to_string()),
    ];
    let syms = workspace_symbols(docs.into_iter(), "");
    assert!(syms.iter().any(|s| s.name == "salamu"));
    assert!(syms.iter().any(|s| s.name == "hesabu"));
}

#[test]
fn workspace_symbols_filters_by_query() {
    let docs = vec![("file:///a.as".to_string(), SOURCE.to_string())];
    let syms = workspace_symbols(docs.into_iter(), "sal");
    assert!(syms.iter().all(|s| s.name.to_lowercase().contains("sal")));
}

// ── Rename ─────────────────────────────────────────────────────────────────────

#[test]
fn rename_finds_all_token_spans() {
    let src = "weka x = 1\nx + x";
    let ranges = rename_locations(src, "x");
    assert_eq!(ranges.len(), 3, "should find 3 spans of 'x'");
}

#[test]
fn word_at_returns_token_under_cursor() {
    // SOURCE line 1 (0-based) = "kazi salamu(jina: Neno) -> Neno {"
    // "salamu" starts at col 5 (0-based)
    let word = word_at(SOURCE, 1, 5);
    assert_eq!(word.as_deref(), Some("salamu"));
}
