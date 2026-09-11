//! Expression and statement evaluation.

mod expr;
mod stmt;

use asili_parser::{Block, Expr, Module};

use crate::runtime::{Runtime, MAX_EVAL_DEPTH};
use crate::value::{EvalError, EvalOut};

// TODO: depth is incremented on every block entry (if/while/for/match arms), not just function
// calls. This means MAX_EVAL_DEPTH is not a true recursion limit. See runtime.rs for details.
pub(crate) fn eval_block_impl(block: &Block, rt: &mut Runtime<'_>) -> Result<EvalOut, EvalError> {
    rt.depth += 1;
    rt.update_peak_depth();
    if rt.depth > MAX_EVAL_DEPTH {
        rt.depth -= 1;
        return Err(EvalError::Unknown("undani mno".into()));
    }
    // Red zone widened from the original 32KB: debug builds (no inlining, full stack slots) have
    // much larger per-call frames than release, and 32KB left too little margin before an actual
    // stack overflow could race ahead of the MAX_EVAL_DEPTH check on some nested-expression shapes
    // (e.g. deep `Expr::Group` chains) — confirmed by this exact recursion depth test overflowing
    // in `cargo test` (debug) while passing cleanly under `--release`.
    let result = stacker::maybe_grow(256 * 1024, 2 * 1024 * 1024, || eval_block_inner(block, rt));
    rt.depth -= 1;
    result
}

fn eval_block_inner(block: &Block, rt: &mut Runtime<'_>) -> Result<EvalOut, EvalError> {
    rt.env.push_scope();
    let result = eval_block_in_env(block, rt);
    rt.env.pop_scope();
    result
}

/// Run block in the current env without pushing a new scope. Used by REPL so that
/// `weka` bindings persist across lines.
pub(crate) fn eval_block_in_env(block: &Block, rt: &mut Runtime<'_>) -> Result<EvalOut, EvalError> {
    for stmt in &block.statements {
        // Dispatch any pending signal to registered kazi (mfumo.sikiliza_ishara).
        let sig_id = crate::signal::take_pending();
        if sig_id != 0 {
            if let Some(kazi_name) = crate::signal::get_handler(sig_id) {
                if let Some(f) = rt.module.functions.iter().find(|x| x.name == kazi_name) {
                    let out = eval_block_impl(&f.body, rt);
                    out?;
                }
            }
        }
        let out = match stmt::eval_stmt_impl(stmt, rt) {
            Err(EvalError::Propagate(v)) => return Ok(EvalOut::Return(v)),
            other => other?,
        };
        match out {
            EvalOut::Next => {}
            other => return Ok(other),
        }
    }
    Ok(EvalOut::Next)
}

pub fn eval_expr(
    expr: &Expr,
    env: &mut crate::env::Env,
    module: &Module,
) -> Result<crate::value::Value, EvalError> {
    let mut rt = Runtime::new(env, module);
    eval_expr_impl(expr, &mut rt)
}

pub(crate) fn eval_expr_impl(expr: &Expr, rt: &mut Runtime<'_>) -> Result<crate::value::Value, EvalError> {
    rt.depth += 1;
    rt.update_peak_depth();
    if rt.depth > MAX_EVAL_DEPTH {
        rt.depth -= 1;
        return Err(EvalError::Unknown("undani mno".into()));
    }
    // See the matching comment in eval_block_impl above for why the red zone was widened.
    let result = stacker::maybe_grow(256 * 1024, 2 * 1024 * 1024, || expr::eval_expr_inner(expr, rt));
    rt.depth -= 1;
    result
}
