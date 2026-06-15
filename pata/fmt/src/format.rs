/// Canonical source formatting for Asili code.
/// Provides stable, consistent style for diffs and CI.
pub fn canonical_format(input: &str) -> String {
    let mut out = String::new();
    let mut last_blank = false;

    for raw in input.lines() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_spacing() {
        let input = "kazi foo()  { chapisha( \"x\" ) }";
        let output = canonical_format(input);
        assert!(output.contains("foo()"));
        assert!(output.contains("{ "));
        assert!(output.contains("( \"x\" )"));
    }

    #[test]
    fn test_format_idempotent() {
        let input = "kazi foo() -> Tupu { chapisha(\"x\") }\n";
        let a = canonical_format(input);
        let b = canonical_format(&a);
        assert_eq!(a, b);
    }

    #[test]
    fn test_format_blank_consolidation() {
        let input = "kazi foo() -> Tupu {\n\n\n  chapisha(\"x\")\n\n}";
        let output = canonical_format(input);
        assert!(output.matches('\n').count() <= 5);
    }
}
