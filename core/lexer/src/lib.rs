use asili_diagnostics::Diagnostic;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

/// What a token is, decided once by the lexer: every keyword and punctuation mark has its own
/// kind, so the parser compares one byte instead of text. Generated with the lookups below from
/// one table.
macro_rules! token_kinds {
    ($($text:literal => $name:ident,)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum TokenKind {
            /// A name (anything word-like that is not a reserved word).
            Ident,
            /// A number literal: digits and at most the dots of a decimal (`12`, `1.5`).
            Number,
            /// A string literal (the lexeme keeps its quotes).
            Str,
            /// A character literal (lexeme `CHAR:<c>`).
            Char,
            $($name,)*
        }

        impl TokenKind {
            /// The kind of a keyword or punctuation text (`None` for anything else).
            pub fn of_text(text: &str) -> Option<TokenKind> {
                match text {
                    $($text => Some(TokenKind::$name),)*
                    _ => None,
                }
            }

            /// [`TokenKind::of_text`] at compile time (see [`tk!`]): unknown text is a
            /// compile error, so a typo can never silently match every name.
            pub const fn of_text_const(text: &str) -> TokenKind {
                $(if const_eq(text, $text) { return TokenKind::$name; })*
                panic!("not a keyword or punctuation token")
            }

            /// The fixed text of a keyword or punctuation kind.
            pub fn text(self) -> Option<&'static str> {
                match self {
                    $(TokenKind::$name => Some($text),)*
                    _ => None,
                }
            }
        }
    };
}

const fn const_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

token_kinds! {
    "au" => KwAu,
    "au_biti" => KwAuBiti,
    "au_ikiwa" => KwAuIkiwa,
    "azima" => KwAzima,
    "azima_tenda" => KwAzimaTenda,
    "endelea" => KwEndelea,
    "hadi" => KwHadi,
    "ikiwa" => KwIkiwa,
    "jaribu" => KwJaribu,
    "jenum" => KwJenum,
    "kama" => KwKama,
    "katika" => KwKatika,
    "kazi" => KwKazi,
    "kutoka" => KwKutoka,
    "kwa" => KwKwa,
    "kweli" => KwKweli,
    "lebo" => KwLebo,
    "leta" => KwLeta,
    "linganisha" => KwLinganisha,
    "milele" => KwMilele,
    "na" => KwNa,
    "na_biti" => KwNaBiti,
    "rejesha" => KwRejesha,
    "shughuli" => KwShughuli,
    "si_kweli" => KwSiKweli,
    "sifa" => KwSifa,
    "siyo" => KwSiyo,
    "siyo_biti" => KwSiyoBiti,
    "sogeza_kulia" => KwSogezaKulia,
    "sogeza_kushoto" => KwSogezaKushoto,
    "thabiti" => KwThabiti,
    "tupa" => KwTupa,
    "umbo" => KwUmbo,
    "umma" => KwUmma,
    "vinginevyo" => KwVinginevyo,
    "vunja" => KwVunja,
    "wakati" => KwWakati,
    "weka" => KwWeka,
    "xor_biti" => KwXorBiti,
    "ya" => KwYa,
    "Hamna" => Hamna,
    "_" => Underscore,
    "'" => Quote,
    "(" => LParen,
    ")" => RParen,
    "{" => LBrace,
    "}" => RBrace,
    ":" => Colon,
    "," => Comma,
    "." => Dot,
    ";" => Semi,
    "+" => Plus,
    "-" => Minus,
    "*" => Star,
    "/" => Slash,
    "%" => Percent,
    "<" => Lt,
    ">" => Gt,
    "!" => Bang,
    "=" => Assign,
    "[" => LBracket,
    "]" => RBracket,
    "?" => Question,
    "#" => Hash,
    "&" => Amp,
    "|" => Pipe,
    "^" => Caret,
    "==" => EqEq,
    "!=" => NotEq,
    ">=" => Ge,
    "<=" => Le,
    "->" => Arrow,
    "+=" => PlusEq,
    "-=" => MinusEq,
    "*=" => StarEq,
    "/=" => SlashEq,
    "%=" => PercentEq,
    "&=" => AmpEq,
    "|=" => PipeEq,
    "^=" => CaretEq,
    "//" => SlashSlash,
    "=>" => FatArrow,
    "::" => ColonColon,
    "**" => StarStar,
    "&&" => AmpAmp,
    "||" => PipePipe,
    "<<" => Shl,
    ">>" => Shr,
    "//=" => SlashSlashEq,
}

impl TokenKind {
    /// Punctuation (operators, brackets, separators), as opposed to words and literals.
    pub fn is_punct(self) -> bool {
        self.text()
            .is_some_and(|t| !t.as_bytes()[0].is_ascii_alphabetic() && t != "_" && t != "'")
    }
}

/// The [`TokenKind`] of a keyword or punctuation text, resolved at compile time:
/// `tk!("weka")`, `tk!("{")`.
#[macro_export]
macro_rules! tk {
    ($text:literal) => {{
        const KIND: $crate::TokenKind = $crate::TokenKind::of_text_const($text);
        KIND
    }};
}

/// A comment captured as trivia rather than a token — see [`tokenize_with_trivia`]. `text`
/// includes the leading `#` marker; `after_token_index` is the index into the returned
/// token vec of the last token before this comment (`None` if the comment precedes every token),
/// letting a consumer re-attach each comment to "immediately after token N" during re-emission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    pub text: String,
    pub line: usize,
    pub column: usize,
    pub after_token_index: Option<usize>,
}

/// Every reserved word of the language — statement keywords, word operators and literal
/// keywords. The one list the parser's grammar, LSP completion/hover/rename and the formatter
/// all agree on.
pub const KEYWORDS: &[&str] = &[
    "au",
    "au_biti",
    "au_ikiwa",
    "azima",
    "azima_tenda",
    "endelea",
    "hadi",
    "ikiwa",
    "jaribu",
    "jenum",
    "kama",
    "katika",
    "kazi",
    "kutoka",
    "kwa",
    "kweli",
    "lebo",
    "leta",
    "linganisha",
    "milele",
    "na",
    "na_biti",
    "rejesha",
    "shughuli",
    "si_kweli",
    "sifa",
    "siyo",
    "siyo_biti",
    "sogeza_kulia",
    "sogeza_kushoto",
    "thabiti",
    "tupa",
    "umbo",
    "umma",
    "vinginevyo",
    "vunja",
    "wakati",
    "weka",
    "xor_biti",
    "ya",
];

pub fn tokenize(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    tokenize_inner(source, None)
}

/// Same tokenization as [`tokenize`], but comments (`# ...` and `/// ...` doc comments) are captured as
/// [`Comment`] trivia instead of being silently discarded — for `pata nadhifu`, which needs to
/// re-emit them rather than delete them. Every other consumer (the parser, LSP, lint, tests)
/// keeps using [`tokenize`] unchanged; this is purely additive.
pub fn tokenize_with_trivia(source: &str) -> Result<(Vec<Token>, Vec<Comment>), Vec<Diagnostic>> {
    let mut comments = Vec::new();
    let tokens = tokenize_inner(source, Some(&mut comments))?;
    Ok((tokens, comments))
}

/// A character that starts a punctuation token.
#[inline]
fn is_punct_char(c: char) -> bool {
    matches!(
        c,
        '(' | ')'
            | '{'
            | '}'
            | ':'
            | ','
            | '.'
            | ';'
            | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '<'
            | '>'
            | '!'
            | '='
            | '['
            | ']'
            | '?'
            | '#'
            | '&'
            | '|'
            | '^'
    )
}

/// A character that ends a word (punctuation, a string's quote, or whitespace).
#[inline]
fn ends_word(c: char) -> bool {
    c.is_whitespace() || c == '"' || (is_punct_char(c) && c != '.')
}

/// The two-character operator `a` `b` starts, if any.
#[inline]
fn pair(a: char, b: char) -> Option<&'static str> {
    Some(match (a, b) {
        ('=', '=') => "==",
        ('!', '=') => "!=",
        ('>', '=') => ">=",
        ('<', '=') => "<=",
        ('-', '>') => "->",
        ('+', '=') => "+=",
        ('-', '=') => "-=",
        ('*', '=') => "*=",
        ('/', '=') => "/=",
        ('%', '=') => "%=",
        ('&', '=') => "&=",
        ('|', '=') => "|=",
        ('^', '=') => "^=",
        ('/', '/') => "//",
        ('=', '>') => "=>",
        (':', ':') => "::",
        ('*', '*') => "**",
        ('&', '&') => "&&",
        ('|', '|') => "||",
        ('<', '<') => "<<",
        ('>', '>') => ">>",
        _ => return None,
    })
}

fn tokenize_inner(
    source: &str,
    mut trivia: Option<&mut Vec<Comment>>,
) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut tokens = Vec::with_capacity(source.len() / 4);
    let mut errors = Vec::new();
    // One buffer for every line's characters (lookahead needs indexing).
    let mut chars: Vec<char> = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let line_no = line_idx + 1;
        chars.clear();
        chars.extend(line.chars());
        let mut col = 1usize;
        let mut i = 0usize;
        while i < chars.len() {
            let ch = chars[i];
            if ch.is_whitespace() {
                i += 1;
                col += 1;
                continue;
            }

            if ch == '#' && chars.get(i + 1) != Some(&'[') {
                // Comment: the rest of the line (`#[` starts an attribute instead).
                skip_comment(&chars, &mut i, &mut col, line_idx, &tokens, &mut trivia);
                continue;
            }

            // `///` is a documentation comment (read by `pata thibitisha`'s public-API docs
            // check). A plain `//` is the floor-division operator, not a comment.
            if ch == '/' && chars.get(i + 1) == Some(&'/') && chars.get(i + 2) == Some(&'/') {
                skip_comment(&chars, &mut i, &mut col, line_idx, &tokens, &mut trivia);
                continue;
            }

            if ch == '\'' {
                let start_col = col;
                let mut decoded = None;
                if i + 1 < chars.len() {
                    let c = chars[i + 1];
                    if c == '\\' && i + 3 < chars.len() && chars[i + 3] == '\'' {
                        let escaped = chars[i + 2];
                        decoded = Some(match escaped {
                            'n' => '\n',
                            't' => '\t',
                            '\'' => '\'',
                            '\\' => '\\',
                            _ => escaped,
                        });
                    } else if i + 2 < chars.len() && chars[i + 2] == '\'' && c != '\'' {
                        decoded = Some(c);
                    }
                }
                if let Some(ch) = decoded {
                    let width = if ch == '\\' || ch == '\'' { 4 } else { 3 };
                    i += width;
                    col += width;
                    tokens.push(Token {
                        kind: TokenKind::Char,
                        lexeme: format!("CHAR:{ch}"),
                        line: line_no,
                        column: start_col,
                    });
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
                            .with_span(line_no, start_col),
                    );
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Str,
                        lexeme: s,
                        line: line_no,
                        column: start_col,
                    });
                }
                continue;
            }

            if is_punct_char(ch) {
                // `//=` (floor-division assignment) is the one three-character operator.
                let text: &'static str = if ch == '/'
                    && chars.get(i + 1) == Some(&'/')
                    && chars.get(i + 2) == Some(&'=')
                {
                    "//="
                } else if let Some(two) = chars.get(i + 1).and_then(|&n| pair(ch, n)) {
                    two
                } else {
                    TokenKind::of_text(ch.encode_utf8(&mut [0; 4]))
                        .and_then(TokenKind::text)
                        .expect("every punctuation character is a token")
                };
                let width = text.len();
                tokens.push(Token {
                    kind: TokenKind::of_text(text).expect("listed punctuation"),
                    lexeme: text.to_string(),
                    line: line_no,
                    column: col,
                });
                i += width;
                col += width;
                continue;
            }

            let start_col = col;
            let mut word = String::new();
            // Whether the word so far is all digits: a `.` followed by a digit then continues a
            // decimal number (`100.0`) instead of ending the word.
            let mut digits = true;
            while i < chars.len() {
                let c = chars[i];
                if c == '.'
                    && digits
                    && !word.is_empty()
                    && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())
                {
                    word.push('.');
                    digits = false;
                    i += 1;
                    col += 1;
                    continue;
                }
                if ends_word(c) || c == '.' {
                    break;
                }
                digits &= c.is_ascii_digit();
                word.push(c);
                i += 1;
                col += 1;
            }
            if word.is_empty() {
                i += 1;
                col += 1;
                continue;
            }
            let kind = TokenKind::of_text(&word).unwrap_or_else(|| {
                if word.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    TokenKind::Number
                } else {
                    TokenKind::Ident
                }
            });
            tokens.push(Token {
                kind,
                lexeme: word,
                line: line_no,
                column: start_col,
            });
        }
    }

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

/// Consume a comment running from `chars[*i]` to the end of the line, recording it as trivia
/// when the caller asked for it.
fn skip_comment(
    chars: &[char],
    i: &mut usize,
    col: &mut usize,
    line_idx: usize,
    tokens: &[Token],
    trivia: &mut Option<&mut Vec<Comment>>,
) {
    let (start, start_col) = (*i, *col);
    *col += chars.len() - *i;
    *i = chars.len();
    if let Some(out) = trivia.as_deref_mut() {
        out.push(Comment {
            text: chars[start..].iter().collect(),
            line: line_idx + 1,
            column: start_col,
            after_token_index: tokens.len().checked_sub(1),
        });
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
        let s_tok = t
            .iter()
            .find(|x| x.lexeme.starts_with('"'))
            .expect("string token");
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
        assert_eq!(
            lexemes,
            ["ni_namba", "?", "(", "3", ")"],
            "ni_namba? should be two tokens"
        );
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
        let src = "weka x = 1 # maoni\nweka y = 2 # maoni mengine\n";
        let plain = tokenize(src).expect("tokenize");
        let (with_trivia, _) = tokenize_with_trivia(src).expect("tokenize_with_trivia");
        assert_eq!(
            plain, with_trivia,
            "trivia capture must not change the token stream"
        );
    }

    #[test]
    fn tokenize_with_trivia_captures_hash_comments() {
        let src = "weka x = 1 # ya kwanza\n# mstari mzima\nweka y = 2";
        let (tokens, comments) = tokenize_with_trivia(src).expect("tokenize_with_trivia");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, "# ya kwanza");
        assert_eq!(comments[0].line, 1);
        assert_eq!(comments[1].text, "# mstari mzima");
        assert_eq!(comments[1].line, 2);
        // First comment follows the last token on line 1 ("1"); second comment precedes any
        // token on its own line, so it should attach to that same prior token, not None.
        let tok_1_idx = tokens.iter().position(|t| t.lexeme == "1").unwrap();
        assert_eq!(comments[0].after_token_index, Some(tok_1_idx));
        assert_eq!(comments[1].after_token_index, Some(tok_1_idx));
    }

    #[test]
    fn double_slash_is_floor_division_not_a_comment() {
        let lexemes: Vec<String> = tokenize("a // b //= c /// doc comment")
            .expect("tokenize")
            .into_iter()
            .map(|t| t.lexeme)
            .collect();
        assert_eq!(lexemes, ["a", "//", "b", "//=", "c"]);
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
