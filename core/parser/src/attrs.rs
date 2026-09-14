//! Conditional compilation: `#[sharti(...)]` predicate parsing and evaluation.

use crate::Attribute;

/// The active compile target (e.g. "native", "wasm").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target(pub String);

impl Target {
    pub fn native() -> Self {
        Target("native".to_string())
    }
}

/// A parsed `#[sharti(...)]` predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShartiPredicate {
    /// `target = "a" | "b" | ...` — survives if the active target matches any listed value.
    Target(Vec<String>),
}

/// Parse the raw `Attribute.args` string of a `sharti` attribute into a predicate.
///
/// Grammar: `key = "value" ( '|' "value" )*`. Only `lengo` is a recognized key.
pub fn parse_sharti_predicate(args: &str) -> Result<ShartiPredicate, String> {
    let args = args.trim();
    let (key, rest) = args
        .split_once('=')
        .ok_or_else(|| format!("sharti predicate haijaeleweka: \"{args}\" (inahitaji key = \"thamani\")"))?;
    let key = key.trim();
    if key != "lengo" {
        return Err(format!("sharti key isiyojulikana: \"{key}\" (pekee \"lengo\" inatambulika)"));
    }

    let mut values = Vec::new();
    for part in rest.split('|') {
        let part = part.trim();
        let quoted = part
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .ok_or_else(|| format!("sharti thamani lazima iwe ndani ya alama za nukuu: \"{part}\""))?;
        if quoted.is_empty() {
            return Err("sharti thamani haiwezi kuwa tupu".to_string());
        }
        values.push(quoted.to_string());
    }
    if values.is_empty() {
        return Err("sharti inahitaji angalau thamani moja".to_string());
    }
    Ok(ShartiPredicate::Target(values))
}

/// Determine whether an item carrying `attrs` should be kept for `target`.
///
/// Items with no `sharti` attribute always survive. Multiple `sharti` attributes on the
/// same item are ANDed together (all predicates must allow the target).
pub fn item_survives(attrs: &[Attribute], target: &Target) -> Result<bool, String> {
    for attr in attrs {
        if attr.name != "sharti" {
            continue;
        }
        let raw = attr.args.as_deref().unwrap_or("");
        let predicate = parse_sharti_predicate(raw)?;
        let ShartiPredicate::Target(values) = predicate;
        if !values.iter().any(|v| v == &target.0) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_value() {
        let pred = parse_sharti_predicate("lengo = \"wasm\"").unwrap();
        assert_eq!(pred, ShartiPredicate::Target(vec!["wasm".to_string()]));
    }

    #[test]
    fn parses_or_values() {
        let pred = parse_sharti_predicate("lengo = \"wasm\" | \"native\"").unwrap();
        assert_eq!(
            pred,
            ShartiPredicate::Target(vec!["wasm".to_string(), "native".to_string()])
        );
    }

    #[test]
    fn rejects_unknown_key() {
        assert!(parse_sharti_predicate("bahati = \"x\"").is_err());
    }

    #[test]
    fn rejects_missing_equals() {
        assert!(parse_sharti_predicate("lengo \"wasm\"").is_err());
    }

    #[test]
    fn rejects_unquoted_value() {
        assert!(parse_sharti_predicate("lengo = wasm").is_err());
    }

    fn attr(name: &str, args: Option<&str>) -> Attribute {
        Attribute { name: name.to_string(), args: args.map(|s| s.to_string()), line: 1 }
    }

    #[test]
    fn item_with_no_sharti_always_survives() {
        let attrs = vec![attr("jaribio", None)];
        assert!(item_survives(&attrs, &Target::native()).unwrap());
        assert!(item_survives(&attrs, &Target("wasm".into())).unwrap());
    }

    #[test]
    fn item_gated_to_matching_target_survives() {
        let attrs = vec![attr("sharti", Some("lengo = \"wasm\""))];
        assert!(!item_survives(&attrs, &Target::native()).unwrap());
        assert!(item_survives(&attrs, &Target("wasm".into())).unwrap());
    }

    #[test]
    fn multiple_sharti_attrs_are_anded() {
        let attrs = vec![
            attr("sharti", Some("lengo = \"wasm\" | \"native\"")),
            attr("sharti", Some("lengo = \"wasm\"")),
        ];
        assert!(!item_survives(&attrs, &Target::native()).unwrap());
        assert!(item_survives(&attrs, &Target("wasm".into())).unwrap());
    }
}
