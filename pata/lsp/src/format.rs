//! Document formatting via LSP: delegates to `pata-fmt`'s real token-stream printer
//! (`pata_fmt::canonical_format_with_indent`), respecting a project's `[fmt]` `pata.toml`
//! indent settings the same way `pata nadhifu`'s own CLI does. Previously this module carried a
//! hand-inlined copy of a much weaker line-based whitespace normalizer (blind string-replace on
//! `{`/`}`/`,`, including inside string literals) — its own doc comment claimed it was "inlined
//! from pata/cli/src/pipeline/format.rs::canonical_format" to avoid a circular dependency, but
//! `pata-fmt` was a `[[bin]]`-only crate with no `[lib]` target at the time, so there was nothing
//! real to inline from or depend on. Adding a `[lib]` target to `pata-fmt` (this session) retired
//! that gap — format-on-save in the editor now produces exactly the same output `pata nadhifu`
//! would for the same file, not a separately-maintained, weaker approximation.

use pata_fmt::config::FormatterConfig;
use std::path::Path;
use tower_lsp::lsp_types::{Position, Range, TextEdit};

/// Format Asili source using the real canonical formatter, honoring `[fmt]` indent settings
/// from the nearest ancestor `pata.toml` above `file_path` (when given — `None`, e.g. an
/// unsaved buffer with no on-disk path, formats with the default 4-space indent).
pub fn format_document(text: &str, file_path: Option<&Path>) -> String {
    let config = file_path
        .and_then(|p| FormatterConfig::find_and_load(p).ok())
        .unwrap_or_default();
    pata_fmt::canonical_format_with_indent(text, &config.indent_unit())
}

/// Generate TextEdit for full document formatting.
/// Replaces entire document content with formatted version.
pub fn format_to_edits(text: &str, file_path: Option<&Path>) -> Option<Vec<TextEdit>> {
    let formatted = format_document(text, file_path);

    if formatted == text {
        return Some(vec![]);
    }

    // Count lines in original document to create end range
    let line_count = text.lines().count() as u32;
    let last_line = if line_count == 0 { 0 } else { line_count - 1 };
    let last_line_len = text
        .lines()
        .last()
        .map(|l| l.len() as u32)
        .unwrap_or(0);

    // Replace entire document
    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: last_line,
                character: last_line_len,
            },
        },
        new_text: formatted,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple() {
        let input = "kazi foo( ) -> Tupu {\n    chapisha( \"x\" )\n}";
        let output = format_document(input, None);
        assert!(!output.is_empty());
    }

    #[test]
    fn test_format_idempotent() {
        let input = "kazi foo() -> Tupu { chapisha(\"x\") }";
        let a = format_document(input, None);
        let b = format_document(&a, None);
        assert_eq!(a, b);
    }

    #[test]
    fn test_format_to_edits_same_content() {
        let input = "kazi foo() -> Tupu { chapisha(\"x\") }\n";
        // After formatting, should be identical
        let formatted = format_document(input, None);
        if formatted == input {
            let edits = format_to_edits(input, None);
            assert_eq!(edits, Some(vec![]));
        }
    }

    /// The real formatter's biggest observable behavior difference from the old inline
    /// normalizer: it must never rewrite the contents of a string literal — the old text
    /// transform did blind brace/comma replacement everywhere, including inside strings.
    #[test]
    fn test_format_preserves_string_literal_contents() {
        let input = r#"chapisha("a, b {c}")"#;
        let output = format_document(input, None);
        assert!(output.contains(r#""a, b {c}""#), "string literal contents must survive formatting verbatim, got: {output}");
    }

    #[test]
    fn test_format_respects_project_indent_config() {
        let root = std::env::temp_dir().join(format!("pata-lsp-fmt-test-{}", std::process::id()));
        let src_dir = root.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            root.join("pata.toml"),
            "[fmt]\nindent_style = \"tabs\"\n",
        ).unwrap();
        let file_path = src_dir.join("kuu.as");

        let input = "kazi kuu() -> Tupu {\nweka x = 1\n}\n";
        let output = format_document(input, Some(&file_path));
        assert!(output.contains("\n\tweka x = 1"), "should use tab indent from pata.toml, got: {output:?}");

        std::fs::remove_dir_all(&root).ok();
    }
}
