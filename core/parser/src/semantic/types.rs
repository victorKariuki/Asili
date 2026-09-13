//! Type string parsing: split_generic_args and parse_value_type.
//
// TODO(Phase II): parse_value_type collapses Biti8/uBiti8/Biti32/Biti64/uBiti32/uBiti64 into
// ValueType::Namba, losing all fixed-width information. The type checker cannot distinguish
// Biti8 from Namba, so overflow and range errors are invisible at compile time.
// Fix: add ValueType::FixedInt(BitWidth, Signed) variants and propagate them through type checking.

use crate::ValueType;

/// Split a generic args string by comma at depth 0 (ignoring commas inside < >).
pub(crate) fn split_generic_args(s: &str) -> Vec<&str> {
    let s = s.trim();
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if start <= s.len() {
        out.push(s[start..].trim());
    }
    out
}

/// Check if a string is a type variable (single uppercase letter: T, E, U, etc.)
fn is_type_variable(s: &str) -> bool {
    s.len() == 1 && s.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Parse a type string (e.g. from .asi or AST) into ValueType. Public API for shared use.
pub fn parse_value_type(s: &str) -> ValueType {
    let s = s.replace(' ', "");
    if s == "Namba" || s.starts_with("Biti") || s.starts_with("uBiti") {
        return ValueType::Namba;
    }
    if s == "Neno" {
        return ValueType::Neno;
    }
    if s == "Ukweli" {
        return ValueType::Ukweli;
    }
    if s == "Tupu" {
        return ValueType::Tupu;
    }
    if let Some(rest) = s.strip_prefix("&mut") {
        return ValueType::Rejeo(Box::new(parse_value_type(rest)), true);
    }
    if let Some(rest) = s.strip_prefix('&') {
        return ValueType::Rejeo(Box::new(parse_value_type(rest)), false);
    }
    if s.starts_with("Rejeo_Tenda<") && s.ends_with('>') {
        return ValueType::Rejeo(Box::new(parse_value_type(&s[12..s.len() - 1])), true);
    }
    if s.starts_with("Rejeo<") && s.ends_with('>') {
        return ValueType::Rejeo(Box::new(parse_value_type(&s[6..s.len() - 1])), false);
    }
    if s.ends_with('?') {
        return ValueType::Chaguo(Box::new(parse_value_type(&s[..s.len() - 1])));
    }
    if s.starts_with("Chaguo<") && s.ends_with('>') {
        let inner = &s[7..s.len() - 1];
        return ValueType::Chaguo(Box::new(parse_value_type(inner)));
    }
    if s.starts_with("Tokeo<") && s.ends_with('>') {
        let inner = s[6..s.len() - 1].trim();
        let parts = split_generic_args(inner);
        if parts.len() >= 2 {
            return ValueType::Tokeo(
                Box::new(parse_value_type(parts[0])),
                Box::new(parse_value_type(parts[1])),
            );
        }
        return ValueType::Tokeo(Box::new(ValueType::Unknown), Box::new(ValueType::Unknown));
    }
    if s.starts_with("Orodha<") && s.ends_with('>') {
        let inner = s[7..s.len() - 1].trim();
        return ValueType::Orodha(Box::new(parse_value_type(inner)));
    }
    if s.starts_with("Kamusi<") && s.ends_with('>') {
        let inner = s[7..s.len() - 1].trim();
        let parts = split_generic_args(inner);
        if parts.len() >= 2 {
            return ValueType::Kamusi(
                Box::new(parse_value_type(parts[0])),
                Box::new(parse_value_type(parts[1])),
            );
        }
    }
    if s.starts_with("Mfululizo<") && s.ends_with('>') {
        let inner = s[10..s.len() - 1].trim();
        return ValueType::Mfululizo(Box::new(parse_value_type(inner)));
    }
    if s.starts_with("Jozi<") && s.ends_with('>') {
        let inner = s[5..s.len() - 1].trim();
        let parts = split_generic_args(inner);
        if parts.len() >= 2 {
            return ValueType::Jozi(
                Box::new(parse_value_type(parts[0])),
                Box::new(parse_value_type(parts[1])),
            );
        }
    }
    if s.starts_with("Seti<") && s.ends_with('>') {
        let inner = s[5..s.len() - 1].trim();
        return ValueType::Seti(Box::new(parse_value_type(inner)));
    }
    if s.starts_with("Kasha_GC<") && s.ends_with('>') {
        let inner = s[9..s.len() - 1].trim();
        return ValueType::KashaGC(Box::new(parse_value_type(inner)));
    }
    if s.starts_with("Kumbukumbu<") && s.ends_with('>') {
        let inner = s[11..s.len() - 1].trim();
        return ValueType::Kumbukumbu(Box::new(parse_value_type(inner)));
    }
    if s.starts_with("NjiaTx<") && s.ends_with('>') {
        let inner = s[7..s.len() - 1].trim();
        return ValueType::NjiaTx(Box::new(parse_value_type(inner)));
    }
    if s.starts_with("NjiaRx<") && s.ends_with('>') {
        let inner = s[7..s.len() - 1].trim();
        return ValueType::NjiaRx(Box::new(parse_value_type(inner)));
    }
    if s.starts_with("Fungo<") && s.ends_with('>') {
        let inner = s[6..s.len() - 1].trim();
        return ValueType::Fungo(Box::new(parse_value_type(inner)));
    }
    if s == "Faili" {
        return ValueType::Faili;
    }
    if s == "Mkondo" {
        return ValueType::Mkondo;
    }
    if s == "Herufi" {
        return ValueType::Herufi;
    }
    if s == "Namba_Kuu" {
        return ValueType::NambaKuu;
    }
    if s == "Namba_Sahihi" {
        return ValueType::NambaSahihi;
    }
    if s == "Wakati" {
        return ValueType::Wakati;
    }
    if s == "Anuani" {
        return ValueType::Anuani;
    }
    if is_type_variable(&s) {
        return ValueType::TypeVar(s.to_string());
    }
    ValueType::Unknown
}
