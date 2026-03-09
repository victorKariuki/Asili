//! Token cursor: stream position, peek, advance, consume, and error helpers.

use asili_diagnostics::Diagnostic;
use asili_lexer::Token;

pub(crate) struct Parser<'a> {
    pub(crate) tokens: &'a [Token],
    pub(crate) pos: usize,
    pub(crate) errors: Vec<Diagnostic>,
    pub(crate) depth: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
            depth: 0,
        }
    }

    pub(crate) fn consume_ident(&mut self, code: &'static str, msg: &str) -> Option<Token> {
        if self.check_ident() {
            return Some(self.advance().clone());
        }
        self.err_here(code, msg);
        None
    }

    /// Parse optional label after vunja/endelea: `'` ident or ident starting with `'`.
    pub(crate) fn parse_optional_label(&mut self, err_code: &'static str, err_msg: &str) -> Option<String> {
        if self.match_tok("'") {
            self.consume_ident(err_code, err_msg).map(|t| t.lexeme)
        } else if !self.is_eof() && self.peek().lexeme.starts_with('\'') {
            let t = self.advance();
            Some(t.lexeme.trim_start_matches('\'').to_string())
        } else {
            None
        }
    }

    pub(crate) fn consume(&mut self, want: &str, code: &'static str, msg: &str) -> Option<Token> {
        if self.match_tok(want) {
            return Some(self.prev().clone());
        }
        self.err_here(code, msg);
        None
    }

    #[allow(dead_code)]
    pub(crate) fn match_any(&mut self, toks: &[&str]) -> bool {
        for t in toks {
            if self.match_tok(t) {
                return true;
            }
        }
        false
    }

    pub(crate) fn match_tok(&mut self, tok: &str) -> bool {
        if self.check(tok) {
            self.pos += 1;
            return true;
        }
        false
    }

    pub(crate) fn check(&self, tok: &str) -> bool {
        !self.is_eof() && self.peek().lexeme == tok
    }

    pub(crate) fn check_n(&self, n: usize, tok: &str) -> bool {
        self.peek_n(n).map(|t| t.lexeme.as_str() == tok).unwrap_or(false)
    }

    pub(crate) fn check_ident(&self) -> bool {
        if self.is_eof() {
            return false;
        }
        let l = self.peek().lexeme.as_str();
        !(
            ["(", ")", "{", "}", ",", ":", ";", "=", "+", "-", "*", "/", "%", "==", "!=", ">", "<", ">=", "<=", "?", "->", "=>", "::", "**", "#", "[", "]", "&"].contains(&l)
                || l.starts_with('"')
        )
    }

    pub(crate) fn err_here(&mut self, code: &'static str, msg: &str) {
        let mut d = Diagnostic::new(code, msg).with_stage("parse");
        if !self.is_eof() {
            d = d.with_span(self.peek().line, self.peek().column);
        }
        self.errors.push(d);
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    pub(crate) fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t
    }

    pub(crate) fn prev(&self) -> &Token {
        &self.tokens[self.pos - 1]
    }

    pub(crate) fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    pub(crate) fn peek_n(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.pos + n)
    }
}
