//! Best practices lint rules

use asili_diagnostics::Diagnostic;
use asili_parser::Module;
use std::collections::HashMap;

/// Extract the contents of every `"..."` string literal in `source`, in source order. Skips
/// `#` line comments (including any quotes they contain, e.g. a doc comment like
/// `# Renders as "Ndiyo"/"Hapana"` — those aren't real code string literals) — `#[...]`
/// attributes are not comments and are scanned normally. A minimal scanner (handles `\"`
/// escapes) — good enough for a duplicate-literal heuristic, not a full lexer pass.
fn extract_string_literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '#' && chars.peek() != Some(&'[') {
                break; // rest of the line is a comment
            }
            if c != '"' {
                continue;
            }
            let mut lit = String::new();
            loop {
                match chars.next() {
                    None => break,
                    Some('"') => break,
                    Some('\\') => {
                        if let Some(next) = chars.next() {
                            lit.push('\\');
                            lit.push(next);
                        }
                    }
                    Some(c) => lit.push(c),
                }
            }
            out.push(lit);
        }
    }
    out
}

/// Check for best practice violations
pub fn check_best_practices(module: &Module, source: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Flag string literals that are genuinely repeated (appear more than once) — the message
    // always said "extracting repeated ones", but the rule used to just count total string
    // literals in the file, firing on any file with more than 5 of them even when none repeat
    // (common in demo/test-fixture code with many distinct one-off messages). Now it actually
    // checks for repetition, matching what it claims to check.
    let mut counts: HashMap<String, usize> = HashMap::new();
    for lit in extract_string_literals(source) {
        if lit.trim().is_empty() {
            continue;
        }
        *counts.entry(lit).or_insert(0) += 1;
    }
    let mut repeated: Vec<(String, usize)> = counts.into_iter().filter(|(_, n)| *n > 1).collect();
    if !repeated.is_empty() {
        repeated.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let summary = repeated
            .iter()
            .take(5)
            .map(|(s, n)| format!("\"{s}\" x{n}"))
            .collect::<Vec<_>>()
            .join(", ");
        diags.push(
            Diagnostic::new("LINT201", format!(
                "maneno {} yanarudiwa bila kubadilika — fikiria kuyafanya thabiti: {}",
                repeated.len(),
                summary
            ))
            .with_stage("ukaguzi")
        );
    }

    // Check for functions without documentation: a `#` line comment (not a `#[...]`
    // attribute) immediately above the function, skipping over any attribute lines in
    // between (e.g. `# Squares a number\n#[jaribio]\nkazi ...`).
    let lines: Vec<&str> = source.lines().collect();
    for func in &module.functions {
        if func.name == "kuu" || func.name.starts_with("test_") {
            continue;
        }
        let mut i = func.line as isize - 2; // 0-based index of the line above `kazi ...`
        while i >= 0 {
            let l = lines[i as usize].trim();
            if l.starts_with("#[") && l.ends_with(']') {
                i -= 1;
                continue;
            }
            break;
        }
        let has_doc_comment = i >= 0 && {
            let l = lines[i as usize].trim();
            l.starts_with('#') && !l.starts_with("#[")
        };
        if !has_doc_comment {
            diags.push(
                Diagnostic::new("LINT202", format!(
                    "kazi '{}' haina maelezo (doc comment)",
                    func.name
                ))
                .with_stage("ukaguzi")
                .with_span(func.line, 1)
            );
        }
    }

    // Check for unused imports
    if module.imports.len() > 0 && module.functions.is_empty() {
        diags.push(
            Diagnostic::new("LINT203", "faili lina leta lakini hakuna kanuni inayoyatumia".to_string())
            .with_stage("ukaguzi")
        );
    }

    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use asili_lexer::tokenize;
    use asili_parser::parse_tokens;

    fn lint(src: &str) -> Vec<Diagnostic> {
        let tokens = tokenize(src).expect("tokenize");
        let module = parse_tokens(&tokens).expect("parse");
        check_best_practices(&module, src)
    }

    #[test]
    fn lint202_flags_undocumented_function() {
        let src = "kazi undocumented() -> Tupu { rejesha Tupu }";
        let diags = lint(src);
        assert!(diags.iter().any(|d| d.code == "LINT202"));
    }

    #[test]
    fn lint202_accepts_documented_function() {
        let src = "# This is documented\nkazi documented() -> Tupu { rejesha Tupu }";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT202"));
    }

    #[test]
    fn lint203_flags_unused_imports() {
        let src = "leta msingi";
        let diags = lint(src);
        assert!(diags.iter().any(|d| d.code == "LINT203"));
    }

    #[test]
    fn lint203_accepts_used_imports() {
        let src = "leta msingi\nkazi kuu() -> Tupu { rejesha Tupu }";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT203"));
    }

    #[test]
    fn lint202_accepts_main_function() {
        let src = "kazi kuu() -> Tupu { rejesha Tupu }";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT202"));
    }

    #[test]
    fn lint202_accepts_test_function() {
        let src = "#[jaribio]\nkazi test_something() -> Tupu { rejesha Tupu }";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT202"));
    }

    #[test]
    fn lint201_accepts_unique_strings() {
        let src = r#"kazi example() -> Tupu { chapisha("a") chapisha("b") rejesha Tupu }"#;
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT201"));
    }
}
