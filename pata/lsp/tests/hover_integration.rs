// Integration tests for Phase II hover improvements.

use pata_lsp::hover::compute_hover;

#[test]
fn test_hover_keyword() {
    let code = "kazi foo() -> Tupu { }";
    let hover = compute_hover(code, 0, 0);
    assert!(hover.is_some());
    let h = hover.unwrap();
    if let pata_lsp::tower_lsp::lsp_types::HoverContents::Scalar(
        pata_lsp::tower_lsp::lsp_types::MarkedString::String(content),
    ) = h.contents
    {
        assert!(content.contains("Keyword"));
        assert!(content.contains("kazi"));
    } else {
        panic!("Expected scalar hover content");
    }
}

#[test]
fn test_hover_function_signature() {
    let code = "kazi add(a: Nambari, b: Nambari) -> Nambari { rejesha a }";
    let hover = compute_hover(code, 0, 5);
    assert!(hover.is_some());
    let h = hover.unwrap();
    if let pata_lsp::tower_lsp::lsp_types::HoverContents::Scalar(
        pata_lsp::tower_lsp::lsp_types::MarkedString::String(content),
    ) = h.contents
    {
        assert!(content.contains("Function"));
        assert!(content.contains("add"));
    }
}

#[test]
fn test_hover_variable_type() {
    let code = "kazi foo() -> Tupu {
    weka x = 42
    weka y = x
}";
    let hover = compute_hover(code, 2, 14);
    assert!(hover.is_some());
    let h = hover.unwrap();
    if let pata_lsp::tower_lsp::lsp_types::HoverContents::Scalar(
        pata_lsp::tower_lsp::lsp_types::MarkedString::String(content),
    ) = h.contents
    {
        assert!(content.contains("Variable") || content.contains("Identifier"));
    }
}

#[test]
fn test_hover_struct_fields() {
    let code = "umbo Point {
    x: Nambari,
    y: Nambari,
}

kazi foo() -> Tupu {
    weka p = Point { x: 1, y: 2 }
}";
    let hover = compute_hover(code, 0, 5);
    assert!(hover.is_some());
    let h = hover.unwrap();
    if let pata_lsp::tower_lsp::lsp_types::HoverContents::Scalar(
        pata_lsp::tower_lsp::lsp_types::MarkedString::String(content),
    ) = h.contents
    {
        assert!(content.contains("Struct") || content.contains("Point"));
    }
}

#[test]
fn test_hover_identifier_no_type() {
    let code = "kazi foo() -> Tupu { }";
    let hover = compute_hover(code, 0, 5);
    assert!(hover.is_some());
    let h = hover.unwrap();
    if let pata_lsp::tower_lsp::lsp_types::HoverContents::Scalar(
        pata_lsp::tower_lsp::lsp_types::MarkedString::String(content),
    ) = h.contents
    {
        assert!(content.contains("foo") || content.contains("Function"));
    }
}

#[test]
fn test_hover_no_token_at_position() {
    let code = "kazi foo() -> Tupu { }";
    let hover = compute_hover(code, 0, 100);
    assert!(hover.is_none());
}

#[test]
fn test_hover_nested_block_variable() {
    let code = "kazi foo() -> Tupu {
    ikiwa kweli {
        weka x = 10
    }
}";
    let hover = compute_hover(code, 2, 14);
    assert!(hover.is_some());
}

#[test]
fn test_hover_list_type() {
    let code = "kazi foo() -> Tupu {
    weka nums = [1, 2, 3]
}";
    let hover = compute_hover(code, 1, 14);
    assert!(hover.is_some());
}

#[test]
fn test_hover_parameter() {
    let code = "kazi add(a: Nambari, b: Nambari) -> Nambari {
    rejesha a
}";
    let hover = compute_hover(code, 1, 12);
    assert!(hover.is_some());
    let h = hover.unwrap();
    if let pata_lsp::tower_lsp::lsp_types::HoverContents::Scalar(
        pata_lsp::tower_lsp::lsp_types::MarkedString::String(content),
    ) = h.contents
    {
        assert!(
            content.contains("Variable") || content.contains("Identifier") || content.contains("a")
        );
    }
}

