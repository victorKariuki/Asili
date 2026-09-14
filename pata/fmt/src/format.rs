use asili_lexer::{tokenize_with_trivia, Comment, Token};

/// Canonical source formatting for Asili code.
/// Provides stable, consistent style for diffs and CI.
///
/// Token-stream-driven, not AST-based: re-tokenizes the input (capturing comments as trivia via
/// `tokenize_with_trivia`, since they aren't part of the AST) and re-emits tokens with layout
/// rules driven by token kind and brace/paren nesting depth. A full CST/lossless-syntax-tree
/// rewrite was rejected because AST nodes only carry a start line/column today, not an end span
/// — span-slicing the original source isn't viable without a larger, cross-cutting AST change.
///
/// On a lex error (e.g. an unterminated string), falls back to returning the input unchanged
/// rather than panicking or producing a mangled partial rewrite — `pata nadhifu` is meant to be
/// safe to run on in-progress, possibly-invalid source.
pub fn canonical_format(input: &str) -> String {
    let (tokens, comments) = match tokenize_with_trivia(input) {
        Ok(pair) => pair,
        Err(_) => return input.to_string(),
    };
    if tokens.is_empty() && comments.is_empty() {
        return String::new();
    }
    Printer::new(&tokens, &comments).print()
}

/// True for delimiters that print with no space before them, and `NO_SPACE_AFTER` for no space
/// after — punctuation should hug the token it's attached to rather than floating with a space
/// on both sides.
///
/// `(` and `[` are call/index syntax (`foo(x)`, `a[0]`) only when they immediately follow an
/// identifier or a closing `)`/`]` — otherwise they're a grouping expression (`1 + (2 * 3)`) or
/// an array literal (`weka a = [1, 2, 3]`), which keep the space a preceding operator/keyword
/// already gets. See `Printer::hugs_previous`.
const NO_SPACE_BEFORE: &[&str] = &[")", "]", ",", ";", ":", ".", "?"];
const NO_SPACE_AFTER: &[&str] = &["(", "[", ".", "#"];
/// Keywords that can precede a grouping `(...)` or an array literal `[...]` without that being
/// call/index syntax (`rejesha (x)`, not `rejesha(x)` as a call). Sourced from every string
/// literal `parse.rs`'s `match_tok` checks against, since this lexer has no token-kind
/// classification of its own to query. `kama`/`kweli`/`si_kweli` (comparison/booleans) don't
/// need listing here — they're already excluded by not being an identifier-or-closer below.
const KEYWORDS: &[&str] = &[
    "_", "au_ikiwa", "endelea", "ikiwa", "jenum", "kama", "katika", "kazi", "kutoka", "kwa",
    "kweli", "lebo", "leta", "linganisha", "milele", "rejesha", "shughuli", "sifa", "si_kweli",
    "thabiti", "tupa", "umbo", "umma", "vinginevyo", "vunja", "wakati", "weka", "ya", "jaribu",
];
/// Opens a new indented block; the matching close dedents before printing.
const OPENERS: &[&str] = &["{"];
const CLOSERS: &[&str] = &["}"];

struct Printer<'a> {
    tokens: &'a [Token],
    comments: &'a [Comment],
    out: String,
    depth: usize,
    /// Comments already emitted (`after_token_index`), so a comment isn't printed twice when
    /// multiple comments share the same anchor.
    next_comment: usize,
    /// Indices of `<`/`>` tokens classified as generic-type brackets (`Orodha<Neno>`), not the
    /// less-than/greater-than operators — see `classify_generic_brackets`. These print with no
    /// surrounding spaces, unlike the comparison operators sharing the same lexemes.
    generic_brackets: std::collections::HashSet<usize>,
}

impl<'a> Printer<'a> {
    fn new(tokens: &'a [Token], comments: &'a [Comment]) -> Self {
        let generic_brackets = classify_generic_brackets(tokens);
        Self { tokens, comments, out: String::new(), depth: 0, next_comment: 0, generic_brackets }
    }

    fn print(mut self) -> String {
        // Comments preceding the very first token (after_token_index == None).
        self.emit_comments_after(None);

        let mut i = 0;
        while i < self.tokens.len() {
            let tok = &self.tokens[i];
            self.emit_token(tok, i);
            self.emit_comments_after(Some(i));
            i += 1;
        }

        self.trim_trailing_space();
        if !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        // Collapse 3+ consecutive blank lines down to at most one, mirroring the previous
        // implementation's blank-line consolidation behavior.
        collapse_blank_lines(&self.out)
    }

    fn emit_token(&mut self, tok: &Token, idx: usize) {
        let rendered = render_lexeme(&tok.lexeme);
        let lex: &str = &rendered;

        if CLOSERS.contains(&lex) {
            self.depth = self.depth.saturating_sub(1);
            self.trim_trailing_space();
            if !self.out.is_empty() && !self.out.ends_with('\n') {
                self.newline_indent();
            }
        }

        let is_generic_bracket = self.generic_brackets.contains(&idx);
        let hugs_left =
            NO_SPACE_BEFORE.contains(&lex) || self.is_call_or_index_open(lex, idx) || is_generic_bracket;
        if hugs_left {
            self.trim_trailing_space();
        }

        let needs_space_before = idx > 0
            && !self.out.ends_with('\n')
            && !self.out.ends_with(' ')
            && !hugs_left
            && !self.prev_suppresses_space_after(idx);

        if needs_space_before {
            self.out.push(' ');
        }

        self.out.push_str(lex);

        if OPENERS.contains(&lex) {
            self.depth += 1;
            self.newline_indent();
        } else if lex == ";" || self.starts_new_line_after(idx) {
            // A gap of 2+ source lines between tokens means the author left at least one blank
            // line — preserve exactly one, so paragraph breaks between top-level declarations
            // survive instead of being silently deleted.
            if self.has_blank_gap_after(idx) {
                self.out.push('\n');
            }
            self.newline_indent();
        } else if !NO_SPACE_AFTER.contains(&lex) && !(lex == "<" && is_generic_bracket) {
            self.out.push(' ');
        }
    }

    /// A statement boundary: the next token sits on a later source line than this one, and
    /// neither token is a delimiter that should hug its neighbor — used to preserve the
    /// author's own line breaks between statements rather than collapsing everything onto one
    /// line, since this printer has no statement-level AST to drive layout from.
    fn starts_new_line_after(&self, idx: usize) -> bool {
        let Some(next) = self.tokens.get(idx + 1) else { return false };
        let cur = &self.tokens[idx];
        if OPENERS.contains(&cur.lexeme.as_str()) || CLOSERS.contains(&next.lexeme.as_str()) {
            return false;
        }
        next.line > cur.line
    }

    fn has_blank_gap_after(&self, idx: usize) -> bool {
        match self.tokens.get(idx + 1) {
            Some(next) => next.line > self.tokens[idx].line + 1,
            None => false,
        }
    }

    fn prev_suppresses_space_after(&self, idx: usize) -> bool {
        idx > 0
            && (NO_SPACE_AFTER.contains(&self.tokens[idx - 1].lexeme.as_str())
                || (self.tokens[idx - 1].lexeme == "<" && self.generic_brackets.contains(&(idx - 1))))
    }

    /// True when `lex` is `(` or `[` immediately following a call/index target — an identifier,
    /// or a closing `)`/`]` (chained calls/indexing: `f()()`, `a[0][1]`) — rather than a
    /// grouping expression (`1 + (2 * 3)`) or an array literal (`weka a = [1, 2, 3]`), which
    /// follow an operator or keyword and keep the preceding space.
    fn is_call_or_index_open(&self, lex: &str, idx: usize) -> bool {
        if lex != "(" && lex != "[" {
            return false;
        }
        let Some(prev_idx) = idx.checked_sub(1) else { return false };
        let Some(prev) = self.tokens.get(prev_idx) else { return false };
        let prev_lex = prev.lexeme.as_str();
        if prev_lex == ")" || prev_lex == "]" {
            return true;
        }
        let is_identifier_like = prev_lex
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_');
        if !is_identifier_like {
            return false;
        }
        // A method name right after `.` is always call syntax, even if the same word is also a
        // statement keyword elsewhere (e.g. `.weka(...)` the method vs. `weka x = ...`).
        let is_method_name = prev_idx
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .is_some_and(|before| before.lexeme == ".");
        is_method_name || !KEYWORDS.contains(&prev_lex)
    }

    fn trim_trailing_space(&mut self) {
        while self.out.ends_with(' ') {
            self.out.pop();
        }
    }

    fn newline_indent(&mut self) {
        self.trim_trailing_space();
        if !self.out.is_empty() {
            self.out.push('\n');
        }
        for _ in 0..self.depth {
            self.out.push_str("    ");
        }
    }

    fn emit_comments_after(&mut self, anchor: Option<usize>) {
        while self.next_comment < self.comments.len()
            && self.comments[self.next_comment].after_token_index == anchor
        {
            let c = &self.comments[self.next_comment];
            self.trim_trailing_space();
            if !self.out.is_empty() && !self.out.ends_with('\n') {
                self.out.push(' ');
            }
            self.out.push_str(&c.text);
            self.newline_indent();
            self.next_comment += 1;
        }
    }
}

/// Converts a lexer token's internal representation back into the source syntax it came from.
/// Two token shapes need this:
/// - A char literal like `'A'` is folded into one token whose lexeme is the debug form `CHAR:A`
///   (see `core/lexer/src/lib.rs`) — printing that debug form verbatim isn't valid Asili syntax
///   and doesn't round-trip (a second format pass would re-tokenize `CHAR:A` as three tokens).
/// - A string literal's lexeme is already fully *decoded* (escapes resolved to raw characters,
///   e.g. the two source characters `\` `n` become one raw `'\n'` byte in the lexeme) — printing
///   a raw newline/tab/quote/backslash verbatim would corrupt the output (a raw newline inside
///   what must stay a single-line string literal) or fail to round-trip, so these get re-escaped
///   on the way out.
/// Every other lexeme already prints as-is.
fn render_lexeme(lexeme: &str) -> std::borrow::Cow<'_, str> {
    if let Some(rest) = lexeme.strip_prefix("CHAR:") {
        let ch = rest.chars().next().unwrap_or('\0');
        return std::borrow::Cow::Owned(match ch {
            '\n' => "'\\n'".to_string(),
            '\t' => "'\\t'".to_string(),
            '\'' => "'\\''".to_string(),
            '\\' => "'\\\\'".to_string(),
            other => format!("'{other}'"),
        });
    }
    if lexeme.starts_with('"') && lexeme.ends_with('"') && lexeme.len() >= 2 {
        let inner = &lexeme[1..lexeme.len() - 1];
        if inner.contains(['\n', '\t', '"', '\\']) {
            let mut out = String::with_capacity(lexeme.len() + 4);
            out.push('"');
            for c in inner.chars() {
                match c {
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    other => out.push(other),
                }
            }
            out.push('"');
            return std::borrow::Cow::Owned(out);
        }
    }
    std::borrow::Cow::Borrowed(lexeme)
}

/// Finds `<...>` regions that are generic-type brackets (`Orodha<Neno>`, `Kamusi<Neno, Namba>`)
/// rather than the less-than/greater-than comparison operators sharing the same lexemes — this
/// lexer has no token-kind distinction between the two, and a token-stream printer has no type
/// grammar to consult, so this is a heuristic: a `<` immediately after a capitalized identifier
/// (this language's type-name convention — `Namba`, `Neno`, `Orodha`, ...) with no space opens a
/// candidate region; it's confirmed as generic brackets only if a matching `>` (respecting `<`
/// nesting, for `Kamusi<Neno, Orodha<Namba>>`) is found before a token that could only appear in
/// an expression, never a type (`(`, a numeric literal, a string, or a statement-ending `;`/
/// newline-implied boundary via `{`). This keeps `a < b` (comparison) from being misclassified
/// even though `a` might itself be a capitalized identifier.
fn classify_generic_brackets(tokens: &[Token]) -> std::collections::HashSet<usize> {
    let mut result = std::collections::HashSet::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].lexeme == "<" && i > 0 {
            let prev = &tokens[i - 1].lexeme;
            let is_type_name = prev.chars().next().is_some_and(|c| c.is_uppercase());
            if is_type_name {
                if let Some(close) = find_matching_generic_close(tokens, i) {
                    // Every `<`/`>` strictly between open and close is a nested generic bracket
                    // by construction — `find_matching_generic_close` only accepts identifiers,
                    // `,`, and balanced `<`/`>` inside the region, so mark the whole span.
                    for (k, t) in tokens.iter().enumerate().take(close + 1).skip(i) {
                        if t.lexeme == "<" || t.lexeme == ">" {
                            result.insert(k);
                        }
                    }
                    i = close + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    result
}

/// From an opening `<` at `open_idx`, scans forward tracking `<`/`>` nesting depth for the
/// matching `>`. Every token strictly between the brackets must be an identifier-like lexeme or
/// `,` (a type argument list: `Neno`, `Orodha<Namba>`, `Neno, Namba`) — anything else (`(`, a
/// numeric/string literal, `;`, `{`, `=`, `?`, `->`, ...) means this was never a generic bracket
/// to begin with (e.g. `a < b` or `a < b > c`, chained comparisons), so the caller treats `<` as
/// the comparison operator instead.
fn find_matching_generic_close(tokens: &[Token], open_idx: usize) -> Option<usize> {
    let mut depth = 1i32;
    let mut j = open_idx + 1;
    while j < tokens.len() {
        let lex = tokens[j].lexeme.as_str();
        match lex {
            "<" => depth += 1,
            ">" => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            "," => {}
            _ => {
                let is_identifier_like =
                    lex.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_');
                if !is_identifier_like {
                    return None;
                }
            }
        }
        // A generic argument list realistically never spans more than a couple of source lines
        // (it's a type annotation, not a block); bail if it does, treating this as unmatched
        // rather than risk misclassifying a large stretch of unrelated operators as brackets.
        if tokens[j].line > tokens[open_idx].line + 3 {
            return None;
        }
        j += 1;
    }
    None
}

fn collapse_blank_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut blank_run = 0usize;
    for line in s.split_inclusive('\n') {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
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
        assert!(output.contains("{\n"));
        assert!(output.contains("chapisha(\"x\")"));
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

    #[test]
    fn string_and_char_literals_survive_unchanged() {
        // The exact bug class the previous String::replace-based implementation had: braces,
        // commas inside a string literal must not be touched by layout rules.
        let input = r#"weka s = "a, {b} c"
weka c = 'x'"#;
        let output = canonical_format(input);
        assert!(output.contains(r#""a, {b} c""#), "string literal must survive verbatim: {output}");
    }

    #[test]
    fn comments_are_preserved_not_deleted() {
        let input = "weka x = 1 # muhimu\nchapisha(x)\n";
        let output = canonical_format(input);
        assert!(output.contains("# muhimu"), "comment must be preserved, got: {output}");
        assert!(output.contains("chapisha(x)"));
    }

    #[test]
    fn leading_file_comment_preserved() {
        let input = "# maelezo ya faili\nkazi kuu() -> Tupu { rejesha Tupu }\n";
        let output = canonical_format(input);
        assert!(output.starts_with("# maelezo ya faili"));
    }

    #[test]
    fn slash_comment_preserved() {
        let input = "kazi foo() -> Tupu {\n    // maelezo\n    rejesha Tupu\n}\n";
        let output = canonical_format(input);
        assert!(output.contains("// maelezo"));
    }

    #[test]
    fn invalid_source_falls_back_unchanged() {
        let input = "weka s = \"haijafungwa";
        let output = canonical_format(input);
        assert_eq!(output, input);
    }

    #[test]
    fn call_and_index_parens_hug_their_target() {
        let input = "weka a = orodha(10, 20, 30)\nrejesha a[0]?\n";
        let output = canonical_format(input);
        assert!(output.contains("orodha(10, 20, 30)"));
        assert!(output.contains("a[0]?"), "indexing must hug, got: {output}");
    }

    #[test]
    fn grouping_paren_after_keyword_keeps_space() {
        let input = "kazi f() -> Namba {\n  rejesha (v kama Namba)\n}\n";
        let output = canonical_format(input);
        assert!(output.contains("rejesha (v kama Namba)"), "got: {output}");
    }

    #[test]
    fn grouping_paren_after_operator_keeps_space() {
        let input = "chapisha(\"x = \" + (j kama Neno))\n";
        let output = canonical_format(input);
        assert!(output.contains("+ (j kama Neno)"), "got: {output}");
    }

    #[test]
    fn array_literal_after_assignment_keeps_space() {
        let input = "weka a = [1, 2, 3]\n";
        let output = canonical_format(input);
        assert!(output.contains("= [1, 2, 3]"), "got: {output}");
    }

    #[test]
    fn blank_line_between_top_level_items_preserved() {
        let input = "thabiti X: Namba = 1\n\numbo Pika {\n    x: Namba\n}\n";
        let output = canonical_format(input);
        assert!(
            output.contains("thabiti X: Namba = 1\n\numbo Pika"),
            "single blank line between top-level items must survive, got: {output}"
        );
    }

    #[test]
    fn char_literal_round_trips_and_is_idempotent() {
        // The lexer folds 'A' into a single internal `CHAR:A` token — printing that debug form
        // verbatim isn't valid syntax and would re-tokenize as three tokens on a second pass.
        let input = "m.ingiza('A', \"Herufi\")\nrejesha m.pata('A')\n";
        let once = canonical_format(input);
        assert!(once.contains("'A'"), "char literal must render as 'A', got: {once}");
        assert!(!once.contains("CHAR:"), "internal lexeme must not leak into output: {once}");
        let twice = canonical_format(&once);
        assert_eq!(once, twice, "formatting a char literal must be idempotent");
    }

    #[test]
    fn string_escape_sequences_round_trip_and_are_idempotent() {
        // The lexer stores a string's DECODED content in the token lexeme (a real newline byte,
        // not the two characters `\`+`n`) — printing that raw byte verbatim would corrupt a
        // supposedly single-line string and fail to re-tokenize (LEX001 unterminated string).
        let input = r#"chapisha("Habari\nulimwengu\t\"ndani\"\\nje")"#;
        let once = canonical_format(input);
        assert!(once.contains(r#""Habari\nulimwengu\t\"ndani\"\\nje""#), "got: {once}");
        assert!(!once.contains('\n') || once.matches('\n').count() == 1, "must stay one line, got: {once:?}");
        let twice = canonical_format(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn escaped_char_literals_round_trip() {
        for (src, expect) in [(r"'\n'", r"'\n'"), (r"'\''", r"'\''"), (r"'\\'", r"'\\'")] {
            let input = format!("weka c = {src}\n");
            let output = canonical_format(&input);
            assert!(output.contains(expect), "expected {expect} in output, got: {output}");
        }
    }

    #[test]
    fn nested_blocks_indent_by_depth() {
        let input = "kazi foo() -> Tupu {\nkama kweli {\nchapisha(\"x\")\n}\n}\n";
        let output = canonical_format(input);
        assert!(output.contains("    kama kweli {\n        chapisha(\"x\")\n    }\n"), "got: {output}");
    }

    #[test]
    fn generic_type_brackets_hug_no_spaces() {
        let input = "kazi kuu(hoja: Orodha<Neno>) -> Tupu {\n    rejesha Tupu\n}\n";
        let output = canonical_format(input);
        assert!(output.contains("Orodha<Neno>"), "got: {output}");
        assert!(!output.contains("Orodha <"), "got: {output}");
    }

    #[test]
    fn nested_generic_brackets_hug() {
        let input = "weka m: Kamusi<Neno, Orodha<Namba>> = kamusi()\n";
        let output = canonical_format(input);
        assert!(output.contains("Kamusi<Neno, Orodha<Namba>>"), "got: {output}");
    }

    #[test]
    fn comparison_operator_not_confused_with_generic_bracket() {
        let input = "kama Kiwango > x {\n    chapisha(\"kubwa\")\n}\n";
        let output = canonical_format(input);
        assert!(output.contains("Kiwango > x"), "got: {output}");
    }

    #[test]
    fn method_call_named_like_a_keyword_hugs_its_parens() {
        // `weka` is also a statement keyword (`weka x = ...`), but `.weka(...)` here is a
        // method name on Kasha_GC and must still hug like any other call.
        let input = "b.weka(42.0)\n";
        let output = canonical_format(input);
        assert!(output.contains("b.weka(42.0)"), "got: {output}");
    }

    #[test]
    fn no_trailing_whitespace_on_any_line() {
        let input = "kazi kuu(hoja: Orodha<Neno>) -> Tupu {\n  weka a = 1\n  chapisha(a)\n}\n";
        let output = canonical_format(input);
        for line in output.lines() {
            assert!(!line.ends_with(' '), "trailing whitespace on line: {line:?}");
        }
    }

    #[test]
    fn full_example_is_idempotent_and_clean() {
        let input = "leta kasha_gc\nleta matumizi\n\nkazi kuu(hoja: Orodha<Neno>) -> Tupu {\n  weka a = kasha_gc_unda(0.0)\n  weka b = a.shirikisha()\n\n  b.weka(42.0)\n  chapisha(\"a.pata() baada ya b.weka(42.0): \" + (a.pata() kama Neno))\n}\n";
        let once = canonical_format(input);
        let twice = canonical_format(&once);
        assert_eq!(once, twice);
        assert!(once.contains("Orodha<Neno>"));
        assert!(once.contains("b.weka(42.0)"));
        for line in once.lines() {
            assert!(!line.ends_with(' '), "trailing whitespace on line: {line:?}");
        }
    }
}
