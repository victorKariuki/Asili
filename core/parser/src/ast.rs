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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub variants: Vec<EnumVariant>,
    pub line: usize,
    pub is_public: bool,
    pub attrs: Vec<Attribute>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub data: Option<TypeExpr>,
    pub line: usize,
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
    pub attrs: Vec<Attribute>,
    pub is_public: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitDecl {
    pub name: String,
    pub line: usize,
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
    pub attrs: Vec<Attribute>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub line: usize,
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
    },
    Assign {
        name: String,
        op: AssignOp,
        value: Expr,
        line: usize,
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
    Ident(String),
    Struct {
        struct_name: String,
        fields: Vec<(String, Pattern)>,
    },
    Jozi(Box<Pattern>, Box<Pattern>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Number(String),
    String(String),
    Bool(bool),
    Char(char),
    Ident(String),
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
    StructLiteral {
        struct_name: String,
        fields: Vec<(String, Expr)>,
        line: usize,
    },
    FieldAccess {
        receiver: Box<Expr>,
        field: String,
        line: usize,
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
    Unknown,
}

/// Contract for external functions (used by semantic_check_with_env).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)] // constructed by callers
pub struct FnContract {
    pub params: Vec<ValueType>,
    pub ret: ValueType,
}
