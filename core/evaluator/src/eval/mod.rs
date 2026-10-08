//! Expression and statement evaluation.

pub(crate) mod expr;
pub(crate) mod methods;
pub(crate) mod ops;
mod stmt;

use asili_parser::{Block, ExprId, Function, Module};

use crate::runtime::{Runtime, MAX_CALL_DEPTH};
use crate::value::{EvalError, EvalOut, Value};

/// Call a `kazi` or method body: `args` bound to `params` in a fresh scope (a missing argument is
/// `Hamna`). The tree-walker's one call path, and the one place its call depth is counted —
/// the same limit native code has. Deep nesting within a call needs no limit: every block and
/// expression grows the stack on demand.
pub(crate) fn call_body(
    rt: &mut Runtime<'_>,
    f: &Function,
    args: impl IntoIterator<Item = Value>,
) -> Result<Value, EvalError> {
    if rt.calls >= MAX_CALL_DEPTH {
        return Err(EvalError::Unknown("undani mno".into()));
    }
    let span = asili_trace::enter(&f.name, f.line as u32);
    rt.calls += 1;
    rt.env.push_scope();
    let mut args = args.into_iter();
    for p in &f.params {
        rt.env.define(&p.name, args.next().unwrap_or(Value::Hamna));
    }
    let out = eval_block_impl(&f.body, rt);
    rt.env.pop_scope();
    rt.calls -= 1;
    match out {
        // An error is traced once, by the `kazi` it first leaves.
        Err(e) => {
            if asili_trace::on() && !rt.error_traced {
                asili_trace::emit(asili_trace::Tukio::Kosa, &e.to_string(), f.line as u32);
                rt.error_traced = true;
            }
            drop(span);
            Err(e)
        }
        Ok(out) => {
            rt.error_traced = false;
            drop(span);
            Ok(match out {
                EvalOut::Return(v) => v,
                _ => Value::Tupu,
            })
        }
    }
}

/// Nesting levels (blocks and expressions) between stack checks: the 256 KiB red zone covers
/// this many levels many times over, even in debug builds, so checking at every level (which
/// cost ~7% of the tree-walker's time) is unnecessary.
const STACK_CHECK_EVERY: usize = 16;

/// Run `f` one nesting level deeper (`depth` also tracks nesting for telemetry), growing the
/// stack on demand.
#[inline(always)]
fn nested<R>(rt: &mut Runtime<'_>, f: impl FnOnce(&mut Runtime<'_>) -> R) -> R {
    rt.depth += 1;
    rt.update_peak_depth();
    let result = if rt.depth % STACK_CHECK_EVERY == 0 {
        stacker::maybe_grow(256 * 1024, 2 * 1024 * 1024, || f(rt))
    } else {
        f(rt)
    };
    rt.depth -= 1;
    result
}

/// Evaluate a block.
pub(crate) fn eval_block_impl(block: &Block, rt: &mut Runtime<'_>) -> Result<EvalOut, EvalError> {
    nested(rt, |rt| eval_block_inner(block, rt))
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

#[inline]
pub(crate) fn eval_expr_impl(
    expr: ExprId,
    rt: &mut Runtime<'_>,
) -> Result<crate::value::Value, EvalError> {
    rt.count_expression();
    nested(rt, |rt| expr::eval_expr_inner(expr, rt))
}
