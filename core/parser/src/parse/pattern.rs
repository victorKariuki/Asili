//! The flat pattern machine (`linganisha` arms, `weka (a, b) = ...`): no recursion. Finished
//! sub-patterns wait on an operand stack and open brackets on a frame stack, as in the
//! expression machine.

use super::strip_string_lexeme_quotes;
use crate::cursor::Parser;
use crate::{Expr, Pattern};
use asili_lexer::{tk, TokenKind};

enum Open {
    /// `(p` — becomes a pair at `,`.
    Paren,
    /// `(p, q`.
    Pair,
    /// `Jenum::Kigezo(p`.
    EnumData {
        enum_name: String,
        variant_name: String,
        variant_line: usize,
        variant_column: usize,
    },
    /// `Umbo { uga: p, ...`.
    Struct {
        name: String,
        line: usize,
        column: usize,
        fields: Vec<String>,
    },
}

impl<'a> Parser<'a> {
    /// Parse one pattern. Never recurses.
    pub(crate) fn parse_pattern(&mut self) -> Option<Pattern> {
        let mut done: Vec<Pattern> = Vec::new();
        let mut open: Vec<(Open, usize)> = Vec::new();
        loop {
            // Start a pattern: a leaf, or an opening bracket (then start its first pattern).
            if let Some(bracket) = self.pattern_start(&mut done)? {
                open.push((bracket, done.len()));
                continue;
            }
            // A pattern ended: close brackets until one wants another sub-pattern.
            loop {
                let Some((bracket, base)) = open.last_mut() else {
                    return done.pop();
                };
                match bracket {
                    Open::Paren if self.match_tok(tk!(",")) => {
                        *bracket = Open::Pair;
                        break;
                    }
                    Open::Paren => {
                        self.consume(tk!(")"), "PAR053", "muundo unahitaji ')'")?;
                        open.pop();
                    }
                    Open::Pair => {
                        self.consume(tk!(")"), "PAR053", "muundo wa jozi unahitaji ')'")?;
                        let second = done.pop()?;
                        let first = done.pop()?;
                        done.push(Pattern::Jozi(Box::new(first), Box::new(second)));
                        open.pop();
                    }
                    Open::EnumData { .. } => {
                        self.consume(tk!(")"), "PAR086", "jenum pattern inahitaji ')'")?;
                        let data = done.pop()?;
                        let Some((
                            Open::EnumData {
                                enum_name,
                                variant_name,
                                variant_line,
                                variant_column,
                            },
                            _,
                        )) = open.pop()
                        else {
                            unreachable!("matched above");
                        };
                        done.push(Pattern::Enum {
                            enum_name,
                            variant_name,
                            data: Some(Box::new(data)),
                            variant_line,
                            variant_column,
                        });
                    }
                    Open::Struct { fields, .. } => {
                        let more = self.match_tok(tk!(",")) && !self.check(tk!("}"));
                        if more {
                            fields.push(self.pattern_field()?);
                            break;
                        }
                        self.consume(tk!("}"), "PAR053", "umbo pattern inahitaji '}'")?;
                        let base = *base;
                        let Some((
                            Open::Struct {
                                name,
                                line,
                                column,
                                fields,
                            },
                            _,
                        )) = open.pop()
                        else {
                            unreachable!("matched above");
                        };
                        let subs = done.split_off(base);
                        done.push(Pattern::Struct {
                            struct_name: name,
                            line,
                            column,
                            fields: fields.into_iter().zip(subs).collect(),
                        });
                    }
                }
            }
        }
    }

    /// `uga :` inside a struct pattern.
    fn pattern_field(&mut self) -> Option<String> {
        let name = self.consume_ident("PAR053", "umbo pattern inahitaji jina la uga")?;
        self.consume(tk!(":"), "PAR053", "umbo pattern inahitaji ':'")?;
        Some(name.lexeme.clone())
    }

    /// A leaf pattern pushed onto `done` (`None` inside), or the bracket a pattern opens
    /// (`Some`), with its first sub-pattern still to come.
    fn pattern_start(&mut self, done: &mut Vec<Pattern>) -> Option<Option<Open>> {
        if self.match_tok(tk!("(")) {
            return Some(Some(Open::Paren));
        }
        if self.match_tok(tk!("_")) {
            done.push(Pattern::Wildcard);
            return Some(None);
        }
        let literal = self.peek_n(0).and_then(|t| match t.kind {
            TokenKind::Str => Some(Expr::String(strip_string_lexeme_quotes(&t.lexeme))),
            TokenKind::Char => Some(Expr::Char(
                t.lexeme["CHAR:".len()..].chars().next().unwrap_or('\0'),
            )),
            TokenKind::Number | TokenKind::Dot => Some(Expr::Number(t.lexeme.clone())),
            TokenKind::Hamna => Some(Expr::Hamna),
            TokenKind::KwKweli => Some(Expr::Bool(true)),
            TokenKind::KwSiKweli => Some(Expr::Bool(false)),
            _ => None,
        });
        if let Some(literal) = literal {
            self.pos += 1;
            done.push(Pattern::Literal(literal));
            return Some(None);
        }
        if self.check_ident() {
            let t = self.advance();
            let (name, line, column) = (t.lexeme.clone(), t.line, t.column);
            if self.match_tok(tk!("::")) {
                let variant =
                    self.consume_ident("PAR085", "jenum pattern inahitaji jina la kigezo")?;
                if self.match_tok(tk!("(")) {
                    return Some(Some(Open::EnumData {
                        enum_name: name,
                        variant_name: variant.lexeme.clone(),
                        variant_line: variant.line,
                        variant_column: variant.column,
                    }));
                }
                done.push(Pattern::Enum {
                    enum_name: name,
                    variant_name: variant.lexeme.clone(),
                    data: None,
                    variant_line: variant.line,
                    variant_column: variant.column,
                });
                return Some(None);
            }
            if self.match_tok(tk!("{")) {
                if self.match_tok(tk!("}")) {
                    done.push(Pattern::Struct {
                        struct_name: name,
                        line,
                        column,
                        fields: Vec::new(),
                    });
                    return Some(None);
                }
                let first = self.pattern_field()?;
                return Some(Some(Open::Struct {
                    name,
                    line,
                    column,
                    fields: vec![first],
                }));
            }
            done.push(Pattern::Ident { name, line, column });
            return Some(None);
        }
        self.err_here(
            "PAR053",
            "muundo wa linganisha haueleweki — inahitaji thamani, jina, jozi, jenum, au umbo",
        );
        None
    }
}
