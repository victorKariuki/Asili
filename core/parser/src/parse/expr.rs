//! The flat expression machine: a precedence (Pratt) parser with no recursion.
//!
//! One loop reads tokens left to right. Finished sub-expressions wait on an explicit operand
//! stack; pending operators and open brackets wait on an explicit frame stack. An incoming binary
//! operator first collapses every pending operator that binds at least as tightly (all levels
//! are left-associative), then waits itself. An opening bracket (`(`, `[`, `{`, a call, an index,
//! a literal, `ikiwa`) pushes a frame that records where its operands start; its closer collapses
//! down to that frame and builds the node. Nesting therefore costs heap, never machine stack.

use asili_diagnostics::Diagnostic;

use super::{build_binary, strip_string_lexeme_quotes, MAX_NESTING};
use crate::cursor::Parser;
use crate::{BinaryOp, Expr, UnaryOp};

/// Binding power of each binary operator (higher binds tighter), lowest first: `au`, `na`,
/// equality, comparison, bitwise or/xor/and, shifts, `+ -`, `* / % //`, `**`. A cast (`kama`)
/// binds tighter than all of them, prefix operators tighter still, postfix operators tightest.
const BINARY: &[(&str, BinaryOp, u8)] = &[
    ("au", BinaryOp::Or, 1),
    ("||", BinaryOp::Or, 1),
    ("na", BinaryOp::And, 2),
    ("&&", BinaryOp::And, 2),
    ("==", BinaryOp::Eq, 3),
    ("!=", BinaryOp::Ne, 3),
    (">=", BinaryOp::Ge, 4),
    ("<=", BinaryOp::Le, 4),
    (">", BinaryOp::Gt, 4),
    ("<", BinaryOp::Lt, 4),
    ("au_biti", BinaryOp::BitOr, 5),
    ("|", BinaryOp::BitOr, 5),
    ("xor_biti", BinaryOp::BitXor, 6),
    ("^", BinaryOp::BitXor, 6),
    ("na_biti", BinaryOp::BitAnd, 7),
    ("&", BinaryOp::BitAnd, 7),
    ("sogeza_kushoto", BinaryOp::Shl, 8),
    ("sogeza_kulia", BinaryOp::Shr, 8),
    ("<<", BinaryOp::Shl, 8),
    (">>", BinaryOp::Shr, 8),
    ("+", BinaryOp::Add, 9),
    ("-", BinaryOp::Sub, 9),
    ("*", BinaryOp::Mul, 10),
    ("/", BinaryOp::Div, 10),
    ("%", BinaryOp::Rem, 10),
    ("//", BinaryOp::Div, 10),
    ("**", BinaryOp::Pow, 11),
];

const PREFIX: &[(&str, UnaryOp)] = &[
    ("-", UnaryOp::Neg),
    ("siyo", UnaryOp::Not),
    ("!", UnaryOp::Not),
    ("siyo_biti", UnaryOp::BitNot),
    ("azima", UnaryOp::BorrowImm),
    ("azima_tenda", UnaryOp::BorrowMut),
    ("jaribu", UnaryOp::Jaribu),
];

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
        tok: &'static str,
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
    operands: Vec<Expr>,
    frames: Vec<Frame>,
    /// How many `Open` frames are on `frames` (nesting, checked against `MAX_NESTING`).
    open: usize,
}

impl Machine {
    fn pop(&mut self) -> Expr {
        self.operands
            .pop()
            .expect("an operator always has its operands")
    }

    /// Collapse the top frame, a pending operator, into one operand.
    fn reduce(&mut self) {
        match self.frames.pop() {
            Some(Frame::Binary { tok, op, line, .. }) => {
                let right = self.pop();
                let left = self.pop();
                self.operands.push(build_binary(tok, op, left, right, line));
            }
            Some(Frame::Prefix { op, line }) => {
                let expr = Box::new(self.pop());
                self.operands.push(Expr::Unary { op, expr, line });
            }
            _ => unreachable!("only operators are reduced"),
        }
    }

    /// Collapse pending operators down to the innermost open bracket (or the bottom).
    fn reduce_to_open(&mut self) {
        while matches!(
            self.frames.last(),
            Some(Frame::Binary { .. } | Frame::Prefix { .. })
        ) {
            self.reduce();
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
    fn pop_open(&mut self) -> (Open, Vec<Expr>) {
        let Some(Frame::Open { open, base }) = self.frames.pop() else {
            unreachable!("reduce_to_open leaves an open bracket on top");
        };
        self.open -= 1;
        let items = self.operands.split_off(base);
        (open, items)
    }

    /// Replace the top operand with `wrap(it)` (postfix operators and casts).
    fn wrap(&mut self, wrap: impl FnOnce(Expr) -> Expr) {
        let top = self.pop();
        self.operands.push(wrap(top));
    }
}

impl<'a> Parser<'a> {
    /// Parse one expression. Never recurses: see the module documentation.
    pub(crate) fn parse_expression(&mut self) -> Option<Expr> {
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
                if let Some(op) = PREFIX
                    .iter()
                    .find(|(t, _)| self.check(t))
                    .map(|p| p.1.clone())
                {
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
            if self.match_tok("kama") {
                let line = self.prev().line;
                while matches!(m.frames.last(), Some(Frame::Prefix { .. })) {
                    m.reduce();
                }
                let ty = self.parse_type();
                m.wrap(|e| Expr::Cast {
                    expr: Box::new(e),
                    ty,
                    line,
                });
                postfix_ok = false;
                continue;
            }
            if let Some(&(tok, ref op, prec)) = BINARY.iter().find(|(t, ..)| self.check(t)) {
                let line = self.advance().line;
                while match m.frames.last() {
                    Some(Frame::Prefix { .. }) => true,
                    Some(Frame::Binary { prec: p, .. }) => *p >= prec,
                    _ => false,
                } {
                    m.reduce();
                }
                m.frames.push(Frame::Binary {
                    tok,
                    op: op.clone(),
                    prec,
                    line,
                });
                want_operand = true;
                continue;
            }

            // Not an operator: a separator or closer of the innermost bracket, or the end.
            m.reduce_to_open();
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
        let t = self.peek_n(0).map(|t| (t.lexeme.clone(), t.line, t.column));
        let Some((lexeme, line, column)) = t else {
            self.err_here("PAR071", "usemi usiokubalika");
            return None;
        };
        match lexeme.as_str() {
            "ikiwa" => {
                self.pos += 1;
                m.push_open(Open::If {
                    line,
                    phase: IfPhase::Cond,
                    has_else: false,
                });
                return Some(false);
            }
            "(" => {
                self.pos += 1;
                m.push_open(Open::Group);
                return Some(false);
            }
            "kweli" | "si_kweli" | "Hamna" => {
                self.pos += 1;
                m.operands.push(match lexeme.as_str() {
                    "kweli" => Expr::Bool(true),
                    "si_kweli" => Expr::Bool(false),
                    _ => Expr::Hamna,
                });
                return Some(true);
            }
            "[" => {
                self.pos += 1;
                if self.match_tok("]") {
                    m.operands.push(Expr::List {
                        elements: Vec::new(),
                        line,
                    });
                    return Some(true);
                }
                m.push_open(Open::List { line, first: true });
                return Some(false);
            }
            "{" => {
                self.pos += 1;
                if self.match_tok("}") {
                    m.operands.push(Expr::Map {
                        entries: Vec::new(),
                        line,
                    });
                    return Some(true);
                }
                m.push_open(Open::Map { line, value: false });
                return Some(false);
            }
            _ => {}
        }
        if let Some(ch) = lexeme.strip_prefix("CHAR:") {
            self.pos += 1;
            m.operands
                .push(Expr::Char(ch.chars().next().unwrap_or('\0')));
            return Some(true);
        }
        if lexeme.starts_with('"') {
            self.pos += 1;
            m.operands
                .push(Expr::String(strip_string_lexeme_quotes(&lexeme)));
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
        if lexeme.chars().all(|c| c.is_ascii_digit() || c == '.') {
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
            m.operands.push(Expr::Number(lexeme));
            return Some(true);
        }
        if self.check_ident() {
            self.pos += 1;
            // A struct literal only where `{ field :` (or `{}`) follows; `jina {` alone is a
            // block (`linganisha x {`, `ikiwa sharti {`).
            if self.check("{") {
                let first = self.peek_n(1).map(|u| u.lexeme.as_str());
                let second = self.peek_n(2).map(|u| u.lexeme.as_str());
                if first == Some("}") {
                    self.pos += 2;
                    m.operands.push(Expr::StructLiteral {
                        struct_name: lexeme,
                        fields: Vec::new(),
                        field_positions: Vec::new(),
                        line,
                    });
                    return Some(true);
                }
                if second == Some(":") {
                    self.pos += 1;
                    m.push_open(Open::Struct {
                        name: lexeme,
                        line,
                        fields: Vec::new(),
                        positions: Vec::new(),
                    });
                    self.struct_field(m)?;
                    return Some(false);
                }
            }
            m.operands.push(Expr::Ident {
                name: lexeme,
                line,
                column,
            });
            return Some(true);
        }
        self.err_here("PAR071", "usemi usiokubalika");
        None
    }

    /// `field :` inside a struct literal, recorded on its frame.
    fn struct_field(&mut self, m: &mut Machine) -> Option<()> {
        let name = self.consume_ident("PAR074", "umbo literal inahitaji jina la uga")?;
        self.consume(
            ":",
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
            fields.push(name.lexeme);
            positions.push((name.line, name.column));
        }
        Some(())
    }

    /// A postfix operator on the operand just ended: `(args)`, `.uga`, `.njia(args)`, `[i]`, `?`,
    /// `Jenum::Kigezo(data)`.
    fn postfix(&mut self, m: &mut Machine) -> Option<Postfix> {
        if self.match_tok("(") {
            if self.match_tok(")") {
                let line = self.prev().line;
                m.wrap(|callee| Expr::Call {
                    callee: Box::new(callee),
                    args: Vec::new(),
                    line,
                });
                return Some(Postfix::Applied);
            }
            m.push_open(Open::Call);
            return Some(Postfix::Opened);
        }
        if self.match_tok(".") {
            let name_tok = self.consume_ident("PAR063", "uga au njia unahitaji jina")?;
            let (name, line, field_column) = (name_tok.lexeme, name_tok.line, name_tok.column);
            if self.match_tok("(") {
                if self.match_tok(")") {
                    m.wrap(|receiver| Expr::MethodCall {
                        receiver: Box::new(receiver),
                        method_name: name,
                        args: Vec::new(),
                        line,
                    });
                } else {
                    m.push_open(Open::Method { name, line });
                    return Some(Postfix::Opened);
                }
            } else {
                m.wrap(|receiver| Expr::FieldAccess {
                    receiver: Box::new(receiver),
                    field: name,
                    line,
                    field_line: line,
                    field_column,
                });
            }
            return Some(Postfix::Applied);
        }
        if self.match_tok("[") {
            m.push_open(Open::Index);
            return Some(Postfix::Opened);
        }
        if self.match_tok("?") {
            let line = self.prev().line;
            m.wrap(|e| Expr::Propagate {
                expr: Box::new(e),
                line,
            });
            return Some(Postfix::Applied);
        }
        if self.match_tok("::") {
            let Some(Expr::Ident {
                name: enum_name, ..
            }) = m.operands.last()
            else {
                self.err_here("PAR082", ":: inahitaji jina la jenum");
                return None;
            };
            let enum_name = enum_name.clone();
            let variant = self.consume_ident("PAR080", "jenum kigezo inahitaji jina")?;
            let (line, column) = (variant.line, variant.column);
            m.pop();
            if self.match_tok("(") {
                m.push_open(Open::EnumData {
                    enum_name,
                    variant: variant.lexeme,
                    line,
                    column,
                });
                return Some(Postfix::Opened);
            } else {
                m.operands.push(Expr::EnumConstruct {
                    enum_name,
                    variant_name: variant.lexeme,
                    data: None,
                    line,
                    column,
                });
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
        let t = self.peek_n(0).map(|t| t.lexeme.clone()).unwrap_or_default();
        let Some(Frame::Open { open, .. }) = m.frames.last_mut() else {
            unreachable!("the caller checked for an open bracket");
        };
        let step = match (open, t.as_str()) {
            (bracket @ (Open::Call | Open::Method { .. } | Open::List { .. }), ",") => {
                let closer = if let Open::List { first, .. } = bracket {
                    *first = false;
                    "]"
                } else {
                    ")"
                };
                self.pos += 1;
                // A trailing comma before the closer is allowed.
                if self.match_tok(closer) {
                    self.close(m);
                    return Some(Step::Closed);
                }
                Step::Operand
            }
            (bracket @ Open::List { first: true, .. }, ";") => {
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
                ":",
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
                ",",
            ) => {
                *value = false;
                self.pos += 1;
                if self.match_tok("}") {
                    self.close(m);
                    return Some(Step::Closed);
                }
                Step::Operand
            }
            (Open::Struct { .. }, ",") => {
                self.pos += 1;
                if self.match_tok("}") {
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
                "{",
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
                "}",
            ) => {
                self.pos += 1;
                if self.match_tok("au_ikiwa") {
                    *phase = IfPhase::ElifCond;
                    return Some(Step::Operand);
                }
                if self.match_tok("vinginevyo") {
                    *phase = IfPhase::Else;
                    *has_else = true;
                    self.consume("{", "PAR094", "vinginevyo ya thamani inahitaji '{'")?;
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
                "}",
            )
            | (Open::Group | Open::Call | Open::Method { .. } | Open::EnumData { .. }, ")")
            | (Open::Index | Open::List { .. } | Open::Repeat { .. }, "]")
            | (Open::Map { value: true, .. } | Open::Struct { .. }, "}") => {
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
            Open::Group => Expr::Group(Box::new(items.pop().expect("one operand"))),
            Open::Call => {
                let callee = m.pop();
                Expr::Call {
                    callee: Box::new(callee),
                    args: items,
                    line: closer_line,
                }
            }
            Open::Method { name, line } => {
                let receiver = m.pop();
                Expr::MethodCall {
                    receiver: Box::new(receiver),
                    method_name: name,
                    args: items,
                    line,
                }
            }
            Open::Index => {
                let base = m.pop();
                Expr::Index {
                    base: Box::new(base),
                    index: Box::new(items.pop().expect("one operand")),
                    line: closer_line,
                }
            }
            Open::EnumData {
                enum_name,
                variant,
                line,
                column,
            } => Expr::EnumConstruct {
                enum_name,
                variant_name: variant,
                data: Some(Box::new(items.pop().expect("one operand"))),
                line,
                column,
            },
            Open::List { line, .. } => Expr::List {
                elements: items,
                line,
            },
            // `[thamani; idadi]`: `idadi` copies of `thamani` (`orodha_rudia`).
            Open::Repeat { line } => Expr::Call {
                callee: Box::new(Expr::Ident {
                    name: "orodha_rudia".to_string(),
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
                let cond = Box::new(it.next().expect("condition"));
                let then_expr = Box::new(it.next().expect("value"));
                let mut rest: Vec<Expr> = it.collect();
                let else_expr = has_else.then(|| Box::new(rest.pop().expect("else value")));
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
        m.operands.push(node);
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
