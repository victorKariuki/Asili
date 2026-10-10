//! Abstract syntax tree and related types.
//!
//! Expressions live in one contiguous arena per module ([`Exprs`], `Module::exprs`): a node
//! refers to its sub-expressions by [`ExprId`] (an index), never by pointer. Walking, cloning,
//! serializing or dropping a module's expressions is a pass over one array, however deeply the
//! source nests. Statements hold the ids of their expressions; each block's statements are
//! already one contiguous `Vec`.

use serde::{Deserialize, Serialize};

use crate::Name;

/// An expression: an index into its module's [`Exprs`] arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExprId(pub u32);

/// A module's expression nodes, contiguous; children are [`ExprId`]s into the same arena.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Exprs(Vec<Expr>);

impl Exprs {
    /// Append a node; its children must already be in this arena.
    pub fn add(&mut self, expr: Expr) -> ExprId {
        let id = ExprId(u32::try_from(self.0.len()).expect("fewer than 2^32 expressions"));
        self.0.push(expr);
        id
    }

    /// `root` and every expression under it, in preorder (an explicit stack, no recursion): the
    /// one walk analyses use to visit a whole expression.
    pub fn descendants(&self, root: ExprId) -> Vec<ExprId> {
        let mut out = Vec::new();
        let mut work = vec![root];
        while let Some(e) = work.pop() {
            out.push(e);
            work.extend(self[e].children().into_iter().rev());
        }
        out
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Copy the expression `id` of `from` and everything under it into this arena (children
    /// first, with an explicit stack), returning its id here. Used when a function or constant
    /// moves between modules (`merge_modules`).
    pub fn import(&mut self, from: &Exprs, id: ExprId) -> ExprId {
        let mut copied: std::collections::HashMap<ExprId, ExprId> = Default::default();
        let mut work = vec![(id, false)];
        while let Some((e, children_done)) = work.pop() {
            if copied.contains_key(&e) {
                continue;
            }
            let node = &from[e];
            if !children_done {
                work.push((e, true));
                work.extend(node.children().into_iter().map(|c| (c, false)));
                continue;
            }
            let moved = node.map_children(|c| copied[&c]);
            let new = self.add(moved);
            copied.insert(e, new);
        }
        copied[&id]
    }
}

impl std::ops::Index<ExprId> for Exprs {
    type Output = Expr;
    fn index(&self, id: ExprId) -> &Expr {
        &self.0[id.0 as usize]
    }
}

impl std::ops::Index<ExprId> for Module {
    type Output = Expr;
    fn index(&self, id: ExprId) -> &Expr {
        &self.exprs[id]
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Module {
    /// Every expression of the module (see the module documentation).
    pub exprs: Exprs,
    pub imports: Vec<Import>,
    pub constants: Vec<Constant>,
    pub enums: Vec<EnumDecl>,
    pub functions: Vec<Function>,
    pub structs: Vec<StructDecl>,
    pub traits: Vec<TraitDecl>,
    pub impls: Vec<ImplDecl>,
}

impl Module {
    /// Append `other`'s items after this module's and its expressions after this arena's (their
    /// ids offset): exactly the module that parsing the two sources one after the other gives.
    pub fn append(&mut self, other: &Module) {
        let base = u32::try_from(self.exprs.len()).expect("fewer than 2^32 expressions");
        let shift = |id: ExprId| ExprId(id.0 + base);
        self.exprs
            .0
            .extend(other.exprs.0.iter().map(|e| e.map_children(shift)));
        let function = |f: &Function| {
            let mut f = f.clone();
            f.body.map_expr_roots(&mut |id| shift(id));
            f
        };
        self.imports.extend(other.imports.iter().cloned());
        self.constants
            .extend(other.constants.iter().map(|c| Constant {
                value: shift(c.value),
                ..c.clone()
            }));
        self.enums.extend(other.enums.iter().cloned());
        self.functions.extend(other.functions.iter().map(function));
        self.structs.extend(other.structs.iter().cloned());
        self.traits.extend(other.traits.iter().cloned());
        self.impls.extend(other.impls.iter().map(|i| ImplDecl {
            body: i.body.iter().map(function).collect(),
            ..i.clone()
        }));
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    pub ty: TypeExpr,
    pub value: ExprId,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub variants: Vec<EnumVariant>,
    pub line: usize,
    pub column: usize,
    pub is_public: bool,
    pub attrs: Vec<Attribute>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub data: Option<TypeExpr>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ImportPath {
    Full(String),
    Selective { module: String, names: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Import {
    pub path: ImportPath,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub args: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructDecl {
    pub name: String,
    pub generics: Vec<String>,
    /// Field names and optional types. Order is declaration order.
    pub fields: Vec<(String, Option<TypeExpr>)>,
    pub line: usize,
    // column of `name` — see the Expr::Ident NOTE below; same reasoning for LSP semantic tokens.
    pub column: usize,
    pub attrs: Vec<Attribute>,
    pub is_public: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitDecl {
    pub name: String,
    /// Required method signatures (no bodies) — checked for completeness against every
    /// `ImplDecl` naming this trait. Empty for a trait declared with no body/braces.
    pub methods: Vec<TraitMethodSig>,
    pub line: usize,
    pub column: usize,
    pub attrs: Vec<Attribute>,
    pub is_public: bool,
}

/// One required method signature inside a `sifa` body — no body, just the contract an `impl`
/// must satisfy. `Self` in `params`/`return_type` refers to the implementing type, resolved at
/// completeness-check time, not parse time (the trait doesn't know its implementers).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitMethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImplDecl {
    pub target: String,
    pub trait_name: Option<String>,
    /// Method functions (kazi inside the impl block).
    pub body: Vec<Function>,
    pub line: usize,
    pub attrs: Vec<Attribute>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub name: Name,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub body: Block,
    pub is_test: bool,
    pub is_public: bool,
    pub line: usize,
    pub column: usize,
    pub attrs: Vec<Attribute>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: Name,
    pub ty: TypeExpr,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypeExpr {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Let {
        mutable: bool,
        name: Name,
        ty: Option<TypeExpr>,
        value: ExprId,
        line: usize,
        // `column` of `name` — added for LSP semantic-token highlighting (pata/lsp/src/semantic.rs)
        // so declarations/usages can be colored at their real position, not just column 1.
        column: usize,
    },
    LetPattern {
        mutable: bool,
        pattern: Pattern,
        value: ExprId,
        line: usize,
    },
    Assign {
        name: Name,
        op: AssignOp,
        value: ExprId,
        line: usize,
        // see `column` note on `Let` above.
        column: usize,
    },
    If {
        cond: ExprId,
        then_block: Block,
        else_if: Vec<(ExprId, Block)>,
        else_block: Option<Block>,
        line: usize,
    },
    While {
        label: Option<String>,
        cond: ExprId,
        body: Block,
        line: usize,
    },
    For {
        label: Option<String>,
        var: Name,
        var_column: usize,
        mode: ForMode,
        body: Block,
        line: usize,
    },
    Match {
        expr: ExprId,
        arms: Vec<MatchArm>,
        line: usize,
    },
    Break {
        label: Option<String>,
        line: usize,
    },
    Continue {
        label: Option<String>,
        line: usize,
    },
    Return {
        value: Option<ExprId>,
        line: usize,
    },
    Drop {
        name: Name,
        line: usize,
    },
    Expr {
        expr: ExprId,
        line: usize,
    },
}

impl Block {
    /// Replace every expression id the statements hold directly (not their sub-expressions: a
    /// node's children are ids into the same arena) with `f(id)`, nested blocks included —
    /// walked with an explicit stack.
    pub fn map_expr_roots(&mut self, f: &mut impl FnMut(ExprId) -> ExprId) {
        let mut work: Vec<&mut Block> = vec![self];
        while let Some(block) = work.pop() {
            for stmt in &mut block.statements {
                match stmt {
                    Stmt::Let { value, .. }
                    | Stmt::LetPattern { value, .. }
                    | Stmt::Assign { value, .. }
                    | Stmt::Expr { expr: value, .. } => *value = f(*value),
                    Stmt::Return { value, .. } => {
                        if let Some(v) = value {
                            *v = f(*v);
                        }
                    }
                    Stmt::If {
                        cond,
                        then_block,
                        else_if,
                        else_block,
                        ..
                    } => {
                        *cond = f(*cond);
                        work.push(then_block);
                        for (c, b) in else_if {
                            *c = f(*c);
                            work.push(b);
                        }
                        work.extend(else_block.as_mut());
                    }
                    Stmt::While { cond, body, .. } => {
                        *cond = f(*cond);
                        work.push(body);
                    }
                    Stmt::For { mode, body, .. } => {
                        match mode {
                            ForMode::InExpr(e) => *e = f(*e),
                            ForMode::Range { start, end } => {
                                *start = f(*start);
                                *end = f(*end);
                            }
                        }
                        work.push(body);
                    }
                    Stmt::Match { expr, arms, .. } => {
                        *expr = f(*expr);
                        work.extend(arms.iter_mut().map(|a| &mut a.body));
                    }
                    Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Drop { .. } => {}
                }
            }
        }
    }
}

impl Exprs {
    /// `function` (from a module whose arena is `from`) with its expressions copied into this
    /// arena.
    pub fn import_function(&mut self, from: &Exprs, function: &Function) -> Function {
        let mut f = function.clone();
        f.body.map_expr_roots(&mut |id| self.import(from, id));
        f
    }
}

impl Stmt {
    /// The source line this statement starts at — every variant carries one, used for
    /// line-level coverage instrumentation (`core/evaluator`'s `Runtime::executed_lines`).
    pub fn line(&self) -> usize {
        match self {
            Stmt::Let { line, .. }
            | Stmt::LetPattern { line, .. }
            | Stmt::Assign { line, .. }
            | Stmt::If { line, .. }
            | Stmt::While { line, .. }
            | Stmt::For { line, .. }
            | Stmt::Match { line, .. }
            | Stmt::Break { line, .. }
            | Stmt::Continue { line, .. }
            | Stmt::Return { line, .. }
            | Stmt::Drop { line, .. }
            | Stmt::Expr { line, .. } => *line,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ForMode {
    InExpr(ExprId),
    Range { start: ExprId, end: ExprId },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Block,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Wildcard,
    Literal(Expr),
    // NOTE(syntax-highlighting): mirrors the Expr::Ident change above — carries its own
    // position so a match-arm binding (`n` in `Fulani(n) => ...`) can be tracked as a real
    // scoped local by pata/lsp/src/semantic.rs, not just left uncolored.
    Ident {
        name: Name,
        line: usize,
        column: usize,
    },
    Struct {
        struct_name: String,
        line: usize,
        column: usize,
        fields: Vec<(String, Pattern)>,
    },
    Enum {
        enum_name: String,
        variant_name: String,
        data: Option<Box<Pattern>>,
        // position of `variant_name` specifically — used to emit an enumMember token.
        variant_line: usize,
        variant_column: usize,
    },
    Jozi(Box<Pattern>, Box<Pattern>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Number(String),
    String(String),
    /// A byte-string literal `b"..."`.
    Baiti(Vec<u8>),
    Bool(bool),
    Char(char),
    // NOTE(syntax-highlighting): Ident used to be a bare Ident(String). It now carries its own
    // line/column so the LSP can emit a semantic-highlight token at every *usage* of a name, not
    // just its declaration site (pata/lsp/src/semantic.rs). If you're touching Expr::Ident call
    // sites elsewhere (attrs.rs extraction, etc.), match with `Expr::Ident { name, .. }`.
    Ident {
        name: Name,
        line: usize,
        column: usize,
    },
    Hamna,
    Group(ExprId),
    If {
        cond: ExprId,
        then_expr: ExprId,
        else_if: Vec<(ExprId, ExprId)>,
        else_expr: Option<ExprId>,
        line: usize,
    },
    Unary {
        op: UnaryOp,
        expr: ExprId,
        line: usize,
    },
    Binary {
        left: ExprId,
        op: BinaryOp,
        right: ExprId,
        line: usize,
    },
    Cast {
        expr: ExprId,
        ty: TypeExpr,
        line: usize,
    },
    Call {
        callee: ExprId,
        args: Vec<ExprId>,
        line: usize,
    },
    MethodCall {
        receiver: ExprId,
        method_name: String,
        args: Vec<ExprId>,
        line: usize,
    },
    List {
        elements: Vec<ExprId>,
        line: usize,
    },
    Map {
        entries: Vec<(ExprId, ExprId)>,
        line: usize,
    },
    StructLiteral {
        struct_name: String,
        fields: Vec<(String, ExprId)>,
        // Parallel to `fields` (index-aligned) — the (line, column) of each field *name* at
        // this construction site, e.g. `x` in `Point { x: 1, y: 2 }`. Kept separate from
        // `fields` itself rather than widening its tuple, since `fields` is destructured by
        // position in several other places (evaluator, semantic analyzer) that have no need
        // for a position and would otherwise all need updating for no benefit to them.
        field_positions: Vec<(usize, usize)>,
        line: usize,
    },
    EnumConstruct {
        enum_name: Name,
        variant_name: Name,
        data: Option<ExprId>,
        line: usize,
        // column of `variant_name` (line is already the variant name's own line).
        column: usize,
    },
    FieldAccess {
        receiver: ExprId,
        field: String,
        line: usize,
        // line/column of `field` itself (not the receiver) — for LSP semantic "property" tokens.
        field_line: usize,
        field_column: usize,
    },
    Index {
        base: ExprId,
        index: ExprId,
        line: usize,
    },
    Propagate {
        expr: ExprId,
        line: usize,
    },
}

impl Expr {
    /// The direct sub-expressions of `self`, in evaluation order — the one definition of the
    /// expression tree's shape that analyses walking it (linters, the LSP, the parser's own
    /// checks) share instead of each re-listing every variant.
    pub fn children(&self) -> Vec<ExprId> {
        match self {
            Expr::Number(_)
            | Expr::String(_)
            | Expr::Baiti(_)
            | Expr::Bool(_)
            | Expr::Char(_)
            | Expr::Ident { .. }
            | Expr::Hamna => vec![],
            Expr::Group(e)
            | Expr::Unary { expr: e, .. }
            | Expr::Cast { expr: e, .. }
            | Expr::Propagate { expr: e, .. }
            | Expr::FieldAccess { receiver: e, .. } => vec![*e],
            Expr::If {
                cond,
                then_expr,
                else_if,
                else_expr,
                ..
            } => {
                let mut out = vec![*cond, *then_expr];
                for (c, e) in else_if {
                    out.push(*c);
                    out.push(*e);
                }
                out.extend(*else_expr);
                out
            }
            Expr::Binary { left, right, .. } => vec![*left, *right],
            Expr::Index { base, index, .. } => vec![*base, *index],
            Expr::Call { callee, args, .. } => std::iter::once(*callee)
                .chain(args.iter().copied())
                .collect(),
            Expr::MethodCall { receiver, args, .. } => std::iter::once(*receiver)
                .chain(args.iter().copied())
                .collect(),
            Expr::List { elements, .. } => elements.clone(),
            Expr::Map { entries, .. } => entries.iter().flat_map(|(k, v)| [*k, *v]).collect(),
            Expr::StructLiteral { fields, .. } => fields.iter().map(|(_, e)| *e).collect(),
            Expr::EnumConstruct { data, .. } => data.iter().copied().collect(),
        }
    }

    /// `self` with every child id replaced by `f(id)` (the shape `children` lists, in the same
    /// order).
    pub fn map_children(&self, mut f: impl FnMut(ExprId) -> ExprId) -> Expr {
        let mut node = self.clone();
        match &mut node {
            Expr::Number(_)
            | Expr::String(_)
            | Expr::Baiti(_)
            | Expr::Bool(_)
            | Expr::Char(_)
            | Expr::Ident { .. }
            | Expr::Hamna => {}
            Expr::Group(e)
            | Expr::Unary { expr: e, .. }
            | Expr::Cast { expr: e, .. }
            | Expr::Propagate { expr: e, .. }
            | Expr::FieldAccess { receiver: e, .. } => *e = f(*e),
            Expr::If {
                cond,
                then_expr,
                else_if,
                else_expr,
                ..
            } => {
                *cond = f(*cond);
                *then_expr = f(*then_expr);
                for (c, e) in else_if {
                    *c = f(*c);
                    *e = f(*e);
                }
                if let Some(e) = else_expr {
                    *e = f(*e);
                }
            }
            Expr::Binary {
                left: a, right: b, ..
            }
            | Expr::Index {
                base: a, index: b, ..
            } => {
                *a = f(*a);
                *b = f(*b);
            }
            Expr::Call {
                callee: head, args, ..
            }
            | Expr::MethodCall {
                receiver: head,
                args,
                ..
            } => {
                *head = f(*head);
                for a in args {
                    *a = f(*a);
                }
            }
            Expr::List { elements, .. } => {
                for e in elements {
                    *e = f(*e);
                }
            }
            Expr::Map { entries, .. } => {
                for (k, v) in entries {
                    *k = f(*k);
                    *v = f(*v);
                }
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, e) in fields {
                    *e = f(*e);
                }
            }
            Expr::EnumConstruct { data, .. } => {
                if let Some(e) = data {
                    *e = f(*e);
                }
            }
        }
        node
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    BorrowImm,
    BorrowMut,
    Jaribu,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    BitAnd,
    BitXor,
    BitOr,
    Shl,
    Shr,
    And,
    Or,
}

use std::fmt;

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::Namba => write!(f, "Namba"),
            ValueType::Neno => write!(f, "Neno"),
            ValueType::Ukweli => write!(f, "Ukweli"),
            ValueType::Tupu => write!(f, "Tupu"),
            ValueType::Hamna => write!(f, "Hamna"),
            ValueType::Herufi => write!(f, "Herufi"),
            ValueType::NambaKuu => write!(f, "Namba_Kuu"),
            ValueType::NambaSahihi => write!(f, "Namba_Sahihi"),
            ValueType::Chaguo(t) => write!(f, "Chaguo<{}>", t),
            ValueType::Tokeo(t, e) => write!(f, "Tokeo<{}, {}>", t, e),
            ValueType::Rejeo(t, m) => write!(f, "Rejeo<{}, {}>", t, m),
            ValueType::Orodha(t) => write!(f, "Orodha<{}>", t),
            ValueType::Kamusi(k, v) => write!(f, "Kamusi<{}, {}>", k, v),
            ValueType::Mfululizo(t) => write!(f, "Mfululizo<{}>", t),
            ValueType::Baiti => write!(f, "Baiti"),
            ValueType::Jozi(a, b) => write!(f, "Jozi<{}, {}>", a, b),
            ValueType::Seti(t) => write!(f, "Seti<{}>", t),
            ValueType::Struct(name) => write!(f, "{}", name),
            ValueType::Wakati => write!(f, "Wakati"),
            ValueType::Anuani => write!(f, "Anuani"),
            ValueType::KashaGC(t) => write!(f, "Kasha_GC<{}>", t),
            ValueType::KashaGCDhaifu(t) => write!(f, "Kasha_GC_Dhaifu<{}>", t),
            ValueType::Faili => write!(f, "Faili"),
            ValueType::Mkondo => write!(f, "Mkondo"),
            ValueType::MkondoSikilizaji => write!(f, "MkondoSikilizaji"),
            ValueType::TlsUsanidi => write!(f, "TlsUsanidi"),
            ValueType::Kumbukumbu(t) => write!(f, "Kumbukumbu<{}>", t),
            ValueType::NjiaTx(t) => write!(f, "NjiaTx<{}>", t),
            ValueType::NjiaRx(t) => write!(f, "NjiaRx<{}>", t),
            ValueType::NjiaTxBounded(t) => write!(f, "NjiaTxBounded<{}>", t),
            ValueType::NjiaRxBounded(t) => write!(f, "NjiaRxBounded<{}>", t),
            ValueType::Fungo(t) => write!(f, "Fungo<{}>", t),
            ValueType::TypeVar(name) => write!(f, "{}", name),
            ValueType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    Namba,
    Neno,
    Ukweli,
    Tupu,
    Hamna,
    Herufi,
    NambaKuu,
    NambaSahihi,
    Chaguo(Box<ValueType>),
    Tokeo(Box<ValueType>, Box<ValueType>),
    Rejeo(Box<ValueType>, bool),
    Orodha(Box<ValueType>),
    Kamusi(Box<ValueType>, Box<ValueType>),
    Mfululizo(Box<ValueType>),
    /// Bytes (`b"..."`, `baiti(...)`, `.baiti()`): immutable, any values 0–255.
    Baiti,
    Jozi(Box<ValueType>, Box<ValueType>),
    Seti(Box<ValueType>),
    /// Named struct type (e.g. from umbo Foo).
    Struct(String),
    /// Time value (seconds since epoch); used by majira module.
    Wakati,
    /// Raw memory address; used by syscall and kiungo (FFI).
    Anuani,
    /// Reference-counted shared wrapper (opt-in `leta kasha_gc`); see spec's managed-memory module.
    KashaGC(Box<ValueType>),
    /// Weak reference to a `Kasha_GC<T>` (kasha_gc_dhaifu, downgrade); the cycle-breaking escape
    /// hatch, since `Kasha_GC<T>` itself has no cycle collector.
    KashaGCDhaifu(Box<ValueType>),
    /// File handle (leta faili); owns an OS file descriptor, closed on drop.
    Faili,
    /// Network stream/socket handle (leta mfumo); owns an OS socket, closed on drop.
    Mkondo,
    /// TCP listening socket (mkondo_sikiliza, leta mfumo); shared across mkondo_tumikia's
    /// worker-pool threads.
    MkondoSikilizaji,
    /// Loaded TLS server certificate/key pair (tls_sanidi, leta mfumo); passed to
    /// mkondo_tumikia's optional TLS parameter.
    TlsUsanidi,
    /// Heap-allocated box owning a value of type T; no OS resource, plain owning indirection.
    Kumbukumbu(Box<ValueType>),
    /// Channel sender half (njia, leta sambamba); crosses the tenda thread boundary.
    NjiaTx(Box<ValueType>),
    /// Channel receiver half (njia, leta sambamba).
    NjiaRx(Box<ValueType>),
    /// Bounded channel sender half (njia_na_kikomo, leta sambamba); `.tuma()` blocks once the
    /// bound is full instead of the unbounded NjiaTx's unlimited growth.
    NjiaTxBounded(Box<ValueType>),
    /// Bounded channel receiver half (njia_na_kikomo, leta sambamba).
    NjiaRxBounded(Box<ValueType>),
    /// Mutex (fungo, leta sambamba); protects a shared value across tenda threads.
    Fungo(Box<ValueType>),
    /// Type variable (T, E, U, etc. for generic types).
    TypeVar(String),
    #[default]
    Unknown,
}

/// Contract for external functions (used by semantic_check_with_env).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FnContract {
    pub params: Vec<ValueType>,
    pub ret: ValueType,
    /// Parameter names, parallel to `params` (empty when not known).
    #[serde(default)]
    pub names: Vec<String>,
    /// How many trailing parameters a call may leave out.
    #[serde(default)]
    pub optional: usize,
    /// The last parameter takes any number of arguments (`...hoja: T`), each of its type.
    #[serde(default)]
    pub variadic: bool,
    /// One line on what the function does (LSP hover, `lib/std` stubs).
    #[serde(default)]
    pub doc: String,
}

impl FnContract {
    /// A contract with only types: every parameter required, no names or doc.
    pub fn new(params: Vec<ValueType>, ret: ValueType) -> Self {
        FnContract {
            params,
            ret,
            ..Default::default()
        }
    }

    /// The fewest arguments a call may pass.
    pub fn min_args(&self) -> usize {
        self.params.len() - self.optional - usize::from(self.variadic)
    }

    /// Whether `n` arguments are accepted.
    pub fn accepts(&self, n: usize) -> bool {
        n >= self.min_args() && (self.variadic || n <= self.params.len())
    }

    /// The parameter type argument `i` is checked against (a variadic tail repeats its type).
    pub fn param_for_arg(&self, i: usize) -> Option<&ValueType> {
        self.params
            .get(i)
            .or_else(|| self.variadic.then(|| self.params.last()).flatten())
    }
}
