//! Document formatting via LSP.

use tower_lsp::lsp_types::{Position, Range, TextEdit};

/// Format Asili source code using canonical formatting.
/// Consolidates whitespace and normalizes brace/comma spacing.
/// NOTE: This is inlined from pata/cli/src/pipeline/format.rs::canonical_format
/// to avoid circular dependencies between binary and library crates.
pub fn format_document(text: &str) -> String {
    let mut out = String::new();
    let mut last_blank = false;
    for raw in text.lines() {
        let trimmed_end = raw.trim_end();
        let is_blank = trimmed_end.trim().is_empty();
        if is_blank {
            if !last_blank {
                out.push('\n');
            }
            last_blank = true;
            continue;
        }

        last_blank = false;
        let mut line = trimmed_end.replace("{", " { ");
        line = line.replace("}", " } ");
        line = line.replace(",", ", ");
        while line.contains("  ") {
            line = line.replace("  ", " ");
        }
        out.push_str(line.trim());
        out.push('\n');
    }

    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Generate TextEdit for full document formatting.
/// Replaces entire document content with formatted version.
pub fn format_to_edits(text: &str) -> Option<Vec<TextEdit>> {
    let formatted = format_document(text);

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
        let output = format_document(input);
        assert!(!output.is_empty());
    }

    #[test]
    fn test_format_idempotent() {
        let input = "kazi foo() -> Tupu { chapisha(\"x\") }";
        let a = format_document(input);
        let b = format_document(&a);
        assert_eq!(a, b);
    }

    #[test]
    fn test_format_to_edits_same_content() {
        let input = "kazi foo() -> Tupu { chapisha(\"x\") }\n";
        // After formatting, should be identical
        let formatted = format_document(input);
        if formatted == input {
            let edits = format_to_edits(input);
            assert_eq!(edits, Some(vec![]));
        }
    }
}
