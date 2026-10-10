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

fn hover_text(code: &str, line: u32, character: u32) -> String {
    match compute_hover(code, line, character)
        .expect("hover")
        .contents
    {
        pata_lsp::tower_lsp::lsp_types::HoverContents::Scalar(
            pata_lsp::tower_lsp::lsp_types::MarkedString::String(content),
        ) => content,
        other => panic!("Expected scalar hover content: {other:?}"),
    }
}

/// A builtin's hover is its signature (names, optional `?` parameters) and description, from
/// the builtin table.
#[test]
fn test_hover_builtin_function_and_struct() {
    let code = "kazi kuu() -> Tupu {\n    weka j = http_ombi(\"GET\", \"https://mfano.com\", ChaguoHttp { muda: 5 })\n}";
    let line = code.lines().nth(1).unwrap();
    let text = hover_text(code, 1, line.find("http_ombi").unwrap() as u32 + 2);
    assert!(
        text.contains("kazi http_ombi(njia: Neno, anwani: Neno, chaguo?: ChaguoHttp) -> Tokeo<JibuHttp, Neno>"),
        "{text}"
    );
    assert!(
        text.contains("Ombi la HTTP(S)") && text.contains("moduli: `mfumo`"),
        "{text}"
    );
    let text = hover_text(code, 1, line.find("ChaguoHttp").unwrap() as u32 + 2);
    assert!(
        text.contains("umbo ChaguoHttp {") && text.contains("- `muda`: Sekunde"),
        "{text}"
    );
}

#[test]
fn test_signature_help_builtin_names_and_doc() {
    let code = "kazi kuu() -> Tupu {\n    tenda(\"kazi\", 1, 2, 3)\n}";
    let column = code.lines().nth(1).unwrap().find('3').unwrap() as u32;
    let info = pata_lsp::signature::compute_signature_help(code, 1, column, None).expect("help");
    assert_eq!(
        info.label,
        "kazi tenda(kazi_jina: Neno, ...hoja: Haijulikani) -> Tokeo<Namba, Neno>"
    );
    assert_eq!(info.params, vec!["kazi_jina: Neno", "...hoja: Haijulikani"]);
    // The fourth argument is still `...hoja`.
    assert_eq!(info.active_param, 1);
    assert!(info.doc.expect("doc").contains("uzi mpya"));
}
