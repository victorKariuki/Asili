//! `linganisha` and `weka (a, b) = e` patterns: the one definition of what a pattern matches
//! and binds, used by native code's host.

use asili_parser::{Expr, Name, Pattern};

use crate::value::{parse_number, EvalError, Value};

/// Whether `v` matches `pat`, calling `bind` for each name the pattern binds, in order (a later
/// binding of the same name wins). The single implementation of `linganisha` patterns: native
/// code's host binds into registers.
pub(crate) fn match_pattern(pat: &Pattern, v: &Value, bind: &mut dyn FnMut(Name, &Value)) -> bool {
    match pat {
        Pattern::Wildcard => true,
        Pattern::Literal(Expr::Number(s)) => {
            if let Value::Namba(n) = v {
                (parse_number(s) - n).abs() < f64::EPSILON
            } else {
                false
            }
        }
        Pattern::Literal(Expr::String(s)) => matches!(v, Value::Neno(x) if **x == **s),
        Pattern::Literal(Expr::Baiti(b)) => matches!(v, Value::Baiti(x) if **x == **b),
        Pattern::Literal(Expr::Bool(b)) => matches!(v, Value::Ukweli(x) if *x == *b),
        Pattern::Literal(Expr::Hamna) => matches!(v, Value::Hamna | Value::Chaguo(None)),
        Pattern::Literal(Expr::Char(c)) => matches!(v, Value::Herufi(x) if *x == *c),
        Pattern::Ident { name, .. } => {
            bind(*name, v);
            true
        }
        Pattern::Struct {
            struct_name,
            fields,
            ..
        } => {
            let Value::Struct(name, flds) = v else {
                return false;
            };
            if **name != **struct_name {
                return false;
            }
            for (fname, subpat) in fields {
                let Some((_, fval)) = flds.iter().find(|(n, _)| **n == **fname) else {
                    return false;
                };
                if !match_pattern(subpat, fval, bind) {
                    return false;
                }
            }
            true
        }
        Pattern::Jozi(p1, p2) => {
            let Value::Jozi(a, b) = v else {
                return false;
            };
            match_pattern(p1, a, bind) && match_pattern(p2, b, bind)
        }
        Pattern::Enum {
            enum_name,
            variant_name,
            data,
            ..
        } => {
            // `Tokeo`/`Chaguo` have two runtime shapes: `Value::Tokeo`/`Value::Chaguo` (from
            // builtins like `gawio`/`kamusi.pata`/casts) and `Value::Enum("Tokeo"/"Chaguo", ...)`
            // (from explicit `Tokeo::Sawa(x)`-style construction, which the parser registers as a
            // real jenum). A `Tokeo::Sawa(v)`/`Tokeo::Kosa(e)`/`Chaguo::Kuna(v)`/`Chaguo::Hamna`
            // pattern must match both shapes, or matching a builtin-produced Tokeo/Chaguo against
            // its own "constructor" pattern silently never fires (no error, arm just doesn't run).
            if enum_name == "Tokeo" {
                if let Value::Tokeo(res) = v {
                    return match (variant_name.as_str(), data, res) {
                        ("Sawa", Some(dpat), Ok(dval)) => match_pattern(dpat, dval, bind),
                        ("Sawa", None, Ok(_)) => true,
                        ("Kosa", Some(dpat), Err(dval)) => match_pattern(dpat, dval, bind),
                        ("Kosa", None, Err(_)) => true,
                        _ => false,
                    };
                }
            }
            if enum_name == "Chaguo" {
                if let Value::Chaguo(opt) = v {
                    return match (variant_name.as_str(), data, opt) {
                        ("Kuna", Some(dpat), Some(dval)) => match_pattern(dpat, dval, bind),
                        ("Kuna", None, Some(_)) => true,
                        ("Hamna", None, None) => true,
                        _ => false,
                    };
                }
            }
            let Value::Enum(en, vn, en_data) = v else {
                return false;
            };
            if en != enum_name || vn != variant_name {
                return false;
            }
            match (data, en_data) {
                (None, None) => true,
                (Some(dpat), Some(dval)) => match_pattern(dpat, dval, bind),
                _ => false,
            }
        }
        Pattern::Literal(_) => false,
    }
}

/// `weka <pattern> = e` where the value does not match the pattern.
pub(crate) fn let_pattern_mismatch() -> EvalError {
    EvalError::TypeErr("muundo wa weka haulingani na thamani".into())
}
