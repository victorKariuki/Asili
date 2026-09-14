//! Linting configuration

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Per-rule lint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    /// Rules and their configuration (rule code -> config)
    pub rules: Option<BTreeMap<String, RuleConfig>>,
}

/// Individual rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// Severity level: "error", "warning", "info", "ignore"
    pub severity: Option<String>,
    /// Rule-specific options (e.g., line_limit for LINT101)
    pub options: Option<BTreeMap<String, serde_json::Value>>,
}

impl LintConfig {
    /// Create empty config
    pub fn new() -> Self {
        Self { rules: None }
    }

    /// Get severity for a rule (defaults to "warning")
    pub fn severity(&self, rule_code: &str) -> &str {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get(rule_code))
            .and_then(|cfg| cfg.severity.as_deref())
            .unwrap_or("warning")
    }

    /// Check if a rule is enabled (not "ignore")
    pub fn is_enabled(&self, rule_code: &str) -> bool {
        self.severity(rule_code) != "ignore"
    }

    /// Get option for a rule
    pub fn option(&self, rule_code: &str, key: &str) -> Option<&serde_json::Value> {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get(rule_code))
            .and_then(|cfg| cfg.options.as_ref())
            .and_then(|opts| opts.get(key))
    }

    /// Get integer option (useful for limits like line_limit)
    pub fn option_int(&self, rule_code: &str, key: &str) -> Option<i64> {
        self.option(rule_code, key).and_then(|v| v.as_i64())
    }
}

impl Default for LintConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_severity() {
        let cfg = LintConfig::new();
        assert_eq!(cfg.severity("LINT001"), "warning");
    }

    #[test]
    fn custom_severity() {
        let mut rules = BTreeMap::new();
        rules.insert(
            "LINT001".to_string(),
            RuleConfig {
                severity: Some("error".to_string()),
                options: None,
            },
        );
        let cfg = LintConfig { rules: Some(rules) };
        assert_eq!(cfg.severity("LINT001"), "error");
    }

    #[test]
    fn disabled_rule() {
        let mut rules = BTreeMap::new();
        rules.insert(
            "LINT101".to_string(),
            RuleConfig {
                severity: Some("ignore".to_string()),
                options: None,
            },
        );
        let cfg = LintConfig { rules: Some(rules) };
        assert!(!cfg.is_enabled("LINT101"));
    }

    #[test]
    fn rule_options() {
        let mut opts = BTreeMap::new();
        opts.insert("line_limit".to_string(), serde_json::json!(80));
        let mut rules = BTreeMap::new();
        rules.insert(
            "LINT101".to_string(),
            RuleConfig {
                severity: None,
                options: Some(opts),
            },
        );
        let cfg = LintConfig { rules: Some(rules) };
        assert_eq!(cfg.option_int("LINT101", "line_limit"), Some(80));
    }
}
