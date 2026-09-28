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
    /// Lowest stack address a direct call may run below.
    StackLimit = 11,
    /// Finish a directly called function in the interpreter after it deoptimized.
    Resume = 12,
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
    /// Number of set bits (only generated when the target has `popcnt`).
    Popcnt {
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
    /// Call the direct entry of program function `func` (four pointer-sized arguments, a
    /// status result), linked when the image is laid out.
    CallDirect {
        func: u32,
        args: Vec<VReg>,
        dst: VReg,
    },
    /// The machine stack pointer.
    StackPointer {
        dst: VReg,
    },
    /// Address of this function's call buffer (`Func::call_buffer` bytes in its frame), where
    /// direct calls pass the callee's registers.
    CallBuffer {
        dst: VReg,
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
    /// Bytes of frame space for direct calls' register buffers (0: no direct calls).
    pub call_buffer: u32,
}

pub const RT: VReg = VReg(0);
pub const VM: VReg = VReg(1);
pub const FRAME: VReg = VReg(2);
pub const NUMS: VReg = VReg(3);

/// What a call site may assume about a register's home slot without storing it first.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Home {
    /// Nothing: save it before the call to find it there.
    Unknown,
    /// Always holds the value (an incoming argument the prologue parks and nothing redefines).
    Current,
    /// Never needs storing: the register's only definition is this constant (as bits), so it
    /// is rematerialized instead.
    Const(i64),
}

impl Func {
    pub fn class(&self, v: VReg) -> Class {
        self.classes[v.0 as usize]
    }

    /// `Home` for every register.
    pub fn homes(&self) -> Vec<Home> {
        let mut defs = vec![0u32; self.classes.len()];
        let mut constant = vec![None; self.classes.len()];
        for inst in self.blocks.iter().flat_map(|b| &b.insts) {
            for d in inst.defs() {
                defs[d.0 as usize] += 1;
            }
            match inst {
                Inst::IConst { dst, value } => constant[dst.0 as usize] = Some(*value),
                Inst::FConst { dst, value } => {
                    constant[dst.0 as usize] = Some(value.to_bits() as i64)
                }
                _ => {}
            }
        }
        (0..self.classes.len())
            .map(|i| match (defs[i], constant[i]) {
                (0, _) if i < 4 => Home::Current,
                (1, Some(bits)) => Home::Const(bits),
                _ => Home::Unknown,
            })
            .collect()
    }
}

impl Inst {
    /// Whether the instruction only computes its results (removable when they are unused).
    pub fn is_pure(&self) -> bool {
        !matches!(
            self,
            Inst::Store { .. }
                | Inst::StoreIndex { .. }
                | Inst::Call { .. }
                | Inst::CallDirect { .. }
        )
    }

    /// Registers read.
    pub fn uses(&self) -> Vec<VReg> {
        match self {
            Inst::IConst { .. } | Inst::FConst { .. } => vec![],
            Inst::Mov { src, .. }
            | Inst::Neg { src, .. }
            | Inst::Not { src, .. }
            | Inst::Popcnt { src, .. }
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
            Inst::Call { args, .. } | Inst::CallDirect { args, .. } => args.clone(),
            Inst::StackPointer { .. } | Inst::CallBuffer { .. } => vec![],
        }
    }

    /// Mutable access to the registers read (same order as [`Inst::uses`]).
    pub fn uses_mut(&mut self) -> Vec<&mut VReg> {
        match self {
            Inst::IConst { .. } | Inst::FConst { .. } => vec![],
            Inst::Mov { src, .. }
            | Inst::Neg { src, .. }
            | Inst::Not { src, .. }
            | Inst::Popcnt { src, .. }
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
            Inst::Call { args, .. } | Inst::CallDirect { args, .. } => args.iter_mut().collect(),
            Inst::StackPointer { .. } | Inst::CallBuffer { .. } => vec![],
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
            | Inst::Popcnt { dst, .. }
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
            Inst::CallDirect { dst, .. }
            | Inst::StackPointer { dst }
            | Inst::CallBuffer { dst } => vec![*dst],
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

/// A set of virtual registers.
#[derive(Clone, PartialEq, Eq)]
pub struct RegSet(Vec<u64>);

impl RegSet {
    pub fn new(regs: usize) -> Self {
        RegSet(vec![0; regs.div_ceil(64)])
    }
    pub fn insert(&mut self, v: VReg) {
        self.0[v.0 as usize / 64] |= 1 << (v.0 % 64);
    }
    pub fn remove(&mut self, v: VReg) {
        self.0[v.0 as usize / 64] &= !(1 << (v.0 % 64));
    }
    pub fn contains(&self, v: VReg) -> bool {
        self.0[v.0 as usize / 64] & (1 << (v.0 % 64)) != 0
    }
    pub fn union_with(&mut self, other: &RegSet) {
        for (a, b) in self.0.iter_mut().zip(&other.0) {
            *a |= b;
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = VReg> + '_ {
        self.0.iter().enumerate().flat_map(|(w, &bits)| {
            (0..64)
                .filter(move |i| bits & (1 << i) != 0)
                .map(move |i| VReg((w * 64 + i) as u32))
        })
    }
}

/// Registers live on entry to and exit from each block (backward dataflow).
pub struct Liveness {
    pub live_in: Vec<RegSet>,
    pub live_out: Vec<RegSet>,
}

impl Func {
    pub fn liveness(&self) -> Liveness {
        let n = self.classes.len();
        let nb = self.blocks.len();
        let succs: Vec<Vec<usize>> = self
            .blocks
            .iter()
            .map(|b| b.term.successors().iter().map(|s| s.0 as usize).collect())
            .collect();
        let mut gen = vec![RegSet::new(n); nb];
        let mut kill = vec![RegSet::new(n); nb];
        for (b, block) in self.blocks.iter().enumerate() {
            for u in block.term.uses() {
                gen[b].insert(u);
            }
            for inst in block.insts.iter().rev() {
                for d in inst.defs() {
                    kill[b].insert(d);
                    gen[b].remove(d);
                }
                for u in inst.uses() {
                    gen[b].insert(u);
                }
            }
        }
        let mut live_in = vec![RegSet::new(n); nb];
        let mut live_out = vec![RegSet::new(n); nb];
        let mut changed = true;
        while changed {
            changed = false;
            for b in (0..nb).rev() {
                let mut out = RegSet::new(n);
                for &s in &succs[b] {
                    out.union_with(&live_in[s]);
                }
                let inn = RegSet(
                    (0..gen[b].0.len())
                        .map(|w| gen[b].0[w] | (out.0[w] & !kill[b].0[w]))
                        .collect(),
                );
                if inn != live_in[b] {
                    live_in[b] = inn;
                    changed = true;
                }
                live_out[b] = out;
            }
        }
        Liveness { live_in, live_out }
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
                call_buffer: 0,
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
