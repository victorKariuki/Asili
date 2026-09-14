//! Logic error lint rules: LINT301, unused local variables.
//!
//! Scope, deliberately conservative: only `weka`/`thabiti` (`Stmt::Let`) bindings are checked —
//! not `for`-loop variables or `match`-arm pattern bindings, which are a riskier case to flag
//! correctly (a `for` loop variable is very often intentionally unused when only iterating for
//! side effects, and match-arm bindings interact with pattern exhaustiveness in ways a simple
//! reference count doesn't capture). A name is flagged only when it never appears as an
//! `Expr::Ident` reference anywhere else in the same function body — a flat, function-scoped
//! check (not block-scoped), so shadowing across nested blocks isn't distinguished; the
//! consequence is a small false-negative bias (an outer `weka x` shadowed and then only the
//! inner `x` used won't be flagged) rather than false positives, matching this crate's existing
//! rules' bias (e.g. LINT201 was fixed toward under- not over-flagging).
//!
//! A name starting with `_` is never flagged — the established convention (mirrored from Rust)
//! for "intentionally unused," so a real API shape that must bind a value without using it
//! (e.g. matching a function signature) has an escape hatch.

use asili_diagnostics::Diagnostic;
use asili_parser::{Block, Expr, Function, Module, Stmt};
use std::collections::HashSet;

/// Check for logic errors: currently LINT301 (unused local variables) only.
pub fn check_logic_errors(module: &Module) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for func in &module.functions {
        diags.extend(check_unused_locals(func));
    }
    diags
}

fn check_unused_locals(func: &Function) -> Vec<Diagnostic> {
    let mut declared: Vec<(String, usize)> = Vec::new();
    collect_let_bindings(&func.body, &mut declared);
    if declared.is_empty() {
        return Vec::new();
    }

    let mut referenced: HashSet<String> = HashSet::new();
    collect_referenced_idents(&func.body, &mut referenced);

    let mut diags = Vec::new();
    for (name, line) in declared {
        if name.starts_with('_') {
            continue;
        }
        if !referenced.contains(&name) {
            diags.push(
                Diagnostic::new("LINT301", format!(
                    "kigezo '{}' hakijatumika popote kwenye kazi '{}'",
                    name, func.name
                ))
                .with_stage("ukaguzi")
                .with_span(line, 1),
            );
        }
    }
    diags
}

/// Collect every `weka`/`thabiti` binding's `(name, line)` in `block`, recursing into every
/// nested block (`if`/`while`/`for`/`match` bodies) — matches what actually executes, same
/// convention as `pata_cli::pipeline::coverage`'s statement walker.
fn collect_let_bindings(block: &Block, out: &mut Vec<(String, usize)>) {
    for stmt in &block.statements {
        if let Stmt::Let { name, line, .. } = stmt {
            out.push((name.clone(), *line));
        }
        recurse_into_nested_blocks(stmt, &mut |b| collect_let_bindings(b, out));
    }
}

/// Collect every name referenced via `Expr::Ident` anywhere in `block`, including inside nested
/// blocks and every expression position (call args, binary operands, struct-literal field
/// values, etc.) — an exhaustive walk over every `Expr` variant, mirroring
/// `pata-lsp`'s `SemanticAnalyzer::scan_expr` (kept as an independent copy rather than a shared
/// dependency: `pata-lint` has no reason to depend on `pata-lsp`, and this walk is simple enough
/// that duplicating it is cheaper than the coupling).
fn collect_referenced_idents(block: &Block, out: &mut HashSet<String>) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { value, .. } => scan_expr(value, out),
            Stmt::Assign { value, .. } => scan_expr(value, out),
            Stmt::If { cond, .. } => scan_expr(cond, out),
            Stmt::While { cond, .. } => scan_expr(cond, out),
            Stmt::For { mode, .. } => {
                if let asili_parser::ForMode::InExpr(e) = mode {
                    scan_expr(e, out);
                }
            }
            Stmt::Match { expr, arms, .. } => {
                scan_expr(expr, out);
                for arm in arms {
                    scan_pattern(&arm.pattern, out);
                }
            }
            Stmt::Return { value: Some(e), .. } => scan_expr(e, out),
            Stmt::Expr { expr, .. } => scan_expr(expr, out),
            _ => {}
        }
        recurse_into_nested_blocks(stmt, &mut |b| collect_referenced_idents(b, out));
    }
}

/// Call `f` on every nested `Block` directly inside `stmt` (an `if`'s then/else-if/else bodies,
/// a `while`/`for`'s body, a `match`'s arm bodies) — the single place both walkers above share
/// their "which blocks are nested here" knowledge, so adding a new block-bearing `Stmt` variant
/// only needs updating here, not in two diverging copies.
fn recurse_into_nested_blocks(stmt: &Stmt, f: &mut dyn FnMut(&Block)) {
    match stmt {
        Stmt::If { then_block, else_if, else_block, .. } => {
            f(then_block);
            for (_, b) in else_if {
                f(b);
            }
            if let Some(b) = else_block {
                f(b);
            }
        }
        Stmt::While { body, .. } => f(body),
        Stmt::For { body, .. } => f(body),
        Stmt::Match { arms, .. } => {
            for arm in arms {
                f(&arm.body);
            }
        }
        _ => {}
    }
}

/// A pattern can itself reference a value in a `Literal(Expr)` arm (e.g. matching against a
/// constant expression) — scanned for completeness, though the common case (binding patterns)
/// introduces names rather than referencing them, so most pattern shapes contribute nothing here.
fn scan_pattern(pattern: &asili_parser::Pattern, out: &mut HashSet<String>) {
    use asili_parser::Pattern;
    match pattern {
        Pattern::Literal(e) => scan_expr(e, out),
        Pattern::Struct { fields, .. } => {
            for (_, p) in fields {
                scan_pattern(p, out);
            }
        }
        Pattern::Enum { data: Some(p), .. } => scan_pattern(p, out),
        Pattern::Jozi(a, b) => {
            scan_pattern(a, out);
            scan_pattern(b, out);
        }
        _ => {}
    }
}

fn scan_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::Ident { name, .. } => {
            out.insert(name.clone());
        }
        Expr::Group(e) | Expr::Unary { expr: e, .. } | Expr::Cast { expr: e, .. } | Expr::Propagate { expr: e, .. } => {
            scan_expr(e, out);
        }
        Expr::Binary { left, right, .. } => {
            scan_expr(left, out);
            scan_expr(right, out);
        }
        Expr::Call { callee, args, .. } => {
            scan_expr(callee, out);
            for a in args {
                scan_expr(a, out);
            }
        }
        Expr::MethodCall { receiver, args, .. } => {
            scan_expr(receiver, out);
            for a in args {
                scan_expr(a, out);
            }
        }
        Expr::List { elements, .. } => {
            for e in elements {
                scan_expr(e, out);
            }
        }
        Expr::Map { entries, .. } => {
            for (k, v) in entries {
                scan_expr(k, out);
                scan_expr(v, out);
            }
        }
        Expr::StructLiteral { fields, .. } => {
            for (_, e) in fields {
                scan_expr(e, out);
            }
        }
        Expr::EnumConstruct { data: Some(e), .. } => scan_expr(e, out),
        Expr::FieldAccess { receiver, .. } => scan_expr(receiver, out),
        Expr::Index { base, index, .. } => {
            scan_expr(base, out);
            scan_expr(index, out);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asili_lexer::tokenize;
    use asili_parser::parse_tokens;

    fn lint(src: &str) -> Vec<Diagnostic> {
        let tokens = tokenize(src).expect("tokenize");
        let module = parse_tokens(&tokens).expect("parse");
        check_logic_errors(&module)
    }

    #[test]
    fn flags_a_never_referenced_binding() {
        let src = "kazi f() -> Tupu {\nweka bila_matumizi = 1\nrejesha Tupu\n}";
        let diags = lint(src);
        assert!(diags.iter().any(|d| d.code == "LINT301" && d.message.contains("bila_matumizi")));
    }

    #[test]
    fn accepts_a_binding_used_in_a_later_expression() {
        let src = "kazi f() -> Namba {\nweka x = 1\nrejesha x + 1\n}";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT301"));
    }

    #[test]
    fn accepts_a_binding_used_as_a_call_argument() {
        let src = "kazi f() -> Tupu {\nweka jina = \"x\"\nchapisha(jina)\n}";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT301"));
    }

    #[test]
    fn accepts_underscore_prefixed_names_as_intentionally_unused() {
        let src = "kazi f() -> Tupu {\nweka _haitumiki = 1\nrejesha Tupu\n}";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT301"));
    }

    #[test]
    fn accepts_a_binding_used_only_inside_a_nested_if_block() {
        let src = "kazi f() -> Tupu {\nweka x = 1\nikiwa kweli {\nchapisha(x)\n}\nrejesha Tupu\n}";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT301"), "a binding used inside a nested block must not be flagged");
    }

    #[test]
    fn flags_only_the_unused_one_among_several_bindings() {
        let src = "kazi f() -> Namba {\nweka a = 1\nweka haitumiki = 2\nrejesha a\n}";
        let diags = lint(src);
        let lint301: Vec<_> = diags.iter().filter(|d| d.code == "LINT301").collect();
        assert_eq!(lint301.len(), 1);
        assert!(lint301[0].message.contains("haitumiki"));
    }

    #[test]
    fn accepts_a_binding_used_only_in_reassignment_rhs() {
        // `x` is used on the right-hand side of an assignment to another variable -- a real use.
        let src = "kazi f() -> Tupu {\nweka x = 1\nweka y = 0\ny = x\nchapisha(\"\" + y)\nrejesha Tupu\n}";
        let diags = lint(src);
        assert!(!diags.iter().any(|d| d.code == "LINT301" && d.message.contains("'x'")));
    }
}
