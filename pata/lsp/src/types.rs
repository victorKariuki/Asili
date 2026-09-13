//! Semantic type information and symbol metadata for hover and analysis.

use asili_parser::ValueType;
use std::collections::HashMap;

/// Type information for a symbol at a specific scope depth.
#[derive(Clone, Debug)]
pub struct TypeInfo {
    pub value_type: ValueType,
    pub scope_depth: usize,
}

/// Kind of symbol (variable, function, struct, etc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Parameter,
    Function,
    Struct,
    Trait,
}

/// Complete information about a symbol for hover rendering.
#[derive(Clone, Debug)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub type_info: Option<TypeInfo>,
    pub declaration_line: usize,
}

/// A scope level in the scope stack (function, block, loop, etc).
#[derive(Clone, Debug)]
pub struct ScopeContext {
    /// Variables defined in this scope and their types.
    pub variables: HashMap<String, TypeInfo>,
    /// Parent function name (if this scope is inside a function).
    pub parent_fn: Option<String>,
    /// Nesting depth (0 = module level, 1 = inside function, 2+ = nested blocks).
    pub depth: usize,
}

impl ScopeContext {
    pub fn new(depth: usize, parent_fn: Option<String>) -> Self {
        ScopeContext {
            variables: HashMap::new(),
            parent_fn,
            depth,
        }
    }

    /// Insert a variable binding in this scope.
    pub fn bind(&mut self, name: String, value_type: ValueType) {
        self.variables.insert(
            name,
            TypeInfo {
                value_type,
                scope_depth: self.depth,
            },
        );
    }

    /// Look up a variable in this scope only (not parent scopes).
    pub fn get(&self, name: &str) -> Option<&TypeInfo> {
        self.variables.get(name)
    }
}

/// Result of resolving hover information for a symbol at a position.
#[derive(Clone, Debug)]
pub enum HoverInfo {
    /// A keyword like `kazi`, `ikiwa`, etc.
    Keyword(String),
    /// A variable or parameter with inferred type.
    Variable {
        name: String,
        type_: ValueType,
    },
    /// A function with its signature.
    Function {
        name: String,
        params: Vec<(String, ValueType)>,
        return_type: ValueType,
    },
    /// A struct with its fields.
    Struct {
        name: String,
        fields: Vec<(String, ValueType)>,
    },
    /// A trait with its methods.
    Trait {
        name: String,
        methods: Vec<(String, Vec<(String, ValueType)>, ValueType)>, // (name, params, return_type)
    },
    /// Generic identifier with no semantic info.
    Identifier(String),
}

impl HoverInfo {
    /// Render hover info to markdown string for LSP display.
    pub fn render(&self) -> String {
        match self {
            HoverInfo::Keyword(word) => {
                format!("**Keyword:** `{}`", word)
            }
            HoverInfo::Variable { name, type_ } => {
                format!("**Variable:** `{}: {}`", name, format_type(type_))
            }
            HoverInfo::Function {
                name,
                params,
                return_type,
            } => {
                let params_str = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, format_type(t)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "**Function:** `kazi {}({}) -> {}`",
                    name,
                    params_str,
                    format_type(return_type)
                )
            }
            HoverInfo::Struct { name, fields } => {
                let fields_str = fields
                    .iter()
                    .map(|(n, t)| format!("  {}: {}", n, format_type(t)))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("**Struct:** `umbo {} {{\n{}\n}}`", name, fields_str)
            }
            HoverInfo::Trait { name, methods } => {
                let methods_str = methods
                    .iter()
                    .map(|(n, params, ret)| {
                        let params_str = params
                            .iter()
                            .map(|(pn, pt)| format!("{}: {}", pn, format_type(pt)))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("  kazi {}({}) -> {}", n, params_str, format_type(ret))
                    })
                    .collect::<Vec<_>>()
                    .join(";\n");
                format!("**Trait:** `sifa {} {{\n{}\n}}`", name, methods_str)
            }
            HoverInfo::Identifier(name) => {
                format!("**Identifier:** `{}`", name)
            }
        }
    }
}

/// Format a ValueType for human-readable display.
pub fn format_type(t: &ValueType) -> String {
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
        ValueType::Anuani => "Anuani".to_string(),
        ValueType::Unknown => "Haijulikani".to_string(),
        ValueType::Chaguo(t) => format!("{}?", format_type(t)),
        ValueType::Tokeo(ok, err) => format!("Tokeo<{}, {}>", format_type(ok), format_type(err)),
        ValueType::Rejeo(t, mutable) => {
            if *mutable {
                format!("&mut {}", format_type(t))
            } else {
                format!("&{}", format_type(t))
            }
        }
        ValueType::Orodha(t) => format!("Orodha<{}>", format_type(t)),
        ValueType::Kamusi(k, v) => format!("Kamusi<{}, {}>", format_type(k), format_type(v)),
        ValueType::Mfululizo(t) => format!("Mfululizo<{}>", format_type(t)),
        ValueType::Jozi(a, b) => format!("Jozi<{}, {}>", format_type(a), format_type(b)),
        ValueType::Seti(t) => format!("Seti<{}>", format_type(t)),
        ValueType::KashaGC(t) => format!("Kasha_GC<{}>", format_type(t)),
        ValueType::Faili => "Faili".to_string(),
        ValueType::Mkondo => "Mkondo".to_string(),
        ValueType::Kumbukumbu(t) => format!("Kumbukumbu<{}>", format_type(t)),
        ValueType::NjiaTx(t) => format!("NjiaTx<{}>", format_type(t)),
        ValueType::NjiaRx(t) => format!("NjiaRx<{}>", format_type(t)),
        ValueType::Fungo(t) => format!("Fungo<{}>", format_type(t)),
        ValueType::Struct(name) => name.clone(),
        ValueType::TypeVar(name) => name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_types() {
        assert_eq!(format_type(&ValueType::Namba), "Namba");
        assert_eq!(format_type(&ValueType::Neno), "Neno");
        assert_eq!(format_type(&ValueType::Ukweli), "Ukweli");
    }

    #[test]
    fn test_format_generic_types() {
        let orodha_namba = ValueType::Orodha(Box::new(ValueType::Namba));
        assert_eq!(format_type(&orodha_namba), "Orodha<Namba>");

        let chaguo_neno = ValueType::Chaguo(Box::new(ValueType::Neno));
        assert_eq!(format_type(&chaguo_neno), "Neno?");
    }

    #[test]
    fn test_scope_context() {
        let mut scope = ScopeContext::new(1, Some("test_fn".to_string()));
        scope.bind("x".to_string(), ValueType::Namba);

        assert!(scope.get("x").is_some());
        assert!(scope.get("y").is_none());
    }

    #[test]
    fn test_hover_info_render() {
        let hover = HoverInfo::Variable {
            name: "count".to_string(),
            type_: ValueType::Namba,
        };
        assert!(hover.render().contains("count"));
        assert!(hover.render().contains("Namba"));
    }
}
