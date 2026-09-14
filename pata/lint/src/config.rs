//! Linting configuration

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

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

    /// Load lint config from `pata.toml`'s `[lint.rules]` table under `project_root`, if the
    /// file/table exists. Mirrors `pata_fmt::config::FormatterConfig::load`'s shape — a project
    /// with no `pata.toml`, or one with no `[lint]`/`[lint.rules]` table, gets `LintConfig::new()`
    /// (all rules at default "warning" severity, no options), not an error.
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let toml_path = project_root.join("pata.toml");
        if !toml_path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&toml_path)?;
        let parsed: toml::Table = toml::from_str(&content)?;

        let rules_table = parsed
            .get("lint")
            .and_then(|v| v.as_table())
            .and_then(|lint| lint.get("rules"))
            .and_then(|v| v.as_table());

        match rules_table {
            Some(rules_table) => {
                let rules: BTreeMap<String, RuleConfig> =
                    toml::from_str(&toml::to_string(rules_table)?)?;
                Ok(Self { rules: Some(rules) })
            }
            None => Ok(Self::new()),
        }
    }

    /// Like `load`, but walks upward from `start` (a file or directory being linted) to find
    /// the nearest ancestor containing a `pata.toml`, rather than requiring the exact project
    /// root. Needed because `pata-lint` lints an arbitrary file/subdirectory
    /// (`pata-lint src/deep/nested.as`) that usually isn't the project root itself — `pata.toml`
    /// conventionally lives at the root, one or more directories up from what's being linted.
    /// This crate can't depend on `pata-cli`'s or `pata-lsp`'s own project-root walks (both
    /// live in crates that would create a circular dependency back onto `pata-lint`), so it
    /// gets its own minimal version. Returns `LintConfig::new()` (not an error) if no ancestor
    /// has a `pata.toml`.
    pub fn find_and_load(start: &Path) -> anyhow::Result<Self> {
        let start_dir = if start.is_dir() {
            start
        } else {
            start.parent().unwrap_or(start)
        };

        for dir in start_dir.ancestors() {
            if dir.join("pata.toml").exists() {
                return Self::load(dir);
            }
        }

        Ok(Self::new())
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
    fn load_returns_default_when_no_pata_toml() {
        let dir = std::env::temp_dir().join(format!("pata-lint-test-noconfig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = LintConfig::load(&dir).expect("load should not error on missing pata.toml");
        assert_eq!(cfg.severity("LINT101"), "warning");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_reads_lint_rules_table_from_pata_toml() {
        let dir = std::env::temp_dir().join(format!("pata-lint-test-withconfig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("pata.toml"),
            "[lint.rules.LINT101]\nseverity = \"ignore\"\n[lint.rules.LINT101.options]\nline_limit = 80\n",
        ).unwrap();

        let cfg = LintConfig::load(&dir).expect("load should parse a real [lint.rules] table");
        assert!(!cfg.is_enabled("LINT101"));
        assert_eq!(cfg.option_int("LINT101", "line_limit"), Some(80));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_and_load_walks_up_from_a_nested_file() {
        let root = std::env::temp_dir().join(format!("pata-lint-test-walkup-{}", std::process::id()));
        let nested = root.join("src").join("deep");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.join("pata.toml"),
            "[lint.rules.LINT101]\nseverity = \"ignore\"\n",
        ).unwrap();
        let target_file = nested.join("kuu.as");
        std::fs::write(&target_file, "kazi kuu() -> Tupu { rejesha Tupu }").unwrap();

        let cfg = LintConfig::find_and_load(&target_file).expect("walk-up load");
        assert!(!cfg.is_enabled("LINT101"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_and_load_defaults_when_no_ancestor_has_pata_toml() {
        let dir = std::env::temp_dir().join(format!("pata-lint-test-noancestor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = LintConfig::find_and_load(&dir).expect("walk-up load with no pata.toml above");
        assert_eq!(cfg.severity("LINT101"), "warning");
        std::fs::remove_dir_all(&dir).ok();
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
