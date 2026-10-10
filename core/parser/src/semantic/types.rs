//! Type string parsing: split_generic_args and parse_value_type.
//
// TODO(Phase II): parse_value_type collapses Biti8/uBiti8/Biti32/Biti64/uBiti32/uBiti64 into
// ValueType::Namba, losing all fixed-width information. The type checker cannot distinguish
// Biti8 from Namba, so overflow and range errors are invisible at compile time.
// Fix: add ValueType::FixedInt(BitWidth, Signed) variants and propagate them through type checking.

use crate::ValueType;

/// Split a generic args string by comma at depth 0 (ignoring commas inside < >).
pub fn split_generic_args(s: &str) -> Vec<&str> {
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
    parse_value_type_with(s, &|_| None)
}

/// [`parse_value_type`], with `resolve` naming the types it does not know itself (a module's
/// `umbo`/`jenum` names), at any depth: `Orodha<Nukta>` is a list of `Nukta`.
pub fn parse_value_type_with(s: &str, resolve: &dyn Fn(&str) -> Option<ValueType>) -> ValueType {
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
        return ValueType::Rejeo(Box::new(parse_value_type_with(rest, resolve)), true);
    }
    if let Some(rest) = s.strip_prefix('&') {
        return ValueType::Rejeo(Box::new(parse_value_type_with(rest, resolve)), false);
    }
    if s.starts_with("Rejeo_Tenda<") && s.ends_with('>') {
        return ValueType::Rejeo(
            Box::new(parse_value_type_with(&s[12..s.len() - 1], resolve)),
            true,
        );
    }
    if s.starts_with("Rejeo<") && s.ends_with('>') {
        return ValueType::Rejeo(
            Box::new(parse_value_type_with(&s[6..s.len() - 1], resolve)),
            false,
        );
    }
    if s.ends_with('?') {
        return ValueType::Chaguo(Box::new(parse_value_type_with(&s[..s.len() - 1], resolve)));
    }
    if s.starts_with("Chaguo<") && s.ends_with('>') {
        let inner = &s[7..s.len() - 1];
        return ValueType::Chaguo(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("Tokeo<") && s.ends_with('>') {
        let inner = s[6..s.len() - 1].trim();
        let parts = split_generic_args(inner);
        if parts.len() >= 2 {
            return ValueType::Tokeo(
                Box::new(parse_value_type_with(parts[0], resolve)),
                Box::new(parse_value_type_with(parts[1], resolve)),
            );
        }
        return ValueType::Tokeo(Box::new(ValueType::Unknown), Box::new(ValueType::Unknown));
    }
    if s.starts_with("Orodha<") && s.ends_with('>') {
        let inner = s[7..s.len() - 1].trim();
        return ValueType::Orodha(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("Kamusi<") && s.ends_with('>') {
        let inner = s[7..s.len() - 1].trim();
        let parts = split_generic_args(inner);
        if parts.len() >= 2 {
            return ValueType::Kamusi(
                Box::new(parse_value_type_with(parts[0], resolve)),
                Box::new(parse_value_type_with(parts[1], resolve)),
            );
        }
    }
    if s.starts_with("Mfululizo<") && s.ends_with('>') {
        let inner = s[10..s.len() - 1].trim();
        return ValueType::Mfululizo(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("Jozi<") && s.ends_with('>') {
        let inner = s[5..s.len() - 1].trim();
        let parts = split_generic_args(inner);
        if parts.len() >= 2 {
            return ValueType::Jozi(
                Box::new(parse_value_type_with(parts[0], resolve)),
                Box::new(parse_value_type_with(parts[1], resolve)),
            );
        }
    }
    if s.starts_with("Seti<") && s.ends_with('>') {
        let inner = s[5..s.len() - 1].trim();
        return ValueType::Seti(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("Kasha_GC<") && s.ends_with('>') {
        let inner = s[9..s.len() - 1].trim();
        return ValueType::KashaGC(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("Kasha_GC_Dhaifu<") && s.ends_with('>') {
        let inner = s[16..s.len() - 1].trim();
        return ValueType::KashaGCDhaifu(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("Kumbukumbu<") && s.ends_with('>') {
        let inner = s[11..s.len() - 1].trim();
        return ValueType::Kumbukumbu(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("NjiaTx<") && s.ends_with('>') {
        let inner = s[7..s.len() - 1].trim();
        return ValueType::NjiaTx(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("NjiaRx<") && s.ends_with('>') {
        let inner = s[7..s.len() - 1].trim();
        return ValueType::NjiaRx(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("NjiaTxBounded<") && s.ends_with('>') {
        let inner = s[14..s.len() - 1].trim();
        return ValueType::NjiaTxBounded(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("NjiaRxBounded<") && s.ends_with('>') {
        let inner = s[14..s.len() - 1].trim();
        return ValueType::NjiaRxBounded(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("Ahadi<") && s.ends_with('>') {
        let inner = s[6..s.len() - 1].trim();
        return ValueType::Ahadi(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s.starts_with("Fungo<") && s.ends_with('>') {
        let inner = s[6..s.len() - 1].trim();
        return ValueType::Fungo(Box::new(parse_value_type_with(inner, resolve)));
    }
    if s == "Faili" {
        return ValueType::Faili;
    }
    if s == "Mkondo" {
        return ValueType::Mkondo;
    }
    if s == "MkondoSikilizaji" {
        return ValueType::MkondoSikilizaji;
    }
    if s == "MkondoUdp" {
        return ValueType::MkondoUdp;
    }
    if s == "TlsUsanidi" {
        return ValueType::TlsUsanidi;
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
    if s == "Baiti" {
        return ValueType::Baiti;
    }
    if s == "Anuani" {
        return ValueType::Anuani;
    }
    if is_type_variable(&s) {
        return ValueType::TypeVar(s.to_string());
    }
    resolve(&s).unwrap_or(ValueType::Unknown)
}

/// Asili source spelling of a type — the inverse of [`parse_value_type`] (an unknown type prints
/// as `Haijulikani`). Used by the LSP's hover/completion and by the `.asi` drift check.
pub fn format_value_type(t: &ValueType) -> String {
    match t {
        ValueType::Namba => "Namba".to_string(),
        ValueType::Neno => "Neno".to_string(),
        ValueType::Ukweli => "Ukweli".to_string(),
        ValueType::Tupu => "Tupu".to_string(),
        ValueType::Hamna => "Hamna".to_string(),
        ValueType::Herufi => "Herufi".to_string(),
        ValueType::NambaKuu => "Namba_Kuu".to_string(),
        ValueType::NambaSahihi => "Namba_Sahihi".to_string(),
        ValueType::Wakati => "Wakati".to_string(),
        ValueType::Baiti => "Baiti".to_string(),
        ValueType::Ahadi(t) => format!("Ahadi<{}>", format_value_type(t)),
        ValueType::Anuani => "Anuani".to_string(),
        ValueType::Unknown => "Haijulikani".to_string(),
        ValueType::Chaguo(t) => format!("{}?", format_value_type(t)),
        ValueType::Tokeo(ok, err) => format!(
            "Tokeo<{}, {}>",
            format_value_type(ok),
            format_value_type(err)
        ),
        ValueType::Rejeo(t, mutable) => {
            if *mutable {
                format!("&mut {}", format_value_type(t))
            } else {
                format!("&{}", format_value_type(t))
            }
        }
        ValueType::Orodha(t) => format!("Orodha<{}>", format_value_type(t)),
        ValueType::Kamusi(k, v) => {
            format!("Kamusi<{}, {}>", format_value_type(k), format_value_type(v))
        }
        ValueType::Mfululizo(t) => format!("Mfululizo<{}>", format_value_type(t)),
        ValueType::Jozi(a, b) => {
            format!("Jozi<{}, {}>", format_value_type(a), format_value_type(b))
        }
        ValueType::Seti(t) => format!("Seti<{}>", format_value_type(t)),
        ValueType::KashaGC(t) => format!("Kasha_GC<{}>", format_value_type(t)),
        ValueType::KashaGCDhaifu(t) => format!("Kasha_GC_Dhaifu<{}>", format_value_type(t)),
        ValueType::Faili => "Faili".to_string(),
        ValueType::Mkondo => "Mkondo".to_string(),
        ValueType::MkondoSikilizaji => "MkondoSikilizaji".to_string(),
        ValueType::MkondoUdp => "MkondoUdp".to_string(),
        ValueType::TlsUsanidi => "TlsUsanidi".to_string(),
        ValueType::Kumbukumbu(t) => format!("Kumbukumbu<{}>", format_value_type(t)),
        ValueType::NjiaTx(t) => format!("NjiaTx<{}>", format_value_type(t)),
        ValueType::NjiaRx(t) => format!("NjiaRx<{}>", format_value_type(t)),
        ValueType::NjiaTxBounded(t) => format!("NjiaTxBounded<{}>", format_value_type(t)),
        ValueType::NjiaRxBounded(t) => format!("NjiaRxBounded<{}>", format_value_type(t)),
        ValueType::Fungo(t) => format!("Fungo<{}>", format_value_type(t)),
        ValueType::Struct(name) => name.clone(),
        ValueType::TypeVar(name) => name.clone(),
    }
}
