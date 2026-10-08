//! The flat block machine: every block of a function body — `ikiwa`/`au_ikiwa`/`vinginevyo`
//! chains, `kwa` and `wakati` loops, `linganisha` arms — is parsed by one loop over an explicit
//! stack of frames, never by recursion.
//!
//! `{` pushes a frame collecting statements; `}` pops it into a [`Block`] and runs the transition
//! its frame kind names (continue an `ikiwa` chain, finish a loop, add a `linganisha` arm), which
//! hands the finished statement down to the frame below. A statement that fails to parse puts the
//! machine into the synchronize state: it skips to the next `;`, or to the `}` that closes the
//! current block (stepping over balanced `{ }` pairs), and carries on, so every independent
//! mistake is reported once.

use super::MAX_NESTING;
use crate::cursor::Parser;
use crate::{Block, Expr, ExprId, ForMode, MatchArm, Pattern, Stmt};

/// An `ikiwa` statement's chain so far.
struct IfChain {
    cond: ExprId,
    then_block: Block,
    else_if: Vec<(ExprId, Block)>,
    line: usize,
}

/// What a block's `}` completes.
enum Kind {
    /// The function body itself.
    Body,
    IfThen {
        cond: ExprId,
        line: usize,
    },
    IfElif {
        chain: IfChain,
        cond: ExprId,
    },
    IfElse {
        chain: IfChain,
    },
    While {
        label: Option<String>,
        cond: ExprId,
        line: usize,
    },
    For {
        label: Option<String>,
        var: String,
        var_column: usize,
        mode: ForMode,
        line: usize,
    },
    Arm {
        pattern: Pattern,
    },
}

enum Frame {
    /// Inside `{ ... }`: statements so far, and its trace span (closed with the frame).
    Block {
        stmts: Vec<Stmt>,
        kind: Kind,
        span: asili_trace::Span,
    },
    /// Inside `linganisha x { ... }`, between arms.
    Match {
        expr: ExprId,
        arms: Vec<MatchArm>,
        line: usize,
        span: asili_trace::Span,
    },
}

impl Kind {
    /// The construct a block belongs to, as a trace span name.
    fn name(&self) -> &'static str {
        match self {
            Kind::Body => "mwili wa kazi",
            Kind::IfThen { .. } => "ikiwa",
            Kind::IfElif { .. } => "au_ikiwa",
            Kind::IfElse { .. } => "vinginevyo",
            Kind::While { .. } => "wakati",
            Kind::For { .. } => "kwa",
            Kind::Arm { .. } => "mkono wa linganisha",
        }
    }
}

/// What the synchronize state stopped at.
enum Synced {
    /// A statement boundary inside the current block: carry on.
    Boundary,
    /// The end of input or the start of the next top-level item: the body's `}` is missing.
    Lost,
}

impl<'a> Parser<'a> {
    /// Parse a function body `{ ... }`. Never recurses: see the module documentation. `None` when
    /// the body never closes (its error is already reported).
    pub(crate) fn parse_body(&mut self) -> Option<Block> {
        self.consume("{", "PAR020", "kizuizi inahitaji '{'")?;
        let mut frames = vec![Frame::Block {
            stmts: Vec::new(),
            kind: Kind::Body,
            span: asili_trace::Span::none(),
        }];
        let outer = self.depth;
        let block = self.run_blocks(&mut frames);
        self.depth = outer;
        block
    }

    fn run_blocks(&mut self, frames: &mut Vec<Frame>) -> Option<Block> {
        loop {
            self.depth = frames.len();
            if frames.len() > MAX_NESTING {
                self.err_here("PAR073", "undani mno");
                return None;
            }
            if self.is_eof() {
                let code = match frames.last() {
                    Some(Frame::Match { .. }) => ("PAR052", "linganisha inahitaji '}'"),
                    _ => ("PAR021", "kizuizi inahitaji '}'"),
                };
                self.err_here(code.0, code.1);
                return None;
            }
            let start = self.pos;
            let step = match frames.last() {
                Some(Frame::Match { .. }) => self.match_step(frames),
                _ if self.match_tok(";") => Some(None),
                _ if self.match_tok("}") => self.close_block(frames),
                _ => self.statement(frames).map(|()| None),
            };
            match step {
                Some(Some(body)) => return Some(body),
                Some(None) => {}
                None => {
                    if asili_trace::on() {
                        let line = self.peek_n(0).map_or(0, |t| t.line as u32);
                        asili_trace::emit(asili_trace::Tukio::Urejeshaji, "", line);
                    }
                    if let Synced::Lost = self.synchronize(start) {
                        return None;
                    }
                }
            }
        }
    }

    /// The synchronize state: skip to a statement boundary after an error in the statement that
    /// began at `start` (braces it opened before failing are still open).
    fn synchronize(&mut self, start: usize) -> Synced {
        let mut depth = self.tokens[start..self.pos.min(self.tokens.len())]
            .iter()
            .fold(0isize, |d, t| match t.lexeme.as_str() {
                "{" => d + 1,
                "}" => d - 1,
                _ => d,
            })
            .max(0) as usize;
        while !self.is_eof() {
            let t = self.peek();
            match t.lexeme.as_str() {
                "{" => depth += 1,
                "}" if depth == 0 => return Synced::Boundary,
                "}" => depth -= 1,
                ";" if depth == 0 => {
                    self.pos += 1;
                    return Synced::Boundary;
                }
                // A new top-level item: this body's `}` is missing.
                "kazi" | "umma" | "umbo" | "jenum" | "sifa" | "shughuli" | "leta"
                    if depth == 0 && t.column == 1 =>
                {
                    return Synced::Lost;
                }
                _ => {}
            }
            self.pos += 1;
        }
        Synced::Lost
    }

    /// One `linganisha` step: its closing `}`, or the next arm `pattern => {`.
    fn match_step(&mut self, frames: &mut Vec<Frame>) -> Option<Option<Block>> {
        if self.match_tok("}") {
            let Some(Frame::Match {
                expr,
                arms,
                line,
                span,
            }) = frames.pop()
            else {
                unreachable!("called on a match frame");
            };
            drop(span);
            self.emit(frames, Stmt::Match { expr, arms, line });
            return Some(None);
        }
        let pattern = self.parse_pattern()?;
        self.consume("=>", "PAR051", "mkono wa linganisha unahitaji '=>'")?;
        self.open_block(frames, Kind::Arm { pattern })?;
        Some(None)
    }

    fn open_block(&mut self, frames: &mut Vec<Frame>, kind: Kind) -> Option<()> {
        let line = self.consume("{", "PAR020", "kizuizi inahitaji '{'")?.line;
        let span = asili_trace::enter(kind.name(), line as u32);
        frames.push(Frame::Block {
            stmts: Vec::new(),
            kind,
            span,
        });
        Some(())
    }

    /// Hand a finished statement to the block below.
    fn emit(&mut self, frames: &mut [Frame], stmt: Stmt) {
        if let Some(Frame::Block { stmts, .. }) = frames.last_mut() {
            stmts.push(stmt);
        }
    }

    /// `}` was just consumed: finish the top block and run its transition. `Some(Some(body))`
    /// when it was the function body.
    fn close_block(&mut self, frames: &mut Vec<Frame>) -> Option<Option<Block>> {
        let Some(Frame::Block { stmts, kind, span }) = frames.pop() else {
            unreachable!("called on a block frame");
        };
        drop(span);
        let block = Block { statements: stmts };
        match kind {
            Kind::Body => return Some(Some(block)),
            Kind::IfThen { cond, line } => {
                let chain = IfChain {
                    cond,
                    then_block: block,
                    else_if: Vec::new(),
                    line,
                };
                self.continue_if(frames, chain)?;
            }
            Kind::IfElif { mut chain, cond } => {
                chain.else_if.push((cond, block));
                self.continue_if(frames, chain)?;
            }
            Kind::IfElse { chain } => self.emit(
                frames,
                Stmt::If {
                    cond: chain.cond,
                    then_block: chain.then_block,
                    else_if: chain.else_if,
                    else_block: Some(block),
                    line: chain.line,
                },
            ),
            Kind::While { label, cond, line } => self.emit(
                frames,
                Stmt::While {
                    label,
                    cond,
                    body: block,
                    line,
                },
            ),
            Kind::For {
                label,
                var,
                var_column,
                mode,
                line,
            } => self.emit(
                frames,
                Stmt::For {
                    label,
                    var,
                    var_column,
                    mode,
                    body: block,
                    line,
                },
            ),
            Kind::Arm { pattern } => {
                let line = self.prev().line;
                if let Some(Frame::Match { arms, .. }) = frames.last_mut() {
                    arms.push(MatchArm {
                        pattern,
                        body: block,
                        line,
                    });
                }
                self.match_tok(",");
            }
        }
        Some(None)
    }

    /// After an `ikiwa` or `au_ikiwa` block: another `au_ikiwa`, a `vinginevyo`, or the end.
    fn continue_if(&mut self, frames: &mut Vec<Frame>, chain: IfChain) -> Option<()> {
        if self.match_tok("au_ikiwa") {
            let cond = self.parse_expression()?;
            return self.open_block(frames, Kind::IfElif { chain, cond });
        }
        if self.match_tok("vinginevyo") {
            return self.open_block(frames, Kind::IfElse { chain });
        }
        self.emit(
            frames,
            Stmt::If {
                cond: chain.cond,
                then_block: chain.then_block,
                else_if: chain.else_if,
                else_block: None,
                line: chain.line,
            },
        );
        Some(())
    }

    /// One statement in the top block: a simple statement is parsed whole and added; a compound
    /// one parses its header and opens its block.
    fn statement(&mut self, frames: &mut Vec<Frame>) -> Option<()> {
        if self.check("weka") || self.check("thabiti") {
            let group = self.parse_let_group()?;
            if let Some(Frame::Block { stmts, .. }) = frames.last_mut() {
                stmts.extend(group);
            }
            return Some(());
        }
        let label = if self.match_tok("lebo") {
            let id = self
                .consume_ident("PAR042", "lebo inahitaji jina")
                .map(|t| t.lexeme.trim_start_matches('\'').to_string());
            let _ = self.consume(":", "PAR043", "lebo inahitaji ':'");
            id
        } else {
            None
        };
        if self.match_tok("ikiwa") {
            let line = self.prev().line;
            let cond = self.parse_expression()?;
            return self.open_block(frames, Kind::IfThen { cond, line });
        }
        if self.match_tok("wakati") {
            let line = self.prev().line;
            let cond = if self.match_tok("milele") {
                self.exprs.add(Expr::Bool(true))
            } else {
                self.parse_expression()?
            };
            return self.open_block(frames, Kind::While { label, cond, line });
        }
        if self.match_tok("kwa") {
            let line = self.prev().line;
            let var_tok = self.consume_ident("PAR054", "kwa inahitaji jina")?;
            let mode = if self.match_tok("katika") {
                ForMode::InExpr(self.parse_expression()?)
            } else if self.match_tok("kutoka") {
                let start = self.parse_expression()?;
                self.consume("hadi", "PAR055", "kwa kutoka inahitaji 'hadi'")?;
                let end = self.parse_expression()?;
                ForMode::Range { start, end }
            } else {
                self.err_here("PAR056", "kwa inahitaji 'katika' au 'kutoka ... hadi ...'");
                return None;
            };
            return self.open_block(
                frames,
                Kind::For {
                    label,
                    var: var_tok.lexeme,
                    var_column: var_tok.column,
                    mode,
                    line,
                },
            );
        }
        if self.match_tok("linganisha") {
            let line = self.prev().line;
            let expr = self.parse_expression()?;
            self.consume("{", "PAR050", "linganisha inahitaji '{'")?;
            frames.push(Frame::Match {
                expr,
                arms: Vec::new(),
                line,
                span: asili_trace::enter("linganisha", line as u32),
            });
            return Some(());
        }
        let stmt = self.parse_simple_stmt()?;
        self.emit(frames, stmt);
        Some(())
    }
}
