use asili_diagnostics::Diagnostic;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

/// A comment captured as trivia rather than a token — see [`tokenize_with_trivia`]. `text`
/// excludes the leading `#`/`//` marker; `after_token_index` is the index into the returned
/// token vec of the last token before this comment (`None` if the comment precedes every token),
/// letting a consumer re-attach each comment to "immediately after token N" during re-emission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    pub text: String,
    pub line: usize,
    pub column: usize,
    pub after_token_index: Option<usize>,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    tokenize_inner(source, None)
}

/// Same tokenization as [`tokenize`], but comments (`# ...` / `// ...`) are captured as
/// [`Comment`] trivia instead of being silently discarded — for `pata nadhifu`, which needs to
/// re-emit them rather than delete them. Every other consumer (the parser, LSP, lint, tests)
/// keeps using [`tokenize`] unchanged; this is purely additive.
pub fn tokenize_with_trivia(source: &str) -> Result<(Vec<Token>, Vec<Comment>), Vec<Diagnostic>> {
    let mut comments = Vec::new();
    let tokens = tokenize_inner(source, Some(&mut comments))?;
    Ok((tokens, comments))
}

fn tokenize_inner(
    source: &str,
    mut trivia: Option<&mut Vec<Comment>>,
) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let mut col = 1usize;
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            let ch = chars[i];
            if ch.is_whitespace() {
                i += 1;
                col += 1;
                continue;
            }

            if ch == '#' {
                if i + 1 < chars.len() && chars[i + 1] == '[' {
                    // Attribute start, treat '#' as a token
                } else {
                    // Comment, skip until end of line
                    let start_col = col;
                    let start = i;
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                        col += 1;
                    }
                    if let Some(out) = trivia.as_deref_mut() {
                        out.push(Comment {
                            text: chars[start..i].iter().collect(),
                            line: line_idx + 1,
                            column: start_col,
                            after_token_index: tokens.len().checked_sub(1),
                        });
                    }
                    continue;
                }
            }

            if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                // Comment, skip until end of line
                let start_col = col;
                let start = i;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                    col += 1;
                }
                if let Some(out) = trivia.as_deref_mut() {
                    out.push(Comment {
                        text: chars[start..i].iter().collect(),
                        line: line_idx + 1,
                        column: start_col,
                        after_token_index: tokens.len().checked_sub(1),
                    });
                }
                continue;
            }

            if ch == '\'' {
                let start_col = col;
                let mut decoded = None;
                let mut is_char_literal = false;
                if i + 1 < chars.len() {
                    let c = chars[i + 1];
                    if c == '\\' && i + 3 < chars.len() && chars[i + 3] == '\'' {
                        let escaped = chars[i + 2];
                        is_char_literal = true;
                        decoded = Some(match escaped {
                            'n' => '\n',
                            't' => '\t',
                            '\'' => '\'',
                            '\\' => '\\',
                            _ => escaped,
                        });
                    } else if i + 2 < chars.len() && chars[i + 2] == '\'' && c != '\'' {
                        is_char_literal = true;
                        decoded = Some(c);
                    }
                }
                if is_char_literal {
                    i += 1;
                    col += 1;
                    if decoded == Some('\\') || decoded == Some('\'') {
                        i += 3;
                        col += 3;
                    } else {
                        i += 2;
                        col += 2;
                    }
                    if let Some(ch) = decoded {
                        tokens.push(Token {
                            lexeme: format!("CHAR:{}", ch),
                            line: line_idx + 1,
                            column: start_col,
                        });
                    }
                }
                if is_char_literal {
                    continue;
                }
            }

            if ch == '"' {
                let start_col = col;
                i += 1;
                col += 1;
                let mut s = String::from('"');
                let mut closed = false;
                while i < chars.len() {
                    let c = chars[i];
                    if c == '\\' && i + 1 < chars.len() {
                        i += 1;
                        col += 1;
                        let escaped = chars[i];
                        i += 1;
                        col += 1;
                        let decoded = match escaped {
                            'n' => '\n',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            _ => {
                                s.push('\\');
                                escaped
                            }
                        };
                        s.push(decoded);
                        continue;
                    }
                    s.push(c);
                    i += 1;
                    col += 1;
                    if c == '"' {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    errors.push(
                        Diagnostic::new("LEX001", "Kamba haijafungwa")
                            .with_span(line_idx + 1, start_col),
                    );
                } else {
                    tokens.push(Token {
                        lexeme: s,
                        line: line_idx + 1,
                        column: start_col,
                    });
                }
                continue;
            }

            if "(){}:,.;+-*/%<>!=[]?#&|".contains(ch) {
                let start_col = col;
                let mut lexeme = ch.to_string();
                if i + 1 < chars.len() {
                    let pair = format!("{}{}", ch, chars[i + 1]);
                    if ["==", "!=", ">=", "<=", "->", "+=", "-=", "*=", "/=", "=>", "::", "**", "&&", "||"].contains(&pair.as_str()) {
                        lexeme = pair;
                        i += 1;
                        col += 1;
                    }
                }
                tokens.push(Token {
                    lexeme,
                    line: line_idx + 1,
                    column: start_col,
                });
                i += 1;
                col += 1;
                continue;
            }

            let start_col = col;
            let mut word = String::new();
            while i < chars.len() {
                let c = chars[i];
                // Include decimal point in numeric literals: if building a digit-only token
                // and we see '.' followed by a digit, absorb both to form e.g. "100.0".
                if c == '.' && word.chars().all(|ch| ch.is_ascii_digit()) && !word.is_empty()
                    && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()
                {
                    word.push('.');
                    i += 1;
                    col += 1;
                    continue;
                }
                if c.is_whitespace() || "(){}:,.;+-*/%<>!=[]#&?|\"".contains(c) {
                    break;
                }
                word.push(c);
                i += 1;
                col += 1;
            }
            if !word.is_empty() {
                tokens.push(Token {
                    lexeme: word,
                    line: line_idx + 1,
                    column: start_col,
                });
            } else {
                i += 1;
                col += 1;
            }
        }
    }

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::{tokenize, tokenize_with_trivia};

    #[test]
    fn tokenizes_basic_line() {
        let src = "weka x = 1 + 2";
        let t = tokenize(src).expect("tokenize");
        assert!(!t.is_empty());
    }

    #[test]
    fn string_escapes_decoded() {
        let src = r#"weka s = "a\n\t\"\\b""#;
        let t = tokenize(src).expect("tokenize");
        let s_tok = t.iter().find(|x| x.lexeme.starts_with('"')).expect("string token");
        assert!(s_tok.lexeme.contains('\n'));
        assert!(s_tok.lexeme.contains('\t'));
        assert!(s_tok.lexeme.ends_with('"'));
        assert_eq!(s_tok.lexeme.len(), 8); // " a \n \t " \ b "
    }

    #[test]
    fn trailing_question_mark_is_separate_token() {
        let src = "ni_namba?(3)";
        let t = tokenize(src).expect("tokenize");
        let lexemes: Vec<&str> = t.iter().map(|x| x.lexeme.as_str()).collect();
        assert_eq!(lexemes, ["ni_namba", "?", "(", "3", ")"], "ni_namba? should be two tokens");
    }

    #[test]
    fn short_ident_then_propagate_two_tokens() {
        let src = "r?";
        let t = tokenize(src).expect("tokenize");
        let lexemes: Vec<&str> = t.iter().map(|x| x.lexeme.as_str()).collect();
        assert_eq!(lexemes, ["r", "?"], "r? should be two tokens for propagate");
    }

    #[test]
    fn tokenize_unaffected_by_trivia_capture() {
        let src = "weka x = 1 # maoni\nweka y = 2 // maoni mengine\n";
        let plain = tokenize(src).expect("tokenize");
        let (with_trivia, _) = tokenize_with_trivia(src).expect("tokenize_with_trivia");
        assert_eq!(plain, with_trivia, "trivia capture must not change the token stream");
    }

    #[test]
    fn tokenize_with_trivia_captures_hash_and_slash_comments() {
        let src = "weka x = 1 # ya kwanza\n// mstari mzima\nweka y = 2";
        let (tokens, comments) = tokenize_with_trivia(src).expect("tokenize_with_trivia");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, "# ya kwanza");
        assert_eq!(comments[0].line, 1);
        assert_eq!(comments[1].text, "// mstari mzima");
        assert_eq!(comments[1].line, 2);
        // First comment follows the last token on line 1 ("1"); second comment precedes any
        // token on its own line, so it should attach to that same prior token, not None.
        let tok_1_idx = tokens.iter().position(|t| t.lexeme == "1").unwrap();
        assert_eq!(comments[0].after_token_index, Some(tok_1_idx));
        assert_eq!(comments[1].after_token_index, Some(tok_1_idx));
    }

    #[test]
    fn tokenize_with_trivia_leading_comment_has_no_prior_token() {
        let src = "# maelezo ya faili\nweka x = 1";
        let (_, comments) = tokenize_with_trivia(src).expect("tokenize_with_trivia");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].after_token_index, None);
    }

    #[test]
    fn attribute_hash_still_a_token_not_a_comment() {
        let src = "#[jaribio]\nkazi t() -> Tupu { rejesha Tupu }";
        let (tokens, comments) = tokenize_with_trivia(src).expect("tokenize_with_trivia");
        assert!(comments.is_empty(), "#[...] is an attribute, not a comment");
        assert_eq!(tokens[0].lexeme, "#");
        assert_eq!(tokens[1].lexeme, "[");
    }
}
