//! Formatter configuration

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Formatter configuration (read from pata.toml [fmt] section)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatterConfig {
    /// Line length limit for wrapping (default: 100)
    pub line_width: Option<usize>,
    /// Indentation style: "spaces" or "tabs" (default: "spaces")
    pub indent_style: Option<String>,
    /// Indentation width (default: 4 for spaces)
    pub indent_width: Option<usize>,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            line_width: Some(100),
            indent_style: Some("spaces".to_string()),
            indent_width: Some(4),
        }
    }
}

impl FormatterConfig {
    /// Load formatter config from pata.toml if it exists
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let toml_path = project_root.join("pata.toml");
        if !toml_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&toml_path)?;
        let parsed: toml::Table = toml::from_str(&content)?;

        let fmt = parsed.get("fmt").and_then(|v| v.as_table());
        match fmt {
            Some(fmt_table) => {
                let config = toml::from_str::<Self>(&toml::to_string(fmt_table)?)?;
                Ok(config)
            }
            None => Ok(Self::default()),
        }
    }

    pub fn line_width(&self) -> usize {
        self.line_width.unwrap_or(100)
    }

    pub fn indent_width(&self) -> usize {
        self.indent_width.unwrap_or(4)
    }

    pub fn use_tabs(&self) -> bool {
        self.indent_style.as_deref() == Some("tabs")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let cfg = FormatterConfig::default();
        assert_eq!(cfg.line_width(), 100);
        assert_eq!(cfg.indent_width(), 4);
        assert!(!cfg.use_tabs());
    }

    #[test]
    fn custom_config() {
        let cfg = FormatterConfig {
            line_width: Some(80),
            indent_style: Some("tabs".to_string()),
            indent_width: Some(2),
        };
        assert_eq!(cfg.line_width(), 80);
        assert_eq!(cfg.indent_width(), 2);
        assert!(cfg.use_tabs());
    }
}
