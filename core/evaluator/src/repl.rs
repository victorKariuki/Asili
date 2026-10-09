//! The REPL as native code: a session keeps its bindings as values, and each line is compiled
//! into a function that takes them as parameters, runs the line, and returns its result with
//! every top-level binding's new value.

// Runtime code never panics on its own: an impossible state is an error the program sees
// (and its safe state handles), not a crash (see docs/design/safety-critical-roadmap.md §3).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

use asili_parser::{Block, Expr, Function, Module, Name, Param, Pattern, Stmt, TypeExpr};

use crate::value::{EvalError, Value};

/// The name of the function a line compiles into.
const LINE_FUNCTION: &str = "__repl__";
/// First item of what a line's function returns, so a `rejesha` inside the line (which returns
/// its own value instead) is told apart.
const MARK: &str = "\u{0}ingizo";

/// Bindings that persist from one line to the next.
#[derive(Default)]
pub struct ReplSession {
    bindings: Vec<(Name, Value)>,
}

impl ReplSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run one line, the body of `module`'s `__repl__` function as the REPL parsed it: the
    /// line's value (`Tupu` for statements), with the line's bindings kept for the next one.
    /// A single expression (or `rejesha e`) is the line's value.
    pub fn run(&mut self, module: &Module, body: &Block) -> Result<Value, EvalError> {
        let mut module = module.clone();
        let line = body.statements.first().map_or(1, Stmt::line);
        let mut statements = body.statements.clone();
        let result = match statements.as_slice() {
            [Stmt::Expr { expr, .. }]
            | [Stmt::Return {
                value: Some(expr), ..
            }] => {
                let e = *expr;
                statements.clear();
                Some(e)
            }
            _ => None,
        };
        // Names the line declares at its top level, after the session's own.
        let mut names: Vec<Name> = self.bindings.iter().map(|(n, _)| *n).collect();
        for stmt in &statements {
            let mut declared = Vec::new();
            match stmt {
                Stmt::Let { name, .. } => declared.push(*name),
                Stmt::LetPattern { pattern, .. } => pattern_names(pattern, &mut declared),
                _ => {}
            }
            for name in declared {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        let exprs = &mut module.exprs;
        let mut items = vec![exprs.add(Expr::String(MARK.to_string()))];
        items.push(match result {
            Some(e) => e,
            None => exprs.add(Expr::Bool(false)),
        });
        items.push(exprs.add(Expr::Bool(result.is_some())));
        for name in &names {
            items.push(exprs.add(Expr::Ident {
                name: *name,
                line,
                column: 0,
            }));
        }
        let list = exprs.add(Expr::List {
            elements: items,
            line,
        });
        statements.push(Stmt::Return {
            value: Some(list),
            line,
        });
        let params = self
            .bindings
            .iter()
            .map(|(name, value)| Param {
                name: *name,
                ty: TypeExpr {
                    name: param_type(value).to_string(),
                },
                line,
                column: 0,
            })
            .collect();
        let function = Function {
            name: Name::new(LINE_FUNCTION),
            params,
            return_type: TypeExpr {
                name: String::new(),
            },
            body: Block { statements },
            is_test: false,
            is_public: false,
            line,
            column: 0,
            attrs: Vec::new(),
        };
        module.functions.retain(|f| f.name != LINE_FUNCTION);
        module.functions.push(function);
        let program = crate::NativeProgram::build(&module).map_err(|e| {
            // Say why in the checker's words (an unknown name, a type error) where it has some.
            match asili_parser::semantic_check(&module) {
                Err(diagnostics) if !diagnostics.is_empty() => {
                    EvalError::Unknown(diagnostics[0].message.clone())
                }
                _ => e,
            }
        })?;
        let args = self.bindings.iter().map(|(_, v)| v.clone()).collect();
        let out = program.call(LINE_FUNCTION, args)?;
        let Value::Orodha(items) = &out else {
            return Ok(out);
        };
        let mut items = items.iter();
        if !matches!(items.next(), Some(Value::Neno(m)) if **m == *MARK) {
            return Ok(out); // the line's own `rejesha` of a list
        }
        let value = items.next().cloned().unwrap_or(Value::Tupu);
        let has_value = matches!(items.next(), Some(Value::Ukweli(true)));
        self.bindings = names.into_iter().zip(items.cloned()).collect();
        Ok(if has_value { value } else { Value::Tupu })
    }
}

/// The parameter type a binding's value is passed as: the numeric types in registers, text
/// with its methods known, everything else generic.
fn param_type(value: &Value) -> &'static str {
    match value {
        Value::Namba(_) => "Namba",
        Value::Ukweli(_) => "Ukweli",
        Value::Neno(_) => "Neno",
        _ => "",
    }
}

fn pattern_names(pattern: &Pattern, out: &mut Vec<Name>) {
    match pattern {
        Pattern::Ident { name, .. } => out.push(*name),
        Pattern::Jozi(a, b) => {
            pattern_names(a, out);
            pattern_names(b, out);
        }
        Pattern::Struct { fields, .. } => fields.iter().for_each(|(_, p)| pattern_names(p, out)),
        Pattern::Enum { data: Some(p), .. } => pattern_names(p, out),
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::Enum { data: None, .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::ReplSession;
    use crate::value::Value;

    fn line(session: &mut ReplSession, text: &str) -> Result<Value, crate::EvalError> {
        let source = format!("kazi __repl__() -> Tupu {{ {text} }}");
        let tokens = asili_lexer::tokenize(&source).expect("tokenize");
        let module = asili_parser::parse_tokens(&tokens).expect("parse");
        let body = module.functions[0].body.clone();
        session.run(&module, &body)
    }

    #[test]
    fn bindings_persist_between_lines() {
        if !crate::nguvu::supported() {
            return;
        }
        let mut s = ReplSession::new();
        assert_eq!(line(&mut s, "weka x = 5").unwrap(), Value::Tupu);
        assert_eq!(line(&mut s, "x * 2").unwrap(), Value::Namba(10.0));
        line(&mut s, "weka jina = \"Asili\"; x = x + 1").unwrap();
        assert_eq!(
            line(&mut s, "jina + \" \" + (x kama Neno)").unwrap(),
            Value::neno("Asili 6")
        );
        line(&mut s, "weka (a, b) = jozi(1, 2)").unwrap();
        assert_eq!(line(&mut s, "a + b + x").unwrap(), Value::Namba(9.0));
        assert!(line(&mut s, "haipo + 1").is_err());
        // A failed line keeps the session as it was.
        assert_eq!(line(&mut s, "x").unwrap(), Value::Namba(6.0));
    }
}
