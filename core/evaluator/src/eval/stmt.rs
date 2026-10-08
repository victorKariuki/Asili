//! Statement evaluation.

use asili_parser::{AssignOp, BinaryOp, Expr, ForMode, Stmt};

use crate::runtime::Runtime;
use crate::signal;
use crate::value::{self, assign_f64_op, handle_loop_out, EvalError, EvalOut, LoopAction, Value};

use super::expr::match_and_bind_pattern;

pub(crate) fn eval_stmt_impl(stmt: &Stmt, rt: &mut Runtime<'_>) -> Result<EvalOut, EvalError> {
    rt.count_statement();

    let sig = signal::take_pending();
    if sig != 0 {
        if let Some(handler_name) = signal::get_handler(sig) {
            if let Some(f) = rt
                .module
                .functions
                .iter()
                .find(|x| x.name == handler_name)
                .cloned()
            {
                rt.env.push_scope();
                super::eval_block_impl(&f.body, rt)?;
                rt.env.pop_scope();
            }
        }
    }

    match stmt {
        Stmt::Let {
            name, value, line, ..
        } => {
            let v = super::eval_expr_impl(*value, rt)?;
            asili_trace::emit(asili_trace::Tukio::Kigeuzi, name, *line as u32);
            rt.env.define(*name, v);
            Ok(EvalOut::Next)
        }
        Stmt::LetPattern { pattern, value, .. } => {
            let value = super::eval_expr_impl(*value, rt)?;
            if !super::expr::match_and_bind_pattern(pattern, &value, rt) {
                return Err(let_pattern_mismatch());
            }
            Ok(EvalOut::Next)
        }
        Stmt::Assign {
            name, op, value, ..
        } => {
            // `x = x + e` with an `e` that cannot touch `x`: update `x` in place when that gives
            // the same value (appending to unshared text; see `ops::assign_in_place`).
            if let (
                AssignOp::Assign,
                Expr::Binary {
                    left,
                    op: bin @ BinaryOp::Add,
                    right,
                    ..
                },
            ) = (op, &rt.module[*value])
            {
                let module = rt.module;
                if matches!(&module[*left], Expr::Ident { name: n, .. } if n == name)
                    && inert(module, *right)
                    && rt.env.get_ref(*name).is_some()
                {
                    let rhs = super::eval_expr_impl(*right, rt)?;
                    let current = rt.env.get_mut(*name).expect("checked above");
                    if !super::ops::assign_in_place(bin, current, &rhs) {
                        *current = super::ops::binary_value(bin, current, &rhs)?;
                    }
                    return Ok(EvalOut::Next);
                }
            }
            let rhs = super::eval_expr_impl(*value, rt)?;
            // One lookup: the binding is updated in place (no copy of its old value).
            let current = rt
                .env
                .get_mut(*name)
                .ok_or_else(|| EvalError::UndefinedVar(name.to_string()))?;
            *current = match op {
                AssignOp::Assign => rhs,
                AssignOp::AddAssign => match (&*current, &rhs) {
                    (Value::Neno(s1), Value::Neno(s2)) => Value::Neno(value::concat_text(s1, s2)),
                    _ => assign_f64_op(current, &rhs, "+=", |a, b| a + b)?,
                },
                AssignOp::SubAssign => assign_f64_op(current, &rhs, "-=", |a, b| a - b)?,
                AssignOp::MulAssign => assign_f64_op(current, &rhs, "*=", |a, b| a * b)?,
                AssignOp::DivAssign => assign_f64_op(current, &rhs, "/=", |a, b| a / b)?,
            };
            Ok(EvalOut::Next)
        }
        Stmt::Expr { expr, .. } => {
            let _ = super::eval_expr_impl(*expr, rt)?;
            Ok(EvalOut::Next)
        }
        Stmt::Return { value, .. } => {
            let v = match value {
                Some(e) => super::eval_expr_impl(*e, rt)?,
                None => Value::Tupu,
            };
            Ok(EvalOut::Return(v))
        }
        Stmt::Drop { name, .. } => {
            if !rt.env.drop(*name) {
                return Err(EvalError::UndefinedVar(name.to_string()));
            }
            Ok(EvalOut::Next)
        }
        Stmt::If {
            cond,
            then_block,
            else_if,
            else_block,
            ..
        } => {
            let c = super::eval_expr_impl(*cond, rt)?;
            let run = super::ops::truthy(&c);
            if run {
                return super::eval_block_impl(then_block, rt);
            }
            for (c2, blk) in else_if {
                let c2val = super::eval_expr_impl(*c2, rt)?;
                if super::ops::truthy(&c2val) {
                    return super::eval_block_impl(blk, rt);
                }
            }
            if let Some(blk) = else_block {
                return super::eval_block_impl(blk, rt);
            }
            Ok(EvalOut::Next)
        }
        Stmt::While {
            label: my_label,
            cond,
            body,
            ..
        } => {
            loop {
                let c = super::eval_expr_impl(*cond, rt)?;
                if !super::ops::truthy(&c) {
                    break;
                }
                match handle_loop_out(my_label.as_ref(), super::eval_block_impl(body, rt)?) {
                    LoopAction::Continue => {}
                    LoopAction::Break => break,
                    LoopAction::Propagate(out) => return Ok(out),
                }
            }
            Ok(EvalOut::Next)
        }
        Stmt::For {
            label: my_label,
            var,
            mode,
            body,
            ..
        } => {
            match mode {
                ForMode::Range { start, end } => {
                    let s = super::eval_expr_impl(*start, rt)?;
                    let e = super::eval_expr_impl(*end, rt)?;
                    let start_n = value::as_f64(&s)
                        .ok_or_else(|| EvalError::TypeErr("kwa kutoka inahitaji Namba".into()))?;
                    let end_n = value::as_f64(&e)
                        .ok_or_else(|| EvalError::TypeErr("kwa hadi inahitaji Namba".into()))?;
                    let (start_i, end_i) = (start_n as i64, end_n as i64);
                    for i in start_i..end_i {
                        rt.env.push_scope();
                        rt.env.define(*var, Value::Namba(i as f64));
                        let out = super::eval_block_impl(body, rt)?;
                        rt.env.pop_scope();
                        match handle_loop_out(my_label.as_ref(), out) {
                            LoopAction::Continue => {}
                            LoopAction::Break => break,
                            LoopAction::Propagate(out) => return Ok(out),
                        }
                    }
                }
                ForMode::InExpr(expr) => {
                    let col = super::eval_expr_impl(*expr, rt)?;
                    // Move the items out when nothing else shares them; copy one at a time
                    // otherwise.
                    let items: Box<dyn Iterator<Item = Value>> =
                        match std::rc::Rc::try_unwrap(super::methods::iter_items(col)?) {
                            Ok(items) => Box::new(items.into_iter()),
                            Err(shared) => {
                                Box::new((0..shared.len()).map(move |i| shared[i].clone()))
                            }
                        };
                    for item in items {
                        rt.env.push_scope();
                        rt.env.define(*var, item);
                        let out = super::eval_block_impl(body, rt)?;
                        rt.env.pop_scope();
                        match handle_loop_out(my_label.as_ref(), out) {
                            LoopAction::Continue => {}
                            LoopAction::Break => break,
                            LoopAction::Propagate(out) => return Ok(out),
                        }
                    }
                }
            }
            Ok(EvalOut::Next)
        }
        Stmt::Match { expr, arms, .. } => {
            let v = super::eval_expr_impl(*expr, rt)?;
            for arm in arms {
                rt.env.push_scope();
                if match_and_bind_pattern(&arm.pattern, &v, rt) {
                    let out = super::eval_block_impl(&arm.body, rt);
                    rt.env.pop_scope();
                    return out;
                }
                rt.env.pop_scope();
            }
            // TODO: Non-exhaustive linganisha silently falls through to Ok(Next) instead of
            // panicking or emitting a compile-time exhaustiveness warning. The semantic analyzer
            // (SEM023) requires at least one arm, but does not check pattern coverage.
            Ok(EvalOut::Next)
        }
        Stmt::Break { label, .. } => Ok(EvalOut::Break(label.clone())),
        Stmt::Continue { label, .. } => Ok(EvalOut::Continue(label.clone())),
    }
}

/// Whether evaluating `id` reads at most names and constants (so it can neither change a
/// variable nor fail differently when evaluated before its neighbour).
fn inert(module: &asili_parser::Module, id: asili_parser::ExprId) -> bool {
    match &module[id] {
        Expr::String(_) | Expr::Number(_) | Expr::Char(_) | Expr::Bool(_) | Expr::Ident { .. } => {
            true
        }
        Expr::Group(e) | Expr::Cast { expr: e, .. } | Expr::FieldAccess { receiver: e, .. } => {
            inert(module, *e)
        }
        _ => false,
    }
}

/// `acha <pattern> = e` where the value does not match the pattern (every engine's error).
pub(crate) fn let_pattern_mismatch() -> EvalError {
    EvalError::TypeErr("muundo wa weka haulingani na thamani".into())
}
