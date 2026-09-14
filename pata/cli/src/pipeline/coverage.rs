//! Real line-level code coverage: aggregates `asili_evaluator::run_test_with_coverage`'s
//! per-test executed-line sets across every test in a project, compared against every real
//! statement line in the project's functions (walked recursively into nested blocks — `if`/
//! `while`/`for`/`match` arms — matching exactly what the evaluator's `eval_stmt_impl` actually
//! visits during execution, not just top-level statements).
//!
//! Replaces the previous `CoverageMetrics`, which took a static `executed: &[String]` function-
//! name list — real presence/absence of a function name, not anything about what code inside it
//! actually ran. Two tests exercising different branches of the same function both counted as
//! "covering" the whole function under that model; this one distinguishes them for real (see
//! `asili_evaluator`'s own `coverage_distinguishes_which_branch_actually_ran` test).

use asili_parser::{Block, Module, Stmt};
use std::collections::HashSet;

/// Real line-level coverage across a project: every statement line reachable in the module's
/// functions, and which of those lines at least one test actually executed.
#[derive(Debug, Clone)]
pub struct CoverageMetrics {
    /// Every real statement line in the project's functions (recursively, including nested
    /// blocks) — the coverage denominator.
    pub total_lines: HashSet<usize>,
    /// Lines executed by at least one test — the coverage numerator.
    pub executed_lines: HashSet<usize>,
    /// `executed_lines.len() / total_lines.len() * 100`, or 0 when there are no statements at
    /// all (an empty project has nothing to be "covered", not 100% covered).
    pub coverage_percent: f64,
}

impl CoverageMetrics {
    /// Format coverage report
    pub fn report(&self) -> String {
        format!(
            "Kuganda: mistari {}/{} ({:.1}%)",
            self.executed_lines.len(),
            self.total_lines.len(),
            self.coverage_percent
        )
    }
}

/// Every real statement line in `module`'s functions (recursively, including nested blocks) —
/// the coverage denominator `CoverageMetrics::new` uses, exposed standalone so a caller
/// aggregating coverage across several modules (e.g. `pata jaribu --chanjo`'s multi-file
/// project support) can sum totals without needing an executed-lines set for each one first.
pub fn total_statement_lines(module: &Module) -> HashSet<usize> {
    let mut total_lines = HashSet::new();
    for func in &module.functions {
        collect_statement_lines(&func.body, &mut total_lines);
    }
    total_lines
}

/// Recursively walk a block's statements (and every nested block reachable from `if`/`while`/
/// `for`/`match`), recording each one's line — matching exactly the set of statements
/// `eval_stmt_impl` visits during real execution, so `total_lines` isn't an overcount (dead
/// syntax the evaluator never reaches) or undercount (a nested block silently skipped).
fn collect_statement_lines(block: &Block, out: &mut HashSet<usize>) {
    for stmt in &block.statements {
        out.insert(stmt.line());
        match stmt {
            Stmt::If { then_block, else_if, else_block, .. } => {
                collect_statement_lines(then_block, out);
                for (_, b) in else_if {
                    collect_statement_lines(b, out);
                }
                if let Some(b) = else_block {
                    collect_statement_lines(b, out);
                }
            }
            Stmt::While { body, .. } => collect_statement_lines(body, out),
            Stmt::For { body, .. } => collect_statement_lines(body, out),
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    collect_statement_lines(&arm.body, out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asili_lexer::tokenize;
    use asili_parser::parse_tokens;

    fn parse(src: &str) -> Module {
        let tokens = tokenize(src).expect("tokenize");
        parse_tokens(&tokens).expect("parse")
    }

    /// Build a `CoverageMetrics` the same way real callers do (`compile.rs`'s
    /// `run_project_tests_with_coverage`): compute `total_lines` via `total_statement_lines`,
    /// filter `executed` down to real lines in this module, derive the percentage — kept here as
    /// a shared test helper rather than a `CoverageMetrics::new` production method, since real
    /// production code needs to aggregate `total_statement_lines` across *several* modules first
    /// (see compile.rs), which a single-module constructor can't express.
    fn metrics_for(module: &Module, executed: HashSet<usize>) -> CoverageMetrics {
        let total_lines = total_statement_lines(module);
        let executed_lines: HashSet<usize> =
            executed.into_iter().filter(|l| total_lines.contains(l)).collect();
        let coverage_percent = if total_lines.is_empty() {
            0.0
        } else {
            (executed_lines.len() as f64 / total_lines.len() as f64) * 100.0
        };
        CoverageMetrics { total_lines, executed_lines, coverage_percent }
    }

    #[test]
    fn total_lines_walks_into_nested_if_blocks() {
        let module = parse(
            "kazi f() -> Tupu {\nikiwa kweli {\nweka a = 1\n} vinginevyo {\nweka b = 2\n}\n}\n",
        );
        let metrics = metrics_for(&module, HashSet::new());
        // if-condition line, true-branch weka, false-branch weka = 3 real statement lines.
        assert_eq!(metrics.total_lines.len(), 3, "{:?}", metrics.total_lines);
    }

    #[test]
    fn coverage_percent_reflects_partial_execution() {
        let module = parse("kazi f() -> Tupu {\nweka a = 1\nweka b = 2\n}\n");
        let mut executed = HashSet::new();
        // Only one of the two statement lines executed.
        let total_lines_probe = metrics_for(&module, HashSet::new()).total_lines;
        let mut lines_iter = total_lines_probe.iter();
        executed.insert(*lines_iter.next().unwrap());

        let metrics = metrics_for(&module, executed);
        assert_eq!(metrics.coverage_percent, 50.0);
        assert!(metrics.coverage_percent >= 50.0);
        assert!(metrics.coverage_percent < 51.0);
    }

    #[test]
    fn empty_module_has_zero_percent_not_100() {
        let module = parse("kazi f() -> Tupu {\n}\n");
        let metrics = metrics_for(&module, HashSet::new());
        assert_eq!(metrics.total_lines.len(), 0);
        assert_eq!(metrics.coverage_percent, 0.0);
    }

    #[test]
    fn executed_lines_from_a_different_module_do_not_inflate_this_ones_coverage() {
        let module = parse("kazi f() -> Tupu {\nweka a = 1\n}\n");
        let mut foreign_lines = HashSet::new();
        foreign_lines.insert(9999); // not a real line in this module
        let metrics = metrics_for(&module, foreign_lines);
        assert_eq!(metrics.executed_lines.len(), 0, "a line from another module's execution must not count here");
    }
}
