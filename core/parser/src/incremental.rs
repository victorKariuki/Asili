//! Incremental parsing for editors: a document is cut into its top-level items, and an edit
//! re-lexes and re-parses only the items whose text changed.
//!
//! An item starts on a line that begins (column 1) with a top-level keyword — `kazi`, `umma`,
//! `leta`, `umbo`, `jenum`, `sifa`, `shughuli`, `thabiti` — together with any `#[...]` attribute
//! lines right above it: the same boundaries the parser's error recovery stops at. Each item's
//! tokens and syntax tree are cached by its text (and the line it starts on), and the module is
//! the items' trees appended in order ([`Module::append`]).
//!
//! The result is always exactly what [`crate::parse_tokens`] gives for the whole source. Cutting
//! is only trusted when it provably cannot change the parse: every piece lexes, its brackets
//! balance, it parses with no error, and it leaves no attribute waiting for an item. Then the
//! parser, which carries nothing else from one item to the next, sees the same items at the same
//! places. Otherwise — a half-typed item, a `thabiti` at column 1 inside a body — the whole
//! source is lexed and parsed as usual.

use asili_diagnostics::Diagnostic;
use asili_lexer::{Token, TokenKind};

use crate::cursor::Parser;
use crate::{FxHashMap, Module};

/// One item's text: its tokens with lines counted from where it started last time (`line`), and
/// its tree once parsed there.
struct Piece {
    tokens: Vec<Token>,
    line: usize,
    module: Option<Module>,
}

/// A document's parse, kept between edits. See the module documentation.
#[derive(Default)]
pub struct IncrementalParser {
    pieces: FxHashMap<String, Piece>,
}

impl IncrementalParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// `source`'s syntax tree, or its lexer or parser errors — the same result as
    /// `tokenize` then [`crate::parse_tokens`], reusing every unchanged item of the last call.
    pub fn parse(&mut self, source: &str) -> Result<Module, Vec<Diagnostic>> {
        match self.parse_pieces(source) {
            Some(module) => Ok(module),
            None => full_parse(source),
        }
    }

    fn parse_pieces(&mut self, source: &str) -> Option<Module> {
        let mut previous = std::mem::take(&mut self.pieces);
        let mut module = Module::default();
        let parser = Parser::new(&[]);
        for (start_line, text) in pieces(source) {
            let piece = match previous.remove(text) {
                Some(piece) => piece,
                None => {
                    let tokens = asili_lexer::tokenize(text).ok()?;
                    if !balanced(&tokens) {
                        return None;
                    }
                    Piece {
                        tokens,
                        line: 1,
                        module: None,
                    }
                }
            };
            let piece = self.pieces.entry(text.to_string()).or_insert(piece);
            if piece.line != start_line {
                // The item moved: renumber its tokens in place and parse it again.
                for t in &mut piece.tokens {
                    t.line = t.line + start_line - piece.line;
                }
                piece.line = start_line;
                piece.module = None;
            }
            if piece.module.is_none() {
                let mut p = Parser::new(&piece.tokens);
                let (item, dangling) = p.parse_items();
                if dangling || !p.errors.is_empty() {
                    return None;
                }
                piece.module = Some(item);
            }
            module.append(piece.module.as_ref().expect("parsed above"));
        }
        module.enums.extend(parser.standard_enums());
        module.traits.extend(parser.standard_traits());
        Some(module)
    }
}

fn full_parse(source: &str) -> Result<Module, Vec<Diagnostic>> {
    let tokens = asili_lexer::tokenize(source)?;
    crate::parse_tokens(&tokens)
}

/// Every `(`, `[` and `{` closed, in order of nesting depth never below zero.
fn balanced(tokens: &[Token]) -> bool {
    let mut depth = [0i32; 3];
    for t in tokens {
        let (i, d) = match t.kind {
            TokenKind::LParen => (0, 1),
            TokenKind::RParen => (0, -1),
            TokenKind::LBracket => (1, 1),
            TokenKind::RBracket => (1, -1),
            TokenKind::LBrace => (2, 1),
            TokenKind::RBrace => (2, -1),
            _ => continue,
        };
        depth[i] += d;
        if depth[i] < 0 {
            return false;
        }
    }
    depth == [0; 3]
}

/// `source` cut before each top-level item: `(first line, text)`, covering all of it.
fn pieces(source: &str) -> Vec<(usize, &str)> {
    const ITEMS: [&str; 8] = [
        "kazi", "umma", "leta", "umbo", "jenum", "sifa", "shughuli", "thabiti",
    ];
    let starts_item = |line: &str| {
        ITEMS.iter().any(|k| {
            line.strip_prefix(k)
                .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
        })
    };
    // Byte offset and text of every line.
    let mut lines: Vec<(usize, &str)> = Vec::new();
    let mut at = 0;
    for line in source.split_inclusive('\n') {
        lines.push((at, line));
        at += line.len();
    }
    let mut cuts = vec![0usize]; // line indices
    for (i, (_, line)) in lines.iter().enumerate().skip(1) {
        if !starts_item(line) {
            continue;
        }
        // Attribute lines right above (blank and comment lines between them allowed) belong to
        // the item.
        let mut cut = i;
        let last = *cuts.last().expect("starts with 0");
        for j in (if last == 0 { 0 } else { last + 1 }..i).rev() {
            let l = lines[j].1.trim_start();
            if l.starts_with("#[") {
                cut = j;
            } else if !(l.is_empty() || l.starts_with('#')) {
                break;
            }
        }
        if cut > last {
            cuts.push(cut);
        }
    }
    cuts.iter()
        .enumerate()
        .map(|(n, &c)| {
            let begin = lines.get(c).map_or(source.len(), |l| l.0);
            let end = cuts.get(n + 1).map_or(source.len(), |&next| lines[next].0);
            (c + 1, &source[begin..end])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full(source: &str) -> Result<Module, Vec<Diagnostic>> {
        full_parse(source)
    }

    #[test]
    fn cuts_before_items_and_their_attributes() {
        let src = "leta hisabati\n\nkazi a() -> Tupu {\n}\n#[jaribio]\n\nkazi b() -> Tupu {}\n";
        let cut: Vec<(usize, &str)> = pieces(src);
        assert_eq!(
            cut,
            vec![
                (1, "leta hisabati\n\n"),
                (3, "kazi a() -> Tupu {\n}\n"),
                (5, "#[jaribio]\n\nkazi b() -> Tupu {}\n"),
            ]
        );
    }

    #[test]
    fn same_as_a_full_parse_across_edits() {
        let mut inc = IncrementalParser::new();
        let edits = [
            "thabiti N: Namba = 9\n\nkazi kuu() -> Tupu {\n    weka x = N * 2\n    chapisha(x)\n}\n",
            // An edit inside one item.
            "thabiti N: Namba = 9\n\nkazi kuu() -> Tupu {\n    weka x = N * 3\n    chapisha(x)\n}\n",
            // A line inserted above: the item below moves down.
            "thabiti N: Namba = 9\n\n\nkazi kuu() -> Tupu {\n    weka x = N * 3\n    chapisha(x)\n}\n",
            // Half-typed: falls back, with the full parse's errors.
            "thabiti N: Namba = 9\n\nkazi kuu() -> Tupu {\n    weka x = N *\n",
            // `thabiti` at column 1 inside a body is not an item boundary.
            "kazi kuu() -> Tupu {\nthabiti y = 2\n    chapisha(y)\n}\n",
            // An attribute above an item stays with it.
            "#[jaribio]\nkazi t() -> Tupu {\n}\nkazi kuu() -> Tupu {\n}\n",
        ];
        for src in edits {
            assert_eq!(inc.parse(src), full(src), "{src}");
        }
    }
}
