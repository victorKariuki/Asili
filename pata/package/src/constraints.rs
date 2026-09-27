//! Semver constraint parsing and matching

use semver::{Version, VersionReq};
use std::fmt;

/// A version constraint expression (e.g., "^1.2", ">=1.0, <2.0")
#[derive(Clone, Debug)]
pub struct VersionConstraint {
    req: VersionReq,
    original: String,
}

impl VersionConstraint {
    /// Parse a semver constraint string
    pub fn parse(constraint: &str) -> anyhow::Result<Self> {
        let req = VersionReq::parse(constraint)?;
        Ok(VersionConstraint {
            req,
            original: constraint.to_string(),
        })
    }

    /// Check if a version satisfies this constraint
    pub fn matches(&self, version: &Version) -> bool {
        self.req.matches(version)
    }

    /// Get the original constraint string
    pub fn as_str(&self) -> &str {
        &self.original
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.original)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_caret() -> anyhow::Result<()> {
        let constraint = VersionConstraint::parse("^1.2.3")?;
        assert!(constraint.matches(&Version::parse("1.2.3")?));
        assert!(constraint.matches(&Version::parse("1.3.0")?));
        assert!(!constraint.matches(&Version::parse("2.0.0")?));
        Ok(())
    }

    #[test]
    fn parse_range() -> anyhow::Result<()> {
        let constraint = VersionConstraint::parse(">=1.0, <2.0")?;
        assert!(constraint.matches(&Version::parse("1.5.0")?));
        assert!(!constraint.matches(&Version::parse("2.0.0")?));
        Ok(())
    }

    #[test]
    fn parse_tilde() -> anyhow::Result<()> {
        let constraint = VersionConstraint::parse("~1.2.3")?;
        assert!(constraint.matches(&Version::parse("1.2.3")?));
        assert!(constraint.matches(&Version::parse("1.2.4")?));
        assert!(!constraint.matches(&Version::parse("1.3.0")?));
        Ok(())
    }
}
