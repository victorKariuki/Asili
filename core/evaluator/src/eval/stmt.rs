//! Statement evaluation.

use asili_parser::{AssignOp, ForMode, Stmt};

use crate::runtime::Runtime;
use crate::value::{self, assign_f64_op, handle_loop_out, EvalError, EvalOut, LoopAction, Value};
use crate::signal;

use super::expr::match_and_bind_pattern;

pub(crate) fn eval_stmt_impl(stmt: &Stmt, rt: &mut Runtime<'_>) -> Result<EvalOut, EvalError> {
    rt.record_line(stmt.line());

    if let Some(hook) = &rt.debug_hook {
        // Push a fresh bindings snapshot in before should_pause might block, so a real pause
        // always has up-to-date data ready for a `variables` DAP request.
        hook.record_bindings(crate::debug_hook::snapshot_bindings(rt.env));
        hook.should_pause(stmt.line());
    }

    let sig = signal::take_pending();
    if sig != 0 {
        if let Some(handler_name) = signal::get_handler(sig) {
            if let Some(f) = rt.module.functions.iter().find(|x| x.name == handler_name).cloned() {
                rt.env.push_scope();
                super::eval_block_impl(&f.body, rt)?;
                rt.env.pop_scope();
            }
        }
    }

    match stmt {
        Stmt::Let { name, value, .. } => {
            let v = super::eval_expr_impl(value, rt)?;
            rt.env.define(name, v);
            Ok(EvalOut::Next)
        }
        Stmt::Assign { name, op, value, .. } => {
            let rhs = super::eval_expr_impl(value, rt)?;
            let current = rt.env.get(name).ok_or_else(|| EvalError::UndefinedVar(name.clone()))?;
            let new_val = match op {
                AssignOp::Assign => rhs,
                AssignOp::AddAssign => {
                    match (value::as_string(&current), value::as_string(&rhs)) {
                        (Some(s1), Some(s2)) => Value::Neno(format!("{s1}{s2}")),
                        _ => assign_f64_op(&current, &rhs, "+=", |a, b| a + b)?,
                    }
                }
                AssignOp::SubAssign => assign_f64_op(&current, &rhs, "-=", |a, b| a - b)?,
                AssignOp::MulAssign => assign_f64_op(&current, &rhs, "*=", |a, b| a * b)?,
                AssignOp::DivAssign => assign_f64_op(&current, &rhs, "/=", |a, b| a / b)?,
            };
            rt.env.set(name, new_val);
            Ok(EvalOut::Next)
        }
        Stmt::Expr { expr, .. } => {
            let _ = super::eval_expr_impl(expr, rt)?;
            Ok(EvalOut::Next)
        }
        Stmt::Return { value, .. } => {
            let v = match value {
                Some(e) => super::eval_expr_impl(e, rt)?,
                None => Value::Tupu,
            };
            Ok(EvalOut::Return(v))
        }
        Stmt::Drop { name, .. } => {
            if !rt.env.drop(name) {
                return Err(EvalError::UndefinedVar(name.clone()));
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
            let c = super::eval_expr_impl(cond, rt)?;
            let run = matches!(&c, Value::Ukweli(true));
            if run {
                return super::eval_block_impl(then_block, rt);
            }
            for (c2, blk) in else_if {
                let c2val = super::eval_expr_impl(c2, rt)?;
                if matches!(&c2val, Value::Ukweli(true)) {
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
                let c = super::eval_expr_impl(cond, rt)?;
                if !matches!(&c, Value::Ukweli(true)) {
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
                    let s = super::eval_expr_impl(start, rt)?;
                    let e = super::eval_expr_impl(end, rt)?;
                    let start_n = value::as_f64(&s)
                        .ok_or_else(|| EvalError::TypeErr("kwa kutoka inahitaji Namba".into()))?;
                    let end_n = value::as_f64(&e)
                        .ok_or_else(|| EvalError::TypeErr("kwa hadi inahitaji Namba".into()))?;
                    let (start_i, end_i) = (start_n as i64, end_n as i64);
                    for i in start_i..end_i {
                        rt.env.push_scope();
                        rt.env.define(var, Value::Namba(i as f64));
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
                    let col = super::eval_expr_impl(expr, rt)?;
                    match col {
                        Value::Orodha(elems) => {
                            for item in elems {
                                rt.env.push_scope();
                                rt.env.define(var, item);
                                let out = super::eval_block_impl(body, rt)?;
                                rt.env.pop_scope();
                                match handle_loop_out(my_label.as_ref(), out) {
                                    LoopAction::Continue => {}
                                    LoopAction::Break => break,
                                    LoopAction::Propagate(out) => return Ok(out),
                                }
                            }
                        }
                        Value::Kamusi(map) => {
                            for (key, val) in map {
                                let pair = Value::Jozi(
                                    Box::new(key.to_value()),
                                    Box::new(val),
                                );
                                rt.env.push_scope();
                                rt.env.define(var, pair);
                                let out = super::eval_block_impl(body, rt)?;
                                rt.env.pop_scope();
                                match handle_loop_out(my_label.as_ref(), out) {
                                    LoopAction::Continue => {}
                                    LoopAction::Break => break,
                                    LoopAction::Propagate(out) => return Ok(out),
                                }
                            }
                        }
                        _ => {
                            return Err(EvalError::TypeErr(
                                "kwa...katika inashughulikia Orodha na Kamusi tu".to_string()
                            ));
                        }
                    }
                }
            }
            Ok(EvalOut::Next)
        }
        Stmt::Match { expr, arms, .. } => {
            let v = super::eval_expr_impl(expr, rt)?;
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
