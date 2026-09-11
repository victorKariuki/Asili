//! Abstract syntax tree and related types.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub imports: Vec<Import>,
    pub constants: Vec<Constant>,
    pub enums: Vec<EnumDecl>,
    pub functions: Vec<Function>,
    pub structs: Vec<StructDecl>,
    pub traits: Vec<TraitDecl>,
    pub impls: Vec<ImplDecl>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    pub ty: TypeExpr,
    pub value: Expr,
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
    pub line: usize,
    pub column: usize,
    pub attrs: Vec<Attribute>,
    pub is_public: bool,
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
    pub name: String,
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
    pub name: String,
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
        name: String,
        ty: Option<TypeExpr>,
        value: Expr,
        line: usize,
        // `column` of `name` — added for LSP semantic-token highlighting (pata/lsp/src/semantic.rs)
        // so declarations/usages can be colored at their real position, not just column 1.
        column: usize,
    },
    Assign {
        name: String,
        op: AssignOp,
        value: Expr,
        line: usize,
        // see `column` note on `Let` above.
        column: usize,
    },
    If {
        cond: Expr,
        then_block: Block,
        else_if: Vec<(Expr, Block)>,
        else_block: Option<Block>,
        line: usize,
    },
    While {
        label: Option<String>,
        cond: Expr,
        body: Block,
        line: usize,
    },
    For {
        label: Option<String>,
        var: String,
        var_column: usize,
        mode: ForMode,
        body: Block,
        line: usize,
    },
    Match {
        expr: Expr,
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
        value: Option<Expr>,
        line: usize,
    },
    Drop {
        name: String,
        line: usize,
    },
    Expr {
        expr: Expr,
        line: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ForMode {
    InExpr(Expr),
    Range { start: Expr, end: Expr },
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
        name: String,
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
    Bool(bool),
    Char(char),
    // NOTE(syntax-highlighting): Ident used to be a bare Ident(String). It now carries its own
    // line/column so the LSP can emit a semantic-highlight token at every *usage* of a name, not
    // just its declaration site (pata/lsp/src/semantic.rs). If you're touching Expr::Ident call
    // sites elsewhere (attrs.rs extraction, etc.), match with `Expr::Ident { name, .. }`.
    Ident {
        name: String,
        line: usize,
        column: usize,
    },
    Hamna,
    Group(Box<Expr>),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        line: usize,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        line: usize,
    },
    Cast {
        expr: Box<Expr>,
        ty: TypeExpr,
        line: usize,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        line: usize,
    },
    MethodCall {
        receiver: Box<Expr>,
        method_name: String,
        args: Vec<Expr>,
        line: usize,
    },
    List {
        elements: Vec<Expr>,
        line: usize,
    },
    Map {
        entries: Vec<(Expr, Expr)>,
        line: usize,
    },
    StructLiteral {
        struct_name: String,
        fields: Vec<(String, Expr)>,
        // Parallel to `fields` (index-aligned) — the (line, column) of each field *name* at
        // this construction site, e.g. `x` in `Point { x: 1, y: 2 }`. Kept separate from
        // `fields` itself rather than widening its tuple, since `fields` is destructured by
        // position in several other places (evaluator, semantic analyzer) that have no need
        // for a position and would otherwise all need updating for no benefit to them.
        field_positions: Vec<(usize, usize)>,
        line: usize,
    },
    EnumConstruct {
        enum_name: String,
        variant_name: String,
        data: Option<Box<Expr>>,
        line: usize,
        // column of `variant_name` (line is already the variant name's own line).
        column: usize,
    },
    FieldAccess {
        receiver: Box<Expr>,
        field: String,
        line: usize,
        // line/column of `field` itself (not the receiver) — for LSP semantic "property" tokens.
        field_line: usize,
        field_column: usize,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        line: usize,
    },
    Propagate {
        expr: Box<Expr>,
        line: usize,
    },
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
            ValueType::NambaKuu => write!(f, "NambaKuu"),
            ValueType::NambaSahihi => write!(f, "NambaSahihi"),
            ValueType::Chaguo(t) => write!(f, "Chaguo<{}>", t),
            ValueType::Tokeo(t, e) => write!(f, "Tokeo<{}, {}>", t, e),
            ValueType::Rejeo(t, m) => write!(f, "Rejeo<{}, {}>", t, m),
            ValueType::Orodha(t) => write!(f, "Orodha<{}>", t),
            ValueType::Kamusi(k, v) => write!(f, "Kamusi<{}, {}>", k, v),
            ValueType::Mfululizo(t) => write!(f, "Mfululizo<{}>", t),
            ValueType::Jozi(a, b) => write!(f, "Jozi<{}, {}>", a, b),
            ValueType::Seti(t) => write!(f, "Seti<{}>", t),
            ValueType::Struct(name) => write!(f, "{}", name),
            ValueType::Wakati => write!(f, "Wakati"),
            ValueType::Anuani => write!(f, "Anuani"),
            ValueType::KashaGC(t) => write!(f, "Kasha_GC<{}>", t),
            ValueType::TypeVar(name) => write!(f, "{}", name),
            ValueType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Type variable (T, E, U, etc. for generic types).
    TypeVar(String),
    Unknown,
}

/// Contract for external functions (used by semantic_check_with_env).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)] // constructed by callers
pub struct FnContract {
    pub params: Vec<ValueType>,
    pub ret: ValueType,
}
