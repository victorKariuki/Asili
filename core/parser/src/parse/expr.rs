//! The flat expression machine: a precedence (Pratt) parser with no recursion.
//!
//! One loop reads tokens left to right. Finished sub-expressions wait on an explicit operand
//! stack; pending operators and open brackets wait on an explicit frame stack. An incoming binary
//! operator first collapses every pending operator that binds at least as tightly (all levels
//! are left-associative), then waits itself. An opening bracket (`(`, `[`, `{`, a call, an index,
//! a literal, `ikiwa`) pushes a frame that records where its operands start; its closer collapses
//! down to that frame and builds the node. Nesting therefore costs heap, never machine stack.

use crate::Name;
use asili_diagnostics::Diagnostic;

use super::{build_binary, bytes_of_lexeme, strip_string_lexeme_quotes, MAX_NESTING};
use crate::cursor::Parser;
use crate::{BinaryOp, Expr, ExprId, Exprs, UnaryOp};
use asili_lexer::{tk, TokenKind};

/// Binding power of each binary operator (higher binds tighter), lowest first: `au`, `na`,
/// equality, comparison, bitwise or/xor/and, shifts, `+ -`, `* / % //`, `**`. A cast (`kama`)
/// binds tighter than all of them, prefix operators tighter still, postfix operators tightest.
fn binary_op(kind: TokenKind) -> Option<(BinaryOp, u8)> {
    use TokenKind as K;
    Some(match kind {
        K::KwAu | K::PipePipe => (BinaryOp::Or, 1),
        K::KwNa | K::AmpAmp => (BinaryOp::And, 2),
        K::EqEq => (BinaryOp::Eq, 3),
        K::NotEq => (BinaryOp::Ne, 3),
        K::Ge => (BinaryOp::Ge, 4),
        K::Le => (BinaryOp::Le, 4),
        K::Gt => (BinaryOp::Gt, 4),
        K::Lt => (BinaryOp::Lt, 4),
        K::KwAuBiti | K::Pipe => (BinaryOp::BitOr, 5),
        K::KwXorBiti | K::Caret => (BinaryOp::BitXor, 6),
        K::KwNaBiti | K::Amp => (BinaryOp::BitAnd, 7),
        K::KwSogezaKushoto | K::Shl => (BinaryOp::Shl, 8),
        K::KwSogezaKulia | K::Shr => (BinaryOp::Shr, 8),
        K::Plus => (BinaryOp::Add, 9),
        K::Minus => (BinaryOp::Sub, 9),
        K::Star => (BinaryOp::Mul, 10),
        K::Slash | K::SlashSlash => (BinaryOp::Div, 10),
        K::Percent => (BinaryOp::Rem, 10),
        K::StarStar => (BinaryOp::Pow, 11),
        _ => return None,
    })
}

fn prefix_op(kind: TokenKind) -> Option<UnaryOp> {
    use TokenKind as K;
    Some(match kind {
        K::Minus => UnaryOp::Neg,
        K::KwSiyo | K::Bang => UnaryOp::Not,
        K::KwSiyoBiti => UnaryOp::BitNot,
        K::KwAzima => UnaryOp::BorrowImm,
        K::KwAzimaTenda => UnaryOp::BorrowMut,
        K::KwJaribu => UnaryOp::Jaribu,
        _ => return None,
    })
}

/// Where an `ikiwa` value expression is: reading a condition (ends at `{`) or a value (ends at
/// `}`).
#[derive(Clone, Copy, PartialEq)]
enum IfPhase {
    Cond,
    Then,
    ElifCond,
    ElifValue,
    Else,
}

/// An open bracket. `base` is the operand-stack height when it opened: its operands are the
/// ones above it.
enum Open {
    Group,
    Call,
    Method {
        name: String,
        line: usize,
    },
    Index,
    EnumData {
        enum_name: String,
        variant: String,
        line: usize,
        column: usize,
    },
    List {
        line: usize,
        first: bool,
    },
    Repeat {
        line: usize,
    },
    /// `value` is whether the next operand is an entry's value (after `:`).
    Map {
        line: usize,
        value: bool,
    },
    Struct {
        name: String,
        line: usize,
        fields: Vec<String>,
        positions: Vec<(usize, usize)>,
    },
    If {
        line: usize,
        phase: IfPhase,
        has_else: bool,
    },
}

enum Frame {
    Binary {
        tok: TokenKind,
        op: BinaryOp,
        prec: u8,
        line: usize,
    },
    Prefix {
        op: UnaryOp,
        line: usize,
    },
    Open {
        open: Open,
        base: usize,
    },
}

impl Open {
    /// The token that closes this bracket where an operand has just ended, and the error for its
    /// absence.
    fn expected(&self) -> (&'static str, &'static str) {
        match self {
            Open::Group => ("PAR070", "kikundi kinahitaji ')'"),
            Open::Call => ("PAR060", "mwito wa kazi unahitaji ')'"),
            Open::Method { .. } => ("PAR065", "mwito wa njia unahitaji ')'"),
            Open::Index => ("PAR079", "fahirisi inahitaji ']'"),
            Open::EnumData { .. } => ("PAR081", "jenum kigezo data inahitaji ')'"),
            Open::List { .. } | Open::Repeat { .. } => ("PAR079", "orodha inahitaji ']'"),
            Open::Map { value: false, .. } => ("PAR053", "kamusi inahitaji ':'"),
            Open::Map { value: true, .. } => ("PAR053", "kamusi inahitaji '}'"),
            Open::Struct { .. } => ("PAR076", "umbo literal inahitaji '}'"),
            Open::If { phase, .. } => match phase {
                IfPhase::Cond => ("PAR094", "ikiwa ya thamani inahitaji '{'"),
                IfPhase::Then => ("PAR095", "ikiwa ya thamani inahitaji '}'"),
                IfPhase::ElifCond => ("PAR094", "au_ikiwa ya thamani inahitaji '{'"),
                IfPhase::ElifValue => ("PAR095", "au_ikiwa ya thamani inahitaji '}'"),
                IfPhase::Else => ("PAR095", "vinginevyo ya thamani inahitaji '}'"),
            },
        }
    }
}

/// The machine's registers for one expression.
struct Machine {
    operands: Vec<ExprId>,
    frames: Vec<Frame>,
    /// How many `Open` frames are on `frames` (nesting, checked against `MAX_NESTING`).
    open: usize,
}

impl Machine {
    fn pop(&mut self) -> ExprId {
        self.operands
            .pop()
            .expect("an operator always has its operands")
    }

    /// Collapse the top frame, a pending operator, into one operand.
    fn reduce(&mut self, x: &mut Exprs) {
        match self.frames.pop() {
            Some(Frame::Binary { tok, op, line, .. }) => {
                let right = self.pop();
                let left = self.pop();
                self.operands
                    .push(build_binary(x, tok, op, left, right, line));
            }
            Some(Frame::Prefix { op, line }) => {
                let expr = self.pop();
                self.operands.push(x.add(Expr::Unary { op, expr, line }));
            }
            _ => unreachable!("only operators are reduced"),
        }
    }

    /// Collapse pending operators down to the innermost open bracket (or the bottom).
    fn reduce_to_open(&mut self, x: &mut Exprs) {
        while matches!(
            self.frames.last(),
            Some(Frame::Binary { .. } | Frame::Prefix { .. })
        ) {
            self.reduce(x);
        }
    }

    fn innermost(&self) -> Option<&Open> {
        match self.frames.last() {
            Some(Frame::Open { open, .. }) => Some(open),
            _ => None,
        }
    }

    fn push_open(&mut self, open: Open) {
        let base = self.operands.len();
        self.frames.push(Frame::Open { open, base });
        self.open += 1;
    }

    /// Pop the innermost open bracket with the operands it gathered.
    fn pop_open(&mut self) -> (Open, Vec<ExprId>) {
        let Some(Frame::Open { open, base }) = self.frames.pop() else {
            unreachable!("reduce_to_open leaves an open bracket on top");
        };
        self.open -= 1;
        let items = self.operands.split_off(base);
        (open, items)
    }

    /// Replace the top operand with `wrap(it)` (postfix operators and casts).
    fn wrap(&mut self, x: &mut Exprs, wrap: impl FnOnce(ExprId) -> Expr) {
        let top = self.pop();
        self.operands.push(x.add(wrap(top)));
    }
}

impl<'a> Parser<'a> {
    /// Parse one expression. Never recurses: see the module documentation.
    pub(crate) fn parse_expression(&mut self) -> Option<ExprId> {
        let mut m = Machine {
            operands: Vec::new(),
            frames: Vec::new(),
            open: 0,
        };
        // Whether the next token must start an operand (else an operand has just ended).
        let mut want_operand = true;
        // Postfix operators may follow a primary expression, not a cast (`x kama T.f` is not
        // `(x kama T).f`).
        let mut postfix_ok = false;
        loop {
            if self.depth + m.open > MAX_NESTING {
                if !self.is_eof() {
                    self.errors.push(
                        Diagnostic::new("PAR073", "undani mno")
                            .with_stage("uchanganuzi")
                            .with_span(self.peek().line, self.peek().column),
                    );
                }
                return None;
            }
            if want_operand {
                if let Some(op) = self.peek_n(0).and_then(|t| prefix_op(t.kind)) {
                    let line = self.advance().line;
                    m.frames.push(Frame::Prefix { op, line });
                    continue;
                }
                if !self.operand(&mut m)? {
                    continue; // an opening bracket: an operand is still wanted
                }
                want_operand = false;
                postfix_ok = true;
                continue;
            }

            // An operand has just ended. Postfix operators bind to it first.
            if postfix_ok {
                match self.postfix(&mut m)? {
                    Postfix::Applied => continue,
                    Postfix::Opened => {
                        want_operand = true;
                        continue;
                    }
                    Postfix::None => {}
                }
            }
            if self.match_tok(tk!("kama")) {
                let line = self.prev().line;
                while matches!(m.frames.last(), Some(Frame::Prefix { .. })) {
                    m.reduce(&mut self.exprs);
                }
                let ty = self.parse_type();
                m.wrap(&mut self.exprs, |e| Expr::Cast { expr: e, ty, line });
                postfix_ok = false;
                continue;
            }
            if let Some((tok, (op, prec))) = self
                .peek_n(0)
                .and_then(|t| Some((t.kind, binary_op(t.kind)?)))
            {
                let line = self.advance().line;
                while match m.frames.last() {
                    Some(Frame::Prefix { .. }) => true,
                    Some(Frame::Binary { prec: p, .. }) => *p >= prec,
                    _ => false,
                } {
                    m.reduce(&mut self.exprs);
                }
                m.frames.push(Frame::Binary {
                    tok,
                    op,
                    prec,
                    line,
                });
                want_operand = true;
                continue;
            }

            // Not an operator: a separator or closer of the innermost bracket, or the end.
            m.reduce_to_open(&mut self.exprs);
            let Some(expected) = m.innermost().map(Open::expected) else {
                return Some(m.pop());
            };
            match self.close_or_separate(&mut m, expected)? {
                Step::Operand => want_operand = true,
                Step::Closed => postfix_ok = true,
            }
        }
    }

    /// Start an operand at the current token: a whole primary expression (`true`), or an opening
    /// bracket pushed onto `m` (`false`, an operand is still wanted).
    fn operand(&mut self, m: &mut Machine) -> Option<bool> {
        let Some(t) = self.peek_n(0) else {
            self.err_here("PAR071", "usemi usiokubalika");
            return None;
        };
        let (lexeme, line, column) = (t.lexeme.as_str(), t.line, t.column);
        let literal = match t.kind {
            TokenKind::KwIkiwa => {
                self.pos += 1;
                m.push_open(Open::If {
                    line,
                    phase: IfPhase::Cond,
                    has_else: false,
                });
                return Some(false);
            }
            TokenKind::LParen => {
                self.pos += 1;
                m.push_open(Open::Group);
                return Some(false);
            }
            TokenKind::LBracket => {
                self.pos += 1;
                if self.match_tok(tk!("]")) {
                    self.push_node(
                        m,
                        Expr::List {
                            elements: Vec::new(),
                            line,
                        },
                    );
                    return Some(true);
                }
                m.push_open(Open::List { line, first: true });
                return Some(false);
            }
            TokenKind::LBrace => {
                self.pos += 1;
                if self.match_tok(tk!("}")) {
                    self.push_node(
                        m,
                        Expr::Map {
                            entries: Vec::new(),
                            line,
                        },
                    );
                    return Some(true);
                }
                m.push_open(Open::Map { line, value: false });
                return Some(false);
            }
            TokenKind::KwKweli => Some(Expr::Bool(true)),
            TokenKind::KwSiKweli => Some(Expr::Bool(false)),
            TokenKind::Hamna => Some(Expr::Hamna),
            TokenKind::Char => Some(Expr::Char(
                lexeme["CHAR:".len()..].chars().next().unwrap_or('\0'),
            )),
            TokenKind::Str => Some(Expr::String(strip_string_lexeme_quotes(lexeme))),
            TokenKind::Bytes => Some(Expr::Baiti(bytes_of_lexeme(lexeme))),
            _ => None,
        };
        if let Some(literal) = literal {
            self.pos += 1;
            self.push_node(m, literal);
            return Some(true);
        }
        for (prefix, what) in [("0x", "heksadesimali (0x)"), ("0b", "binari (0b)")] {
            if lexeme.len() > 2 && lexeme[..2].eq_ignore_ascii_case(prefix) {
                self.pos += 1;
                self.errors.push(
                    Diagnostic::new(
                        "PAR072",
                        format!("{what} haitumiki — tumia namba za desimali pekee"),
                    )
                    .with_stage("uchanganuzi")
                    .with_span(line, column),
                );
                return None;
            }
        }
        // A lone `.` is reported as a malformed number too.
        if matches!(t.kind, TokenKind::Number | TokenKind::Dot) {
            self.pos += 1;
            if lexeme.trim().parse::<f64>().is_err() {
                self.errors.push(
                    Diagnostic::new(
                        "PAR072",
                        format!("namba batili: \"{lexeme}\" si muundo sahihi wa desimali"),
                    )
                    .with_stage("uchanganuzi")
                    .with_span(line, column),
                );
                return None;
            }
            self.push_node(m, Expr::Number(lexeme.to_string()));
            return Some(true);
        }
        if self.check_ident() {
            self.pos += 1;
            // A struct literal only where `{ field :` (or `{}`) follows; `jina {` alone is a
            // block (`linganisha x {`, `ikiwa sharti {`).
            if self.check(tk!("{")) {
                if self.check_n(1, tk!("}")) {
                    self.pos += 2;
                    self.push_node(
                        m,
                        Expr::StructLiteral {
                            struct_name: lexeme.to_string(),
                            fields: Vec::new(),
                            field_positions: Vec::new(),
                            line,
                        },
                    );
                    return Some(true);
                }
                if self.check_n(2, tk!(":")) {
                    self.pos += 1;
                    m.push_open(Open::Struct {
                        name: lexeme.to_string(),
                        line,
                        fields: Vec::new(),
                        positions: Vec::new(),
                    });
                    self.struct_field(m)?;
                    return Some(false);
                }
            }
            self.push_node(
                m,
                Expr::Ident {
                    name: Name::new(lexeme),
                    line,
                    column,
                },
            );
            return Some(true);
        }
        self.err_here("PAR071", "usemi usiokubalika");
        None
    }

    /// `field :` inside a struct literal, recorded on its frame.
    fn struct_field(&mut self, m: &mut Machine) -> Option<()> {
        let name = self.consume_ident("PAR074", "umbo literal inahitaji jina la uga")?;
        self.consume(
            tk!(":"),
            "PAR075",
            "umbo literal inahitaji ':' baada ya jina la uga",
        )?;
        if let Some(Frame::Open {
            open: Open::Struct {
                fields, positions, ..
            },
            ..
        }) = m.frames.last_mut()
        {
            fields.push(name.lexeme.clone());
            positions.push((name.line, name.column));
        }
        Some(())
    }

    /// A postfix operator on the operand just ended: `(args)`, `.uga`, `.njia(args)`, `[i]`, `?`,
    /// `Jenum::Kigezo(data)`.
    fn postfix(&mut self, m: &mut Machine) -> Option<Postfix> {
        if self.match_tok(tk!("(")) {
            if self.match_tok(tk!(")")) {
                let line = self.prev().line;
                m.wrap(&mut self.exprs, |callee| Expr::Call {
                    callee,
                    args: Vec::new(),
                    line,
                });
                return Some(Postfix::Applied);
            }
            m.push_open(Open::Call);
            return Some(Postfix::Opened);
        }
        if self.match_tok(tk!(".")) {
            let name_tok = self.consume_ident("PAR063", "uga au njia unahitaji jina")?;
            let (name, line, field_column) =
                (name_tok.lexeme.clone(), name_tok.line, name_tok.column);
            if self.match_tok(tk!("(")) {
                if self.match_tok(tk!(")")) {
                    m.wrap(&mut self.exprs, |receiver| Expr::MethodCall {
                        receiver,
                        method_name: name,
                        args: Vec::new(),
                        line,
                    });
                } else {
                    m.push_open(Open::Method { name, line });
                    return Some(Postfix::Opened);
                }
            } else {
                m.wrap(&mut self.exprs, |receiver| Expr::FieldAccess {
                    receiver,
                    field: name,
                    line,
                    field_line: line,
                    field_column,
                });
            }
            return Some(Postfix::Applied);
        }
        if self.match_tok(tk!("[")) {
            m.push_open(Open::Index);
            return Some(Postfix::Opened);
        }
        if self.match_tok(tk!("?")) {
            let line = self.prev().line;
            m.wrap(&mut self.exprs, |e| Expr::Propagate { expr: e, line });
            return Some(Postfix::Applied);
        }
        if self.match_tok(tk!("::")) {
            let Some(Expr::Ident {
                name: enum_name, ..
            }) = m.operands.last().map(|id| &self.exprs[*id])
            else {
                self.err_here("PAR082", ":: inahitaji jina la jenum");
                return None;
            };
            let enum_name = *enum_name;
            let variant = self.consume_ident("PAR080", "jenum kigezo inahitaji jina")?;
            let (line, column) = (variant.line, variant.column);
            m.pop();
            if self.match_tok(tk!("(")) {
                m.push_open(Open::EnumData {
                    enum_name: enum_name.to_string(),
                    variant: variant.lexeme.clone(),
                    line,
                    column,
                });
                return Some(Postfix::Opened);
            } else {
                self.push_node(
                    m,
                    Expr::EnumConstruct {
                        enum_name,
                        variant_name: Name::new(&variant.lexeme),
                        data: None,
                        line,
                        column,
                    },
                );
            }
            return Some(Postfix::Applied);
        }
        Some(Postfix::None)
    }

    /// The token after an operand inside the innermost bracket: a separator (`,` `:` `;`, the
    /// `{` after an `ikiwa` condition), a closer, or an error naming the closer it expected.
    fn close_or_separate(
        &mut self,
        m: &mut Machine,
        (code, msg): (&'static str, &'static str),
    ) -> Option<Step> {
        let t = self.peek_n(0).map(|t| t.kind);
        let Some(Frame::Open { open, .. }) = m.frames.last_mut() else {
            unreachable!("the caller checked for an open bracket");
        };
        let step = match (open, t) {
            (
                bracket @ (Open::Call | Open::Method { .. } | Open::List { .. }),
                Some(TokenKind::Comma),
            ) => {
                let closer = if let Open::List { first, .. } = bracket {
                    *first = false;
                    tk!("]")
                } else {
                    tk!(")")
                };
                self.pos += 1;
                // A trailing comma before the closer is allowed.
                if self.match_tok(closer) {
                    self.close(m);
                    return Some(Step::Closed);
                }
                Step::Operand
            }
            (bracket @ Open::List { first: true, .. }, Some(TokenKind::Semi)) => {
                let Open::List { line, .. } = *bracket else {
                    unreachable!("matched a list");
                };
                *bracket = Open::Repeat { line };
                self.pos += 1;
                Step::Operand
            }
            (
                Open::Map {
                    value: value @ false,
                    ..
                },
                Some(TokenKind::Colon),
            ) => {
                *value = true;
                self.pos += 1;
                Step::Operand
            }
            (
                Open::Map {
                    value: value @ true,
                    ..
                },
                Some(TokenKind::Comma),
            ) => {
                *value = false;
                self.pos += 1;
                if self.match_tok(tk!("}")) {
                    self.close(m);
                    return Some(Step::Closed);
                }
                Step::Operand
            }
            (Open::Struct { .. }, Some(TokenKind::Comma)) => {
                self.pos += 1;
                if self.match_tok(tk!("}")) {
                    self.close(m);
                    return Some(Step::Closed);
                }
                self.struct_field(m)?;
                Step::Operand
            }
            (
                Open::If {
                    phase: phase @ (IfPhase::Cond | IfPhase::ElifCond),
                    ..
                },
                Some(TokenKind::LBrace),
            ) => {
                *phase = if *phase == IfPhase::Cond {
                    IfPhase::Then
                } else {
                    IfPhase::ElifValue
                };
                self.pos += 1;
                Step::Operand
            }
            (
                Open::If {
                    phase: phase @ (IfPhase::Then | IfPhase::ElifValue),
                    has_else,
                    ..
                },
                Some(TokenKind::RBrace),
            ) => {
                self.pos += 1;
                if self.match_tok(tk!("au_ikiwa")) {
                    *phase = IfPhase::ElifCond;
                    return Some(Step::Operand);
                }
                if self.match_tok(tk!("vinginevyo")) {
                    *phase = IfPhase::Else;
                    *has_else = true;
                    self.consume(tk!("{"), "PAR094", "vinginevyo ya thamani inahitaji '{'")?;
                    return Some(Step::Operand);
                }
                self.close(m);
                Step::Closed
            }
            (
                Open::If {
                    phase: IfPhase::Else,
                    ..
                },
                Some(TokenKind::RBrace),
            )
            | (
                Open::Group | Open::Call | Open::Method { .. } | Open::EnumData { .. },
                Some(TokenKind::RParen),
            )
            | (Open::Index | Open::List { .. } | Open::Repeat { .. }, Some(TokenKind::RBracket))
            | (Open::Map { value: true, .. } | Open::Struct { .. }, Some(TokenKind::RBrace)) => {
                self.pos += 1;
                self.close(m);
                Step::Closed
            }
            _ => {
                self.err_here(code, msg);
                return None;
            }
        };
        Some(step)
    }

    /// Build the node of the innermost bracket, whose closer was just consumed.
    fn close(&mut self, m: &mut Machine) {
        let (open, mut items) = m.pop_open();
        let closer_line = self.prev().line;
        let node = match open {
            Open::Group => Expr::Group(items.pop().expect("one operand")),
            Open::Call => {
                let callee = m.pop();
                Expr::Call {
                    callee,
                    args: items,
                    line: closer_line,
                }
            }
            Open::Method { name, line } => {
                let receiver = m.pop();
                Expr::MethodCall {
                    receiver,
                    method_name: name,
                    args: items,
                    line,
                }
            }
            Open::Index => {
                let base = m.pop();
                Expr::Index {
                    base,
                    index: items.pop().expect("one operand"),
                    line: closer_line,
                }
            }
            Open::EnumData {
                enum_name,
                variant,
                line,
                column,
            } => Expr::EnumConstruct {
                enum_name: Name::from(enum_name),
                variant_name: Name::from(variant),
                data: Some(items.pop().expect("one operand")),
                line,
                column,
            },
            Open::List { line, .. } => Expr::List {
                elements: items,
                line,
            },
            // `[thamani; idadi]`: `idadi` copies of `thamani` (`orodha_rudia`).
            Open::Repeat { line } => Expr::Call {
                callee: self.exprs.add(Expr::Ident {
                    name: Name::new("orodha_rudia"),
                    line,
                    column: 0,
                }),
                args: items,
                line,
            },
            Open::Map { line, .. } => {
                let mut entries = Vec::with_capacity(items.len() / 2);
                let mut it = items.into_iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    entries.push((k, v));
                }
                Expr::Map { entries, line }
            }
            Open::Struct {
                name,
                line,
                fields,
                positions,
            } => Expr::StructLiteral {
                struct_name: name,
                fields: fields.into_iter().zip(items).collect(),
                field_positions: positions,
                line,
            },
            Open::If { line, has_else, .. } => {
                let mut it = items.into_iter();
                let cond = it.next().expect("condition");
                let then_expr = it.next().expect("value");
                let mut rest: Vec<ExprId> = it.collect();
                let else_expr = has_else.then(|| rest.pop().expect("else value"));
                let mut else_if = Vec::with_capacity(rest.len() / 2);
                let mut it = rest.into_iter();
                while let (Some(c), Some(v)) = (it.next(), it.next()) {
                    else_if.push((c, v));
                }
                Expr::If {
                    cond,
                    then_expr,
                    else_if,
                    else_expr,
                    line,
                }
            }
        };
        self.push_node(m, node);
    }

    /// Add `node` to the arena and push it as an operand.
    fn push_node(&mut self, m: &mut Machine, node: Expr) {
        let id = self.exprs.add(node);
        m.operands.push(id);
    }
}

/// What a postfix operator did.
enum Postfix {
    /// Wrapped the operand (`.uga`, `?`, `f()`): an operand has still just ended.
    Applied,
    /// Opened a bracket (`f(`, `.njia(`, `[`, `Jenum::Kigezo(`): an operand is wanted.
    Opened,
    /// The token is no postfix operator.
    None,
}

/// What the innermost bracket does after a separator or closer.
enum Step {
    /// Another operand starts (after `,`, `:`, `;`, an `ikiwa` condition's `{`).
    Operand,
    /// The bracket closed: its node is an operand that postfix operators may follow.
    Closed,
}
