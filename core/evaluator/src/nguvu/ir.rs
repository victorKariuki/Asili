//! The backend's intermediate representation: basic blocks of three-address instructions over
//! typed virtual registers. Not SSA — a virtual register may be assigned in several places,
//! exactly like the bytecode register it usually stands for — which keeps lowering a direct
//! translation and lets the register allocator work from liveness alone.

/// Register class of a virtual register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    /// 64-bit integer, pointer or 0/1 flag.
    Int,
    /// IEEE double.
    Float,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VReg(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Block(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntOp {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
    /// Shift left by `b` (0..=63).
    Shl,
    /// Arithmetic shift right by `b` (0..=63).
    Sar,
    /// Truncating signed division; the divisor is never 0 or -1 with `i64::MIN` (lowering
    /// only emits it for proven operands).
    SDiv,
    SRem,
    /// Division of a non-negative dividend by a positive divisor (lowering only emits it for
    /// proven operands), where truncating and unsigned division agree.
    UDiv,
    URem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Integer comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ICond {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// Unsigned `<`.
    Ult,
    /// Unsigned `<=`.
    Ule,
}

/// Float comparison with IEEE semantics: every ordered comparison is false on NaN, `Une` is true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FCond {
    Olt,
    Ole,
    Ogt,
    Oge,
    Oeq,
    Une,
}

/// Runtime entry points (the `native::Runtime` table, by field index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RtFn {
    Exec = 0,
    ListPtr = 1,
    ListLen = 2,
    ListPush = 3,
    ListRemove = 4,
    Fmod = 5,
    Pow = 6,
    Floor = 7,
    Ceil = 8,
    /// Rust's saturating `f64 as i64` (NaN → 0).
    FloatToIntSat = 9,
    /// The interpreter's shift amount: `f64 as i32`, anything outside `0..=63` → 0.
    ShiftAmount = 10,
}

#[derive(Clone, Debug)]
pub enum Inst {
    IConst {
        dst: VReg,
        value: i64,
    },
    FConst {
        dst: VReg,
        value: f64,
    },
    /// Copy within one class.
    Mov {
        dst: VReg,
        src: VReg,
    },
    Int {
        op: IntOp,
        dst: VReg,
        a: VReg,
        b: VReg,
    },
    /// `Int` with a constant right operand (fits in 32 bits sign-extended; shifts `0..=63`;
    /// divisors positive).
    IntImm {
        op: IntOp,
        dst: VReg,
        a: VReg,
        imm: i32,
    },
    Neg {
        dst: VReg,
        src: VReg,
    },
    Not {
        dst: VReg,
        src: VReg,
    },
    /// `dst = a * b` and `ovf = 1` if the signed 64-bit product overflowed.
    MulOverflow {
        dst: VReg,
        ovf: VReg,
        a: VReg,
        b: VReg,
    },
    Float {
        op: FloatOp,
        dst: VReg,
        a: VReg,
        b: VReg,
    },
    /// `dst = (a cond b) ? 1 : 0`.
    ICmp {
        cond: ICond,
        dst: VReg,
        a: VReg,
        b: VReg,
    },
    ICmpImm {
        cond: ICond,
        dst: VReg,
        a: VReg,
        imm: i32,
    },
    /// `dst = ((a & imm) == 0) == zero ? 1 : 0` — a bit test.
    TestImm {
        zero: bool,
        dst: VReg,
        a: VReg,
        imm: i32,
    },
    FCmp {
        cond: FCond,
        dst: VReg,
        a: VReg,
        b: VReg,
    },
    /// `dst = cond != 0 ? a : b` (integer class).
    Select {
        dst: VReg,
        cond: VReg,
        a: VReg,
        b: VReg,
    },
    /// Signed integer → double.
    IntToFloat {
        dst: VReg,
        src: VReg,
    },
    /// Truncating double → integer for values proven to be exact integers.
    FloatToInt {
        dst: VReg,
        src: VReg,
    },
    /// Reinterpret the bits of a double as an integer, and back.
    FloatBits {
        dst: VReg,
        src: VReg,
    },
    BitsFloat {
        dst: VReg,
        src: VReg,
    },
    /// `dst = *(base + offset)` (8 bytes; class of `dst`).
    Load {
        dst: VReg,
        base: VReg,
        offset: i32,
    },
    Store {
        src: VReg,
        base: VReg,
        offset: i32,
    },
    /// `dst = base[index]` / `base[index] = src` for 8-byte elements.
    LoadIndex {
        dst: VReg,
        base: VReg,
        index: VReg,
    },
    StoreIndex {
        src: VReg,
        base: VReg,
        index: VReg,
    },
    /// Call a runtime function; integer arguments and result follow the C ABI by class.
    /// `ret32` marks a `u32` result that must be zero-extended.
    Call {
        target: RtFn,
        args: Vec<VReg>,
        dst: Option<VReg>,
        ret32: bool,
    },
}

#[derive(Clone, Debug)]
pub enum Term {
    Jump(Block),
    /// Branch on `cond != 0`.
    Branch {
        cond: VReg,
        then_: Block,
        else_: Block,
    },
    /// Return the 64-bit status word.
    Return(VReg),
}

#[derive(Clone, Debug)]
pub struct BlockData {
    pub insts: Vec<Inst>,
    pub term: Term,
}

/// One native function. Virtual registers `0..4` are the incoming `rt`, `vm`, `frame` and
/// `nums` arguments.
#[derive(Clone, Debug)]
pub struct Func {
    pub classes: Vec<Class>,
    pub blocks: Vec<BlockData>,
    /// Blocks on rarely taken paths (deoptimization, errors, leaving the call), laid out last.
    pub cold: Vec<bool>,
}

pub const RT: VReg = VReg(0);
pub const VM: VReg = VReg(1);
pub const FRAME: VReg = VReg(2);
pub const NUMS: VReg = VReg(3);

impl Func {
    pub fn class(&self, v: VReg) -> Class {
        self.classes[v.0 as usize]
    }
}

impl Inst {
    /// Whether the instruction only computes its results (removable when they are unused).
    pub fn is_pure(&self) -> bool {
        !matches!(
            self,
            Inst::Store { .. } | Inst::StoreIndex { .. } | Inst::Call { .. }
        )
    }

    /// Registers read.
    pub fn uses(&self) -> Vec<VReg> {
        match self {
            Inst::IConst { .. } | Inst::FConst { .. } => vec![],
            Inst::Mov { src, .. }
            | Inst::Neg { src, .. }
            | Inst::Not { src, .. }
            | Inst::IntToFloat { src, .. }
            | Inst::FloatToInt { src, .. }
            | Inst::FloatBits { src, .. }
            | Inst::BitsFloat { src, .. } => vec![*src],
            Inst::IntImm { a, .. } | Inst::ICmpImm { a, .. } | Inst::TestImm { a, .. } => {
                vec![*a]
            }
            Inst::Int { a, b, .. }
            | Inst::MulOverflow { a, b, .. }
            | Inst::Float { a, b, .. }
            | Inst::ICmp { a, b, .. }
            | Inst::FCmp { a, b, .. } => vec![*a, *b],
            Inst::Select { cond, a, b, .. } => vec![*cond, *a, *b],
            Inst::Load { base, .. } => vec![*base],
            Inst::Store { src, base, .. } => vec![*src, *base],
            Inst::LoadIndex { base, index, .. } => vec![*base, *index],
            Inst::StoreIndex { src, base, index } => vec![*src, *base, *index],
            Inst::Call { args, .. } => args.clone(),
        }
    }

    /// Mutable access to the registers read (same order as [`Inst::uses`]).
    pub fn uses_mut(&mut self) -> Vec<&mut VReg> {
        match self {
            Inst::IConst { .. } | Inst::FConst { .. } => vec![],
            Inst::Mov { src, .. }
            | Inst::Neg { src, .. }
            | Inst::Not { src, .. }
            | Inst::IntToFloat { src, .. }
            | Inst::FloatToInt { src, .. }
            | Inst::FloatBits { src, .. }
            | Inst::BitsFloat { src, .. } => vec![src],
            Inst::IntImm { a, .. } | Inst::ICmpImm { a, .. } | Inst::TestImm { a, .. } => {
                vec![a]
            }
            Inst::Int { a, b, .. }
            | Inst::MulOverflow { a, b, .. }
            | Inst::Float { a, b, .. }
            | Inst::ICmp { a, b, .. }
            | Inst::FCmp { a, b, .. } => vec![a, b],
            Inst::Select { cond, a, b, .. } => vec![cond, a, b],
            Inst::Load { base, .. } => vec![base],
            Inst::Store { src, base, .. } => vec![src, base],
            Inst::LoadIndex { base, index, .. } => vec![base, index],
            Inst::StoreIndex { src, base, index } => vec![src, base, index],
            Inst::Call { args, .. } => args.iter_mut().collect(),
        }
    }

    /// Registers written.
    pub fn defs(&self) -> Vec<VReg> {
        match self {
            Inst::IConst { dst, .. }
            | Inst::FConst { dst, .. }
            | Inst::Mov { dst, .. }
            | Inst::Int { dst, .. }
            | Inst::IntImm { dst, .. }
            | Inst::ICmpImm { dst, .. }
            | Inst::TestImm { dst, .. }
            | Inst::Neg { dst, .. }
            | Inst::Not { dst, .. }
            | Inst::Float { dst, .. }
            | Inst::ICmp { dst, .. }
            | Inst::FCmp { dst, .. }
            | Inst::Select { dst, .. }
            | Inst::IntToFloat { dst, .. }
            | Inst::FloatToInt { dst, .. }
            | Inst::FloatBits { dst, .. }
            | Inst::BitsFloat { dst, .. }
            | Inst::Load { dst, .. }
            | Inst::LoadIndex { dst, .. } => vec![*dst],
            Inst::MulOverflow { dst, ovf, .. } => vec![*dst, *ovf],
            Inst::Store { .. } | Inst::StoreIndex { .. } => vec![],
            Inst::Call { dst, .. } => dst.iter().copied().collect(),
        }
    }
}

impl Term {
    pub fn uses(&self) -> Vec<VReg> {
        match self {
            Term::Jump(_) => vec![],
            Term::Branch { cond, .. } => vec![*cond],
            Term::Return(v) => vec![*v],
        }
    }

    pub fn uses_mut(&mut self) -> Vec<&mut VReg> {
        match self {
            Term::Jump(_) => vec![],
            Term::Branch { cond, .. } => vec![cond],
            Term::Return(v) => vec![v],
        }
    }

    pub fn successors(&self) -> Vec<Block> {
        match self {
            Term::Jump(b) => vec![*b],
            Term::Branch { then_, else_, .. } => vec![*then_, *else_],
            Term::Return(_) => vec![],
        }
    }
}

/// Incremental construction of a [`Func`].
pub struct Builder {
    pub func: Func,
    current: Option<Block>,
    insts: Vec<Inst>,
}

impl Builder {
    pub fn new() -> Self {
        let mut b = Builder {
            func: Func {
                classes: Vec::new(),
                blocks: Vec::new(),
                cold: Vec::new(),
            },
            current: None,
            insts: Vec::new(),
        };
        for _ in 0..4 {
            b.vreg(Class::Int);
        }
        b
    }

    pub fn vreg(&mut self, class: Class) -> VReg {
        self.func.classes.push(class);
        VReg(self.func.classes.len() as u32 - 1)
    }

    /// Reserve a block (filled later by `switch_to`).
    pub fn block(&mut self) -> Block {
        self.func.blocks.push(BlockData {
            insts: Vec::new(),
            term: Term::Return(RT),
        });
        self.func.cold.push(false);
        Block(self.func.blocks.len() as u32 - 1)
    }

    /// A block on a rarely taken path.
    pub fn cold_block(&mut self) -> Block {
        let b = self.block();
        self.func.cold[b.0 as usize] = true;
        b
    }

    /// Start emitting into `b`. The previous block must have been terminated.
    pub fn switch_to(&mut self, b: Block) {
        assert!(self.current.is_none(), "previous block not terminated");
        self.current = Some(b);
    }

    pub fn is_open(&self) -> bool {
        self.current.is_some()
    }

    pub fn push(&mut self, inst: Inst) {
        debug_assert!(self.current.is_some(), "emitting into no block");
        self.insts.push(inst);
    }

    pub fn terminate(&mut self, term: Term) {
        let b = self.current.take().expect("no open block");
        let data = &mut self.func.blocks[b.0 as usize];
        data.insts = std::mem::take(&mut self.insts);
        data.term = term;
    }

    pub fn iconst(&mut self, value: i64) -> VReg {
        let dst = self.vreg(Class::Int);
        self.push(Inst::IConst { dst, value });
        dst
    }

    pub fn fconst(&mut self, value: f64) -> VReg {
        let dst = self.vreg(Class::Float);
        self.push(Inst::FConst { dst, value });
        dst
    }

    pub fn finish(self) -> Func {
        assert!(self.current.is_none(), "unterminated block");
        self.func
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
