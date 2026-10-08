//! Token cursor: stream position, peek, advance, consume, and error helpers.

use asili_diagnostics::Diagnostic;
use asili_lexer::{tk, Token, TokenKind};

pub(crate) struct Parser<'a> {
    pub(crate) tokens: &'a [Token],
    pub(crate) pos: usize,
    pub(crate) errors: Vec<Diagnostic>,
    pub(crate) depth: usize,
    /// The module's expression arena, filled as expressions are parsed.
    pub(crate) exprs: crate::Exprs,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
            depth: 0,
            exprs: crate::Exprs::default(),
        }
    }

    pub(crate) fn consume_ident(&mut self, code: &'static str, msg: &str) -> Option<&'a Token> {
        if self.check_ident() {
            return Some(self.advance());
        }
        self.err_here(code, msg);
        None
    }

    /// Parse optional label after vunja/endelea: `'` ident or ident starting with `'`.
    pub(crate) fn parse_optional_label(
        &mut self,
        err_code: &'static str,
        err_msg: &str,
    ) -> Option<String> {
        if self.match_tok(tk!("'")) {
            self.consume_ident(err_code, err_msg)
                .map(|t| t.lexeme.clone())
        } else if !self.is_eof() && self.peek().lexeme.starts_with('\'') {
            let t = self.advance();
            Some(t.lexeme.trim_start_matches('\'').to_string())
        } else {
            None
        }
    }

    pub(crate) fn consume(
        &mut self,
        want: TokenKind,
        code: &'static str,
        msg: &str,
    ) -> Option<&'a Token> {
        if self.match_tok(want) {
            return Some(self.prev());
        }
        self.err_here(code, msg);
        None
    }

    pub(crate) fn match_tok(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.pos += 1;
            return true;
        }
        false
    }

    #[inline]
    pub(crate) fn check(&self, kind: TokenKind) -> bool {
        self.tokens.get(self.pos).is_some_and(|t| t.kind == kind)
    }

    pub(crate) fn check_n(&self, n: usize, kind: TokenKind) -> bool {
        self.peek_n(n).is_some_and(|t| t.kind == kind)
    }

    /// The next token can stand where a name is expected: anything but a string literal and the
    /// punctuation below (keywords included — the parser rejects them later where it must).
    pub(crate) fn check_ident(&self) -> bool {
        self.tokens.get(self.pos).is_some_and(|t| {
            !matches!(
                t.kind,
                TokenKind::Str
                    | TokenKind::LParen
                    | TokenKind::RParen
                    | TokenKind::LBrace
                    | TokenKind::RBrace
                    | TokenKind::Comma
                    | TokenKind::Colon
                    | TokenKind::Semi
                    | TokenKind::Assign
                    | TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Percent
                    | TokenKind::EqEq
                    | TokenKind::NotEq
                    | TokenKind::Gt
                    | TokenKind::Lt
                    | TokenKind::Ge
                    | TokenKind::Le
                    | TokenKind::Question
                    | TokenKind::Arrow
                    | TokenKind::FatArrow
                    | TokenKind::ColonColon
                    | TokenKind::StarStar
                    | TokenKind::Shl
                    | TokenKind::Shr
                    | TokenKind::Hash
                    | TokenKind::LBracket
                    | TokenKind::RBracket
                    | TokenKind::Amp
                    | TokenKind::Pipe
                    | TokenKind::Caret
            )
        })
    }

    pub(crate) fn err_here(&mut self, code: &'static str, msg: &str) {
        let mut d = Diagnostic::new(code, msg).with_stage("uchanganuzi");
        if !self.is_eof() {
            d = d.with_span(self.peek().line, self.peek().column);
        }
        self.errors.push(d);
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    pub(crate) fn advance(&mut self) -> &'a Token {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t
    }

    pub(crate) fn prev(&self) -> &'a Token {
        &self.tokens[self.pos - 1]
    }

    pub(crate) fn peek(&self) -> &'a Token {
        &self.tokens[self.pos]
    }

    pub(crate) fn peek_n(&self, n: usize) -> Option<&'a Token> {
        self.tokens.get(self.pos + n)
    }
}
