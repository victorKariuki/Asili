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

    /// Like `load`, but walks upward from `start` (a file or directory being formatted) to find
    /// the nearest ancestor containing a `pata.toml`, rather than requiring the exact project
    /// root — `pata fmt` is usually pointed at a file or subdirectory below the real root.
    /// Returns `FormatterConfig::default()` (not an error) if no ancestor has a `pata.toml`.
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

        Ok(Self::default())
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

    /// The literal indent unit `canonical_format_with_indent` repeats per nesting level: a
    /// single `"\t"` when `indent_style = "tabs"` (width doesn't apply to a tab), otherwise
    /// `indent_width()` spaces.
    pub fn indent_unit(&self) -> String {
        if self.use_tabs() {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_width())
        }
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

    #[test]
    fn indent_unit_defaults_to_four_spaces() {
        assert_eq!(FormatterConfig::default().indent_unit(), "    ");
    }

    #[test]
    fn indent_unit_uses_tab_when_style_is_tabs() {
        let cfg = FormatterConfig {
            line_width: None,
            indent_style: Some("tabs".to_string()),
            indent_width: Some(2), // ignored when using tabs
        };
        assert_eq!(cfg.indent_unit(), "\t");
    }

    #[test]
    fn indent_unit_respects_custom_width() {
        let cfg = FormatterConfig {
            line_width: None,
            indent_style: Some("spaces".to_string()),
            indent_width: Some(2),
        };
        assert_eq!(cfg.indent_unit(), "  ");
    }

    #[test]
    fn load_returns_default_when_no_pata_toml() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = FormatterConfig::load(dir.path()).expect("load should not error on missing pata.toml");
        assert_eq!(cfg.indent_width(), 4);
        assert!(!cfg.use_tabs());
    }

    #[test]
    fn load_reads_fmt_table_from_pata_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pata.toml"),
            "[fmt]\nindent_style = \"tabs\"\nindent_width = 2\nline_width = 80\n",
        ).unwrap();

        let cfg = FormatterConfig::load(dir.path()).expect("load should parse a real [fmt] table");
        assert!(cfg.use_tabs());
        assert_eq!(cfg.indent_width(), 2);
        assert_eq!(cfg.line_width(), 80);
    }

    #[test]
    fn find_and_load_walks_up_from_a_nested_file() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("src").join("deep");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.path().join("pata.toml"),
            "[fmt]\nindent_style = \"tabs\"\n",
        ).unwrap();
        let target_file = nested.join("kuu.as");
        std::fs::write(&target_file, "kazi kuu() -> Tupu { rejesha Tupu }").unwrap();

        let cfg = FormatterConfig::find_and_load(&target_file).expect("walk-up load");
        assert!(cfg.use_tabs());
    }

    #[test]
    fn find_and_load_defaults_when_no_ancestor_has_pata_toml() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = FormatterConfig::find_and_load(dir.path()).expect("walk-up load with no pata.toml above");
        assert_eq!(cfg.indent_width(), 4);
    }
}
