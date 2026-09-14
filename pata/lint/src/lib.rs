//! Asili linter for code style and best practices

pub mod rules;
pub mod visitor;
pub mod config;

use asili_diagnostics::Diagnostic;
use asili_parser::parse_tokens;
use asili_lexer::tokenize;
use config::LintConfig;

/// Lint a source file and return warnings/suggestions
/// Returns combined linting issues (parse errors + lint warnings)
///
/// Uses default settings for every rule (see `lint_source_with_config` to apply a project's
/// `[lint.rules]` `pata.toml` table — per-rule severity/options, including disabling a rule
/// entirely via `severity = "ignore"`).
pub fn lint_source(source: &str) -> Result<Vec<Diagnostic>, String> {
    lint_source_with_config(source, &LintConfig::new())
}

/// Lint a source file against an explicit `LintConfig`. A rule configured `severity = "ignore"`
/// is filtered out of the result entirely (not just downgraded) — there's no LSP-visible
/// severity tier below "info" to demote it to, and a linter option a user set to silence a rule
/// should actually silence it. LINT101's `line_limit` option (if set) overrides its default
/// 50-statement threshold.
pub fn lint_source_with_config(source: &str, config: &LintConfig) -> Result<Vec<Diagnostic>, String> {
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(diags) => return Err(format!("leksika imeshindwa: makosa {}", diags.len())),
    };

    let module = match parse_tokens(&tokens) {
        Ok(m) => m,
        Err(diags) => return Err(format!("uchanganuzi umeshindwa: makosa {}", diags.len())),
    };

    let mut lints = Vec::new();

    if config.is_enabled("LINT001") || config.is_enabled("LINT002") || config.is_enabled("LINT003") {
        lints.extend(rules::naming::check_naming_conventions(&module));
    }
    if config.is_enabled("LINT101") {
        let line_limit = config
            .option_int("LINT101", "line_limit")
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(rules::style::DEFAULT_LINE_LIMIT);
        lints.extend(rules::style::check_style_issues_with_limit(&module, line_limit));
    }
    if config.is_enabled("LINT201") || config.is_enabled("LINT202") || config.is_enabled("LINT203") {
        lints.extend(rules::best_practices::check_best_practices(&module, source));
    }
    if config.is_enabled("LINT301") {
        lints.extend(rules::logic::check_logic_errors(&module));
    }

    // Rule-level `is_enabled` above already skips whole check functions when every code they
    // produce is disabled, but naming/best_practices each emit multiple distinct codes from one
    // call — filter per-diagnostic too so e.g. disabling just LINT002 doesn't also suppress
    // LINT001/LINT003 from the same `check_naming_conventions` call.
    lints.retain(|d| config.is_enabled(d.code));

    Ok(lints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::RuleConfig;
    use std::collections::BTreeMap;

    fn long_function_source(statement_count: usize) -> String {
        let mut src = String::from("kazi ndefu() -> Tupu { ");
        for _ in 0..statement_count {
            src.push_str("weka x = 1\n");
        }
        src.push_str("rejesha Tupu }");
        src
    }

    #[test]
    fn default_config_flags_at_50_statement_threshold() {
        let src = long_function_source(51);
        let diags = lint_source(&src).expect("lint");
        assert!(diags.iter().any(|d| d.code == "LINT101"));
    }

    #[test]
    fn custom_line_limit_flags_earlier() {
        // 30 statements would pass the default 50-statement threshold, but a configured
        // line_limit of 20 should flag it — proves LintConfig's option actually reaches the rule.
        let src = long_function_source(30);
        let mut opts = BTreeMap::new();
        opts.insert("line_limit".to_string(), serde_json::json!(20));
        let mut rules = BTreeMap::new();
        rules.insert("LINT101".to_string(), RuleConfig { severity: None, options: Some(opts) });
        let config = LintConfig { rules: Some(rules) };

        let diags = lint_source_with_config(&src, &config).expect("lint");
        assert!(diags.iter().any(|d| d.code == "LINT101"));
    }

    #[test]
    fn ignored_rule_is_absent_from_output() {
        let src = long_function_source(51);
        let mut rules = BTreeMap::new();
        rules.insert(
            "LINT101".to_string(),
            RuleConfig { severity: Some("ignore".to_string()), options: None },
        );
        let config = LintConfig { rules: Some(rules) };

        let diags = lint_source_with_config(&src, &config).expect("lint");
        assert!(!diags.iter().any(|d| d.code == "LINT101"));
    }

    #[test]
    fn ignoring_one_naming_code_does_not_silence_its_siblings() {
        // LINT001 (bad function name) and LINT002 (bad struct name) both come from one
        // check_naming_conventions() call — ignoring LINT002 alone must not also suppress
        // LINT001, which this same source independently violates.
        let src = "umbo not_pascal { }\nkazi BadName() -> Tupu { rejesha Tupu }";
        let mut rules = BTreeMap::new();
        rules.insert(
            "LINT002".to_string(),
            RuleConfig { severity: Some("ignore".to_string()), options: None },
        );
        let config = LintConfig { rules: Some(rules) };

        let diags = lint_source_with_config(src, &config).expect("lint");
        assert!(!diags.iter().any(|d| d.code == "LINT002"));
        assert!(diags.iter().any(|d| d.code == "LINT001"));
    }
}
