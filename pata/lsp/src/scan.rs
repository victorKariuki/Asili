//! A lightweight scan of raw source for the features that must work on half-typed code the
//! lexer would reject (folding, signature help): every character outside string/char literals
//! and comments, with its position. Comment rules match `asili_lexer`: `#` (not `#[`) and `///`
//! start a comment; `//` is the floor-division operator.

/// One code character: index into the source's `char`s, 0-based line and column, the char.
pub(crate) struct CodeChar {
    pub index: usize,
    pub line: u32,
    pub col: u32,
    pub ch: char,
}

pub(crate) fn code_chars(chars: &[char]) -> Vec<CodeChar> {
    let mut out = Vec::new();
    let (mut line, mut col) = (0u32, 0u32);
    let (mut in_string, mut in_char) = (false, false);
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            // An unterminated literal shouldn't swallow the rest of the file — the lexer
            // rejects those, so treat a newline as a reset.
            line += 1;
            col = 0;
            in_string = false;
            in_char = false;
            i += 1;
            continue;
        }
        if in_string || in_char {
            if c == '\\' {
                i += 2;
                col += 2;
                continue;
            }
            if (in_string && c == '"') || (in_char && c == '\'') {
                in_string = false;
                in_char = false;
            }
            i += 1;
            col += 1;
            continue;
        }
        let comment = (c == '#' && chars.get(i + 1) != Some(&'['))
            || (c == '/' && chars.get(i + 1) == Some(&'/') && chars.get(i + 2) == Some(&'/'));
        if comment {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '\'' => in_char = true,
            _ => out.push(CodeChar {
                index: i,
                line,
                col,
                ch: c,
            }),
        }
        i += 1;
        col += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(src: &str) -> String {
        let chars: Vec<char> = src.chars().collect();
        code_chars(&chars)
            .iter()
            .map(|c| c.ch)
            .filter(|c| !c.is_whitespace())
            .collect()
    }

    #[test]
    fn skips_literals_and_comments_but_not_floor_division() {
        assert_eq!(code("a // b # c"), "a//b");
        assert_eq!(code("x /// doc {"), "x");
        assert_eq!(code("f(\"(\", '{') #[a]"), "f(,)#[a]");
    }
}
