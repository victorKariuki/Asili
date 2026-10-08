//! Expression and statement evaluation.

pub(crate) mod expr;
pub(crate) mod methods;
pub(crate) mod ops;
mod stmt;

use asili_parser::{Block, ExprId, Module, Param};

use crate::runtime::{Runtime, MAX_CALL_DEPTH};
use crate::value::{EvalError, EvalOut, Value};

/// Call a `kazi` or method body: `args` bound to `params` in a fresh scope (a missing argument is
/// `Hamna`). The tree-walker's one call path, and the one place its call depth is counted —
/// the same limit native code has. Deep nesting within a call needs no limit: every block and
/// expression grows the stack on demand.
pub(crate) fn call_body(
    rt: &mut Runtime<'_>,
    params: &[Param],
    args: impl IntoIterator<Item = Value>,
    body: &Block,
) -> Result<Value, EvalError> {
    if rt.calls >= MAX_CALL_DEPTH {
        return Err(EvalError::Unknown("undani mno".into()));
    }
    rt.calls += 1;
    rt.env.push_scope();
    let mut args = args.into_iter();
    for p in params {
        rt.env.define(&p.name, args.next().unwrap_or(Value::Hamna));
    }
    let out = eval_block_impl(body, rt);
    rt.env.pop_scope();
    rt.calls -= 1;
    match out? {
        EvalOut::Return(v) => Ok(v),
        _ => Ok(Value::Tupu),
    }
}

/// Evaluate a block (`depth` tracks nesting for telemetry; the stack grows on demand).
pub(crate) fn eval_block_impl(block: &Block, rt: &mut Runtime<'_>) -> Result<EvalOut, EvalError> {
    rt.depth += 1;
    rt.update_peak_depth();
    // Red zone widened from the original 32KB: debug builds (no inlining, full stack slots) have
    // much larger frames than release, and 32KB left too little margin before an actual stack
    // overflow on some nested-expression shapes (e.g. deep `Expr::Group` chains).
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
    expr: ExprId,
    env: &mut crate::env::Env,
    module: &Module,
) -> Result<crate::value::Value, EvalError> {
    let mut rt = Runtime::new(env, module);
    eval_expr_impl(expr, &mut rt)
}

pub(crate) fn eval_expr_impl(
    expr: ExprId,
    rt: &mut Runtime<'_>,
) -> Result<crate::value::Value, EvalError> {
    rt.count_expression();
    rt.depth += 1;
    rt.update_peak_depth();
    // See the matching comment in eval_block_impl above for why the red zone was widened.
    let result = stacker::maybe_grow(256 * 1024, 2 * 1024 * 1024, || {
        expr::eval_expr_inner(expr, rt)
    });
    rt.depth -= 1;
    result
}
