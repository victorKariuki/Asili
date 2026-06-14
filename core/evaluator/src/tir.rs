//! Typed IR and ASB emission.
//
// TODO(Phase II): lower_to_tir currently only records block/statement counts per function.
// A real TIR should lower each AST node to typed 3-address instructions:
//   - Type-annotated temporaries (t0: Namba, t1: Neno, ...)
//   - Explicit control-flow graph with BasicBlock edges
//   - Phi nodes for join points (if/else, loops)
//   - Explicit move/borrow/drop instructions to feed the borrow checker
// Until this exists, emit_asb_from_tir produces a text stub, not real bytecode.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use asili_diagnostics::Diagnostic;
use asili_parser::{Block, ImportPath, Module, Stmt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedIrModule {
    pub functions: Vec<TypedIrFunction>,
    pub imports: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedIrFunction {
    pub name: String,
    pub block_count: usize,
    pub stmt_count: usize,
}

/// One-pass fold: (block_count, stmt_count) for TIR.
fn block_counts(block: &Block) -> (usize, usize) {
    let mut blocks = 1usize;
    let mut stmts = block.statements.len();
    for stmt in &block.statements {
        match stmt {
            Stmt::If {
                then_block,
                else_if,
                else_block,
                ..
            } => {
                let (b, s) = block_counts(then_block);
                blocks += b;
                stmts += s;
                for (_, b) in else_if {
                    let (bb, ss) = block_counts(b);
                    blocks += bb;
                    stmts += ss;
                }
                if let Some(b) = else_block {
                    let (bb, ss) = block_counts(b);
                    blocks += bb;
                    stmts += ss;
                }
            }
            Stmt::While { body, .. } => {
                let (b, s) = block_counts(body);
                blocks += b;
                stmts += s;
            }
            Stmt::For { body, .. } => {
                let (b, s) = block_counts(body);
                blocks += b;
                stmts += s;
            }
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    let (b, s) = block_counts(&arm.body);
                    blocks += b;
                    stmts += s;
                }
            }
            _ => {}
        }
    }
    (blocks, stmts)
}

fn count_blocks(block: &Block) -> usize {
    block_counts(block).0
}

fn count_stmts(block: &Block) -> usize {
    block_counts(block).1
}

pub fn lower_to_tir(module: &Module) -> TypedIrModule {
    let functions = module
        .functions
        .iter()
        .map(|f| TypedIrFunction {
            name: f.name.clone(),
            block_count: count_blocks(&f.body),
            stmt_count: count_stmts(&f.body),
        })
        .collect();
    let imports: Vec<String> = module
        .imports
        .iter()
        .map(|i| match &i.path {
            ImportPath::Full(s) => s.clone(),
            ImportPath::Selective { module: m, names } => format!("{}::{{{}}}", m, names.join(", ")),
        })
        .collect();
    TypedIrModule { functions, imports }
}

// HACK: emit_asb_from_tir emits a human-readable text header ("ASB-STUB"), not binary bytecode.
// The .asb format is currently just AST metadata serialized as key=value lines.
// A real .asb should be a length-prefixed binary format (e.g. bincode of BytecodeProgram).
pub fn emit_asb_from_tir(tir: &TypedIrModule, source: &str) -> String {
    let mut h = DefaultHasher::new();
    source.hash(&mut h);
    let hash = h.finish();
    let symbols: Vec<String> = tir.functions.iter().map(|f| f.name.clone()).collect();
    let block_total: usize = tir.functions.iter().map(|f| f.block_count).sum();
    let stmt_total: usize = tir.functions.iter().map(|f| f.stmt_count).sum();
    format!(
        "ASB-STUB\nversion=2\ntir=typed\nmodule_hash={hash:016x}\nimports={}\nfunctions={}\nblocks={}\nstatements={}\nsymbols={}\n",
        tir.imports.len(),
        tir.functions.len(),
        block_total,
        stmt_total,
        symbols.join(",")
    )
}

pub fn validate_module(module: &Module) -> Result<(), Vec<Diagnostic>> {
    if module.functions.is_empty() {
        return Err(vec![Diagnostic::new("EVAL001", "moduli haina kazi yoyote")
            .with_stage("evaluator")]);
    }
    Ok(())
}
