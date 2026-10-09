//! WebAssembly target: a program's optimized IR as one wasm module, which the host
//! instantiates beside itself (sharing its memory and function table) where native code is a
//! wasm module (the browser playground). Each function keeps the ordinary entry's signature
//! (`(runtime, host, frame, nums) -> status`) and is placed in the host's function table, so
//! the host calls it through a function pointer exactly as it calls machine code elsewhere, and
//! it calls the runtime table's entries with `call_indirect`.
//!
//! Wasm's control flow is structured, so each function's control-flow graph (always reducible:
//! it comes from structured source) is turned into nested `block`/`loop`/`if` by the
//! dominator-tree method of Ramsey, "Beyond Relooper" (ICFP 2022). Every IR register is a wasm
//! local (`i64` or `f64`); addresses are their low 32 bits.

use super::ir::{Block, Class, FCond, FloatOp, Func, ICond, Inst, IntOp, Term, VReg};
use crate::numlist::Kind;

/// The wasm module for `funcs` (the program's functions, in order): it imports `env.memory`,
/// `env.table` and the immutable `env.base` (`i32`), and fills `table[base + i]` with
/// function `i`.
pub fn module(funcs: &[Func]) -> Result<Vec<u8>, String> {
    let mut out = b"\0asm\x01\0\0\0".to_vec();
    // Types: 0 is an entry's, 1 + k the runtime function `k`'s.
    let mut types = Vec::new();
    let entry = [I32, I32, I32, I32];
    func_type(&mut types, &entry, &[I64]);
    for k in 0..RT_COUNT {
        let (params, results) = runtime_signature(k);
        func_type(&mut types, params, results);
    }
    section(&mut out, 1, vector(1 + RT_COUNT, &types));
    // Imports.
    let mut imports = Vec::new();
    import(&mut imports, "env", "memory");
    imports.extend_from_slice(&[0x02, 0x00, 0x00]); // memory, no maximum, minimum 0
    import(&mut imports, "env", "table");
    imports.extend_from_slice(&[0x01, 0x70, 0x00, 0x00]); // table of funcref, minimum 0
    import(&mut imports, "env", "base");
    imports.extend_from_slice(&[0x03, I32, 0x00]); // immutable i32 global
    section(&mut out, 2, vector(3, &imports));
    // Functions, all of the entry type.
    let n = funcs.len() as u32;
    let mut decls = Vec::new();
    for _ in 0..n {
        uleb(&mut decls, 0);
    }
    section(&mut out, 3, vector(n, &decls));
    // Elements: table[base + i] = function i.
    let mut elems = Vec::new();
    uleb(&mut elems, 1);
    elems.push(0x00); // active, table 0, function indices
    elems.extend_from_slice(&[0x23, 0x00, 0x0B]); // global.get 0; end
    uleb(&mut elems, n);
    for i in 0..n {
        uleb(&mut elems, i);
    }
    section(&mut out, 9, elems);
    // Code.
    let mut code = Vec::new();
    for func in funcs {
        let body = function(func)?;
        uleb(&mut code, body.len() as u32);
        code.extend_from_slice(&body);
    }
    section(&mut out, 10, vector(n, &code));
    Ok(out)
}

// -- encoding ---------------------------------------------------------------------------------

const I32: u8 = 0x7F;
const I64: u8 = 0x7E;
const F64: u8 = 0x7C;
const EMPTY: u8 = 0x40;

fn uleb(out: &mut Vec<u8>, mut v: u32) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn sleb(out: &mut Vec<u8>, mut v: i64) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        let done = (v == 0 && byte & 0x40 == 0) || (v == -1 && byte & 0x40 != 0);
        if done {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn section(out: &mut Vec<u8>, id: u8, body: Vec<u8>) {
    out.push(id);
    uleb(out, body.len() as u32);
    out.extend_from_slice(&body);
}

/// `count` items already encoded one after another, as a wasm vector.
fn vector(count: u32, items: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(items.len() + 5);
    uleb(&mut out, count);
    out.extend_from_slice(items);
    out
}

fn name(out: &mut Vec<u8>, s: &str) {
    uleb(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

fn import(out: &mut Vec<u8>, module: &str, field: &str) {
    name(out, module);
    name(out, field);
}

fn func_type(out: &mut Vec<u8>, params: &[u8], results: &[u8]) {
    out.push(0x60);
    uleb(out, params.len() as u32);
    out.extend_from_slice(params);
    uleb(out, results.len() as u32);
    out.extend_from_slice(results);
}

/// Number of runtime functions (`RtFn` variants).
const RT_COUNT: u32 = 19;

/// The wasm signature of runtime function `k` (`native::Runtime`'s field `k`, as Rust lowers its
/// `extern "C"` signature for wasm32: pointers and `u32` as `i32`).
fn runtime_signature(k: u32) -> (&'static [u8], &'static [u8]) {
    const RT: [(&[u8], &[u8]); RT_COUNT as usize] = [
        (&[I32, I32, I32, I32], &[I32]), // Exec
        (&[I32, I32, I32], &[I32]),      // ListPtr
        (&[I32, I32], &[I64]),           // ListLen
        (&[I32, I32], &[I32]),           // ListHead
        (&[I32, I32], &[I64]),           // ListRoom
        (&[I32, I32, F64], &[I32]),      // ListPush
        (&[I32, I32, I64], &[I64]),      // ListRemove
        (&[F64, F64], &[F64]),           // Fmod
        (&[F64, F64], &[F64]),           // Pow
        (&[F64], &[F64]),                // Floor
        (&[F64], &[F64]),                // Ceil
        (&[F64], &[I64]),                // FloatToIntSat
        (&[F64], &[I64]),                // ShiftAmount
        (&[I32, I32, I32], &[I64]),      // CallHost
        (&[I32], &[I64]),                // DepthError
        (&[I32, I32, F64], &[]),         // BoxNum
        (&[I32, I32, F64], &[]),         // BoxBool
        (&[I32, I32, I32], &[]),         // ValMov
        (&[I32, I32, I32, I32], &[]),    // ConstVal
    ];
    RT[k as usize]
}

// -- control flow -----------------------------------------------------------------------------

/// The shape of a function's control-flow graph the structuring needs.
struct Shape {
    /// Reverse-postorder number of each block reachable from the entry.
    rpo: Vec<Option<u32>>,
    /// Immediate dominator.
    idom: Vec<Option<Block>>,
    /// Children in the dominator tree.
    children: Vec<Vec<Block>>,
    /// Forward edges into each block (counted per edge).
    forward_in: Vec<u32>,
    /// Whether a back edge enters the block.
    loop_header: Vec<bool>,
}

fn shape(func: &Func) -> Shape {
    let n = func.blocks.len();
    let succ = |b: usize| func.blocks[b].term.successors();
    // Postorder by an explicit depth-first search from the entry.
    let mut seen = vec![false; n];
    let mut post = Vec::with_capacity(n);
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    seen[0] = true;
    while let Some(&mut (b, ref mut i)) = stack.last_mut() {
        let s = succ(b);
        if *i < s.len() {
            let next = s[*i].0 as usize;
            *i += 1;
            if !seen[next] {
                seen[next] = true;
                stack.push((next, 0));
            }
        } else {
            post.push(b);
            stack.pop();
        }
    }
    let order: Vec<usize> = post.into_iter().rev().collect();
    let mut rpo = vec![None; n];
    for (i, &b) in order.iter().enumerate() {
        rpo[b] = Some(i as u32);
    }
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &b in &order {
        for s in succ(b) {
            preds[s.0 as usize].push(b);
        }
    }
    // Dominators (Cooper, Harvey and Kennedy), on reverse-postorder numbers.
    let mut idom: Vec<Option<usize>> = vec![None; n];
    idom[0] = Some(0);
    let intersect = |idom: &[Option<usize>], mut a: usize, mut b: usize| {
        while a != b {
            while rpo[a] > rpo[b] {
                a = idom[a].expect("processed");
            }
            while rpo[b] > rpo[a] {
                b = idom[b].expect("processed");
            }
        }
        a
    };
    let mut changed = true;
    while changed {
        changed = false;
        for &b in order.iter().skip(1) {
            let mut new = None;
            for &p in &preds[b] {
                if idom[p].is_some() {
                    new = Some(match new {
                        None => p,
                        Some(q) => intersect(&idom, p, q),
                    });
                }
            }
            if new.is_some() && idom[b] != new {
                idom[b] = new;
                changed = true;
            }
        }
    }
    let mut children = vec![Vec::new(); n];
    for &b in order.iter().skip(1) {
        children[idom[b].expect("reachable")].push(Block(b as u32));
    }
    let mut forward_in = vec![0; n];
    let mut loop_header = vec![false; n];
    for &b in &order {
        for s in succ(b) {
            let s = s.0 as usize;
            if rpo[s] > rpo[b] {
                forward_in[s] += 1;
            } else {
                loop_header[s] = true;
            }
        }
    }
    Shape {
        rpo,
        idom: idom
            .into_iter()
            .enumerate()
            .map(|(b, d)| d.filter(|&d| d != b).map(|d| Block(d as u32)))
            .collect(),
        children,
        forward_in,
        loop_header,
    }
}

/// What encloses the code being emitted, innermost last: the labels a branch can target.
#[derive(Clone, Copy, PartialEq)]
enum Frame {
    IfThenElse,
    /// A `loop` whose start is this block's code (branching to it continues the loop).
    LoopHeadedBy(Block),
    /// A `block` whose end is followed by this block's code.
    BlockFollowedBy(Block),
}

struct Gen<'f> {
    func: &'f Func,
    shape: Shape,
    out: Vec<u8>,
    frames: Vec<Frame>,
    /// A spare `i64` local.
    scratch: u32,
}

impl Gen<'_> {
    fn local(&self, v: VReg) -> u32 {
        4 + v.0
    }

    fn rpo(&self, b: Block) -> u32 {
        self.shape.rpo[b.0 as usize].expect("reachable")
    }

    fn is_merge(&self, b: Block) -> bool {
        self.shape.forward_in[b.0 as usize] >= 2
    }

    fn do_tree(&mut self, x: Block) -> Result<(), String> {
        let mut merges: Vec<Block> = self.shape.children[x.0 as usize]
            .iter()
            .copied()
            .filter(|&c| self.is_merge(c))
            .collect();
        merges.sort_by_key(|&c| std::cmp::Reverse(self.rpo(c)));
        if self.shape.loop_header[x.0 as usize] {
            self.out.extend_from_slice(&[0x03, EMPTY]); // loop
            self.frames.push(Frame::LoopHeadedBy(x));
            self.node_within(x, &merges)?;
            self.frames.pop();
            self.out.push(0x0B);
        } else {
            self.node_within(x, &merges)?;
        }
        Ok(())
    }

    fn node_within(&mut self, x: Block, merges: &[Block]) -> Result<(), String> {
        if let Some((&y, rest)) = merges.split_first() {
            self.out.extend_from_slice(&[0x02, EMPTY]); // block
            self.frames.push(Frame::BlockFollowedBy(y));
            self.node_within(x, rest)?;
            self.frames.pop();
            self.out.push(0x0B);
            return self.do_tree(y);
        }
        let data = &self.func.blocks[x.0 as usize];
        for inst in &data.insts {
            self.inst(inst)?;
        }
        match &data.term {
            Term::Jump(t) => self.branch(x, *t),
            Term::Branch { cond, then_, else_ } => {
                self.get(*cond);
                self.out.extend_from_slice(&[0x50, 0x45]); // i64.eqz; i32.eqz
                self.out.extend_from_slice(&[0x04, EMPTY]); // if
                self.frames.push(Frame::IfThenElse);
                self.branch(x, *then_)?;
                self.out.push(0x05); // else
                self.branch(x, *else_)?;
                self.frames.pop();
                self.out.push(0x0B);
                Ok(())
            }
            Term::Return(v) => {
                self.get(*v);
                self.out.push(0x0F);
                Ok(())
            }
            Term::ReturnNum(_) => Err("nguvu-wasm: kazi za moja kwa moja hazipo".into()),
        }
    }

    fn branch(&mut self, from: Block, to: Block) -> Result<(), String> {
        if self.rpo(to) <= self.rpo(from) || self.is_merge(to) {
            let depth = self
                .frames
                .iter()
                .rev()
                .position(
                    |f| matches!(f, Frame::LoopHeadedBy(b) | Frame::BlockFollowedBy(b) if *b == to),
                )
                .ok_or("nguvu-wasm: mtiririko usiopangika")?;
            self.out.push(0x0C); // br
            uleb(&mut self.out, depth as u32);
            Ok(())
        } else {
            debug_assert_eq!(self.shape.idom[to.0 as usize], Some(from));
            self.do_tree(to)
        }
    }

    // -- instructions -----------------------------------------------------------------------

    fn get(&mut self, v: VReg) {
        let l = self.local(v);
        self.out.push(0x20);
        uleb(&mut self.out, l);
    }

    fn set(&mut self, v: VReg) {
        let l = self.local(v);
        self.out.push(0x21);
        uleb(&mut self.out, l);
    }

    fn i64_const(&mut self, value: i64) {
        self.out.push(0x42);
        sleb(&mut self.out, value);
    }

    /// The address in register `base` plus `offset`, as `i32`, and the memory immediate's
    /// offset to use.
    fn address(&mut self, base: VReg, offset: i32) -> u32 {
        self.get(base);
        self.out.push(0xA7); // i32.wrap_i64
        if offset >= 0 {
            return offset as u32;
        }
        self.out.push(0x41); // i32.const
        sleb(&mut self.out, offset as i64);
        self.out.push(0x6A); // i32.add
        0
    }

    fn mem(&mut self, op: u8, offset: u32) {
        self.out.push(op);
        uleb(&mut self.out, 0); // alignment hint: none
        uleb(&mut self.out, offset);
    }

    /// `base + index * width`, as `i32`.
    fn indexed(&mut self, base: VReg, index: VReg, width: usize) {
        self.get(base);
        self.out.push(0xA7);
        self.get(index);
        self.out.push(0xA7);
        if width > 1 {
            self.out.push(0x41);
            sleb(&mut self.out, width.trailing_zeros() as i64);
            self.out.push(0x74); // i32.shl
        }
        self.out.push(0x6A); // i32.add
    }

    fn int_op(&mut self, op: IntOp) {
        self.out.push(match op {
            IntOp::Add => 0x7C,
            IntOp::Sub => 0x7D,
            IntOp::Mul => 0x7E,
            IntOp::SDiv => 0x7F,
            IntOp::UDiv => 0x80,
            IntOp::SRem => 0x81,
            IntOp::URem => 0x82,
            IntOp::And => 0x83,
            IntOp::Or => 0x84,
            IntOp::Xor => 0x85,
            IntOp::Shl => 0x86,
            IntOp::Sar => 0x87,
        });
    }

    fn icmp(&mut self, cond: ICond) {
        self.out.push(match cond {
            ICond::Eq => 0x51,
            ICond::Ne => 0x52,
            ICond::Lt => 0x53,
            ICond::Ult => 0x54,
            ICond::Gt => 0x55,
            ICond::Le => 0x57,
            ICond::Ule => 0x58,
            ICond::Ge => 0x59,
        });
        self.out.push(0xAD); // i64.extend_i32_u
    }

    fn inst(&mut self, inst: &Inst) -> Result<(), String> {
        match inst {
            Inst::IConst { dst, value } => {
                self.i64_const(*value);
                self.set(*dst);
            }
            Inst::FConst { dst, value } => {
                self.out.push(0x44);
                self.out.extend_from_slice(&value.to_le_bytes());
                self.set(*dst);
            }
            Inst::Mov { dst, src } => {
                self.get(*src);
                self.set(*dst);
            }
            Inst::Int { op, dst, a, b } => {
                self.get(*a);
                self.get(*b);
                self.int_op(*op);
                self.set(*dst);
            }
            Inst::IntImm { op, dst, a, imm } => {
                self.get(*a);
                self.i64_const(*imm as i64);
                self.int_op(*op);
                self.set(*dst);
            }
            Inst::Neg { dst, src } => {
                self.i64_const(0);
                self.get(*src);
                self.out.push(0x7D);
                self.set(*dst);
            }
            Inst::Not { dst, src } => {
                self.get(*src);
                self.i64_const(-1);
                self.out.push(0x85);
                self.set(*dst);
            }
            Inst::Popcnt { dst, src } => {
                self.get(*src);
                self.out.push(0x7B);
                self.set(*dst);
            }
            Inst::MulOverflow { dst, ovf, a, b } => {
                // The product first (`dst` may be an operand); then whether it overflowed:
                // `a == -1` overflows only for `i64::MIN`, `a == 0` never, otherwise exactly
                // when dividing the product by `a` does not give `b` back.
                self.get(*a);
                self.get(*b);
                self.out.push(0x7E);
                self.out.push(0x21);
                uleb(&mut self.out, self.scratch);
                self.get(*a);
                self.i64_const(-1);
                self.out.push(0x51);
                self.out.extend_from_slice(&[0x04, I64]);
                self.get(*b);
                self.i64_const(i64::MIN);
                self.out.extend_from_slice(&[0x51, 0xAD]);
                self.out.push(0x05);
                self.get(*a);
                self.out.push(0x50); // i64.eqz
                self.out.extend_from_slice(&[0x04, I64]);
                self.i64_const(0);
                self.out.push(0x05);
                self.out.push(0x20);
                uleb(&mut self.out, self.scratch);
                self.get(*a);
                self.out.push(0x7F);
                self.get(*b);
                self.out.extend_from_slice(&[0x52, 0xAD]);
                self.out.extend_from_slice(&[0x0B, 0x0B]);
                self.set(*ovf);
                self.out.push(0x20);
                uleb(&mut self.out, self.scratch);
                self.set(*dst);
            }
            Inst::Float { op, dst, a, b } => {
                self.get(*a);
                self.get(*b);
                self.out.push(match op {
                    FloatOp::Add => 0xA0,
                    FloatOp::Sub => 0xA1,
                    FloatOp::Mul => 0xA2,
                    FloatOp::Div => 0xA3,
                });
                self.set(*dst);
            }
            Inst::ICmp { cond, dst, a, b } => {
                self.get(*a);
                self.get(*b);
                self.icmp(*cond);
                self.set(*dst);
            }
            Inst::ICmpImm { cond, dst, a, imm } => {
                self.get(*a);
                self.i64_const(*imm as i64);
                self.icmp(*cond);
                self.set(*dst);
            }
            Inst::TestImm { zero, dst, a, imm } => {
                self.get(*a);
                self.i64_const(*imm as i64);
                self.out.push(0x83);
                self.out.push(0x50); // i64.eqz: (a & imm) == 0
                if !zero {
                    self.out.push(0x45); // i32.eqz
                }
                self.out.push(0xAD);
                self.set(*dst);
            }
            Inst::FCmp { cond, dst, a, b } => {
                self.get(*a);
                self.get(*b);
                self.out.push(match cond {
                    FCond::Oeq => 0x61,
                    FCond::Une => 0x62,
                    FCond::Olt => 0x63,
                    FCond::Ogt => 0x64,
                    FCond::Ole => 0x65,
                    FCond::Oge => 0x66,
                });
                self.out.push(0xAD);
                self.set(*dst);
            }
            Inst::Select { dst, cond, a, b } => {
                self.get(*a);
                self.get(*b);
                self.get(*cond);
                self.out.extend_from_slice(&[0x50, 0x45]); // != 0
                self.out.push(0x1B);
                self.set(*dst);
            }
            Inst::IntToFloat { dst, src } => {
                self.get(*src);
                self.out.push(0xB9); // f64.convert_i64_s
                self.set(*dst);
            }
            Inst::FloatToInt { dst, src } => {
                self.get(*src);
                self.out.extend_from_slice(&[0xFC, 0x06]); // i64.trunc_sat_f64_s
                self.set(*dst);
            }
            Inst::FloatBits { dst, src } => {
                self.get(*src);
                self.out.push(0xBD); // i64.reinterpret_f64
                self.set(*dst);
            }
            Inst::BitsFloat { dst, src } => {
                self.get(*src);
                self.out.push(0xBF); // f64.reinterpret_i64
                self.set(*dst);
            }
            Inst::Load { dst, base, offset } => {
                let at = self.address(*base, *offset);
                let op = match self.func.class(*dst) {
                    Class::Int => 0x29,
                    Class::Float => 0x2B,
                };
                self.mem(op, at);
                self.set(*dst);
            }
            Inst::Store { src, base, offset } => {
                let at = self.address(*base, *offset);
                self.get(*src);
                let op = match self.func.class(*src) {
                    Class::Int => 0x37,
                    Class::Float => 0x39,
                };
                self.mem(op, at);
            }
            Inst::LoadIndex {
                dst,
                base,
                index,
                kind,
            } => {
                self.indexed(*base, *index, kind.width());
                let op = match (self.func.class(*dst), kind) {
                    (Class::Float, _) => 0x2B,
                    (_, Kind::U8) => 0x31,
                    (_, Kind::I8) => 0x30,
                    (_, Kind::U16) => 0x33,
                    (_, Kind::I16) => 0x32,
                    (_, Kind::U32) => 0x35,
                    (_, Kind::I32) => 0x34,
                    (_, Kind::I64 | Kind::F64) => 0x29,
                };
                self.mem(op, 0);
                self.set(*dst);
            }
            Inst::StoreIndex {
                src,
                base,
                index,
                kind,
            } => {
                self.indexed(*base, *index, kind.width());
                self.get(*src);
                let op = match (self.func.class(*src), kind.width()) {
                    (Class::Float, _) => 0x39,
                    (_, 1) => 0x3C,
                    (_, 2) => 0x3D,
                    (_, 4) => 0x3E,
                    _ => 0x37,
                };
                self.mem(op, 0);
            }
            Inst::Call {
                table,
                target,
                args,
                dst,
                ret32: _,
            } => {
                let k = *target as u32;
                let (params, results) = runtime_signature(k);
                if params.len() != args.len() {
                    return Err(format!(
                        "nguvu-wasm: {target:?} inahitaji hoja {}",
                        params.len()
                    ));
                }
                for (&arg, &ty) in args.iter().zip(params) {
                    self.get(arg);
                    if ty == I32 {
                        self.out.push(0xA7);
                    }
                }
                // The entry's address in the runtime table (a 4-byte function pointer each).
                let at = self.address(*table, 0);
                self.mem(0x28, at + 4 * k); // i32.load
                self.out.push(0x11); // call_indirect
                uleb(&mut self.out, 1 + k);
                self.out.push(0x00);
                match (dst, results.first()) {
                    (Some(d), Some(&ty)) => {
                        if ty == I32 {
                            self.out.push(0xAD);
                        }
                        self.set(*d);
                    }
                    (None, Some(_)) => self.out.push(0x1A), // drop
                    (None, None) => {}
                    (Some(_), None) => {
                        return Err(format!("nguvu-wasm: {target:?} hairudishi thamani"))
                    }
                }
            }
            Inst::CallDirect { .. } | Inst::StackPointer { .. } | Inst::CallBuffer { .. } => {
                return Err("nguvu-wasm: kazi za moja kwa moja hazipo".into())
            }
        }
        Ok(())
    }
}

/// One function's body: locals, the structured code, `end`.
fn function(func: &Func) -> Result<Vec<u8>, String> {
    if func.blocks.is_empty() {
        return Err("nguvu-wasm: kazi tupu".into());
    }
    let regs = func.classes.len() as u32;
    let mut g = Gen {
        func,
        shape: shape(func),
        out: Vec::new(),
        frames: Vec::new(),
        scratch: 4 + regs,
    };
    // Locals after the four `i32` parameters: one per register, by class, then the scratch.
    let mut groups: Vec<(u32, u8)> = Vec::new();
    for c in func
        .classes
        .iter()
        .map(|c| match c {
            Class::Int => I64,
            Class::Float => F64,
        })
        .chain(std::iter::once(I64))
    {
        match groups.last_mut() {
            Some((n, t)) if *t == c => *n += 1,
            _ => groups.push((1, c)),
        }
    }
    uleb(&mut g.out, groups.len() as u32);
    for (n, t) in groups {
        uleb(&mut g.out, n);
        g.out.push(t);
    }
    // The parameters into their registers (`RT`, `HOST`, `FRAME`, `NUMS` are registers 0..4).
    for i in 0..4u32 {
        g.out.push(0x20);
        uleb(&mut g.out, i);
        g.out.push(0xAD);
        g.set(VReg(i));
    }
    g.do_tree(Block(0))?;
    g.out.extend_from_slice(&[0x00, 0x0B]); // unreachable; end
    Ok(g.out)
}

#[cfg(test)]
mod tests {
    use super::{sleb, uleb};

    #[test]
    fn leb128() {
        let enc = |v: i64| {
            let mut o = Vec::new();
            sleb(&mut o, v);
            o
        };
        assert_eq!(enc(0), [0x00]);
        assert_eq!(enc(-1), [0x7F]);
        assert_eq!(enc(63), [0x3F]);
        assert_eq!(enc(64), [0xC0, 0x00]);
        assert_eq!(enc(-64), [0x40]);
        assert_eq!(enc(-65), [0xBF, 0x7F]);
        let mut o = Vec::new();
        uleb(&mut o, 624_485);
        assert_eq!(o, [0xE5, 0x8E, 0x26]);
    }
}
