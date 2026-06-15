//! Hover content formatters for different symbol types.

use asili_parser::{Function, StructDecl, TraitDecl, ValueType};
use crate::types::format_type;

/// Format hover content for a function with its full signature.
pub fn format_function_hover(func: &Function, param_types: Vec<ValueType>, return_type: &ValueType) -> String {
    let params_str = func.params
        .iter()
        .zip(param_types.iter())
        .map(|(p, t)| format!("{}: {}", p.name, format_type(t)))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "**Function:** `kazi {}({}) -> {}`",
        func.name,
        params_str,
        format_type(return_type)
    )
}

/// Format hover content for a struct with its fields.
pub fn format_struct_hover(s: &StructDecl, field_types: Vec<ValueType>) -> String {
    let fields_str = s.fields
        .iter()
        .zip(field_types.iter())
        .map(|(f, t)| format!("  {}: {}", f.0, format_type(t)))
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        "**Struct:** `umbo {} {{\n{}\n}}`",
        s.name,
        fields_str
    )
}

/// Format hover content for a trait (without method list for now).
pub fn format_trait_hover(t: &TraitDecl) -> String {
    format!("**Trait:** `sifa {}`", t.name)
}

/// Format hover content for a variable with its type.
pub fn format_variable_hover(name: &str, type_: &ValueType) -> String {
    format!("**Variable:** `{}: {}`", name, format_type(type_))
}

/// Format hover content for a keyword.
pub fn format_keyword_hover(word: &str) -> String {
    format!("**Keyword:** `{}`", word)
}

/// Format hover content for a generic identifier.
pub fn format_identifier_hover(name: &str) -> String {
    format!("**Identifier:** `{}`", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_variable_hover() {
        let hover = format_variable_hover("count", &ValueType::Namba);
        assert!(hover.contains("count"));
        assert!(hover.contains("Namba"));
    }

    #[test]
    fn test_format_keyword_hover() {
        let hover = format_keyword_hover("kazi");
        assert!(hover.contains("kazi"));
        assert!(hover.contains("Keyword"));
    }
}
