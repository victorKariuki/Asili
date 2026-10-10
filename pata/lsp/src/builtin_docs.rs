//! Builtin functions and `umbo`s as the editor shows them — hover, signature help, completion —
//! all read from the one builtin table (`asili_parser::builtins::BUILTIN_MODULES`), the same one
//! the compiler checks calls against and `lib/std/*.asi` is generated from.

use asili_parser::builtins::{
    builtin_module_exports, builtin_struct, format_signature, BUILTIN_MODULES,
};
use asili_parser::FnContract;

/// The builtin function `name`: its module and contract.
pub fn function(name: &str) -> Option<(&'static str, FnContract)> {
    BUILTIN_MODULES.iter().find_map(|m| {
        builtin_module_exports(m.name)?
            .functions
            .remove(name)
            .map(|c| (m.name, c))
    })
}

/// Each parameter's label as the signature writes it (`chaguo?: ChaguoHttp`, `...hoja: T`).
pub fn param_labels(name: &str, c: &FnContract) -> Vec<String> {
    let sig = format_signature(name, c);
    let inner = &sig[sig.find('(').map_or(0, |i| i + 1)..sig.rfind(") ->").unwrap_or(sig.len())];
    asili_parser::split_generic_args(inner)
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// Hover markdown for a builtin function or `umbo` called `name`.
pub fn hover(name: &str) -> Option<String> {
    if let Some((module, c)) = function(name) {
        let mut text = format!("```asili\nkazi {}\n```", format_signature(name, &c));
        if !c.doc.is_empty() {
            text.push_str("\n\n");
            text.push_str(&c.doc);
        }
        text.push_str(&format!("\n\n_moduli: `{module}`_"));
        return Some(text);
    }
    let st = builtin_struct(name)?;
    let mut text = format!("```asili\numbo {} {{\n", st.name);
    for f in &st.fields {
        let opt = if f.optional { "?" } else { "" };
        text.push_str(&format!("    {}{opt}: {},\n", f.name, f.ty));
    }
    text.push_str("}\n```\n\n");
    text.push_str(st.doc);
    text.push('\n');
    for f in &st.fields {
        text.push_str(&format!("\n- `{}`: {}", f.name, f.doc));
    }
    text.push_str(&format!("\n\n_moduli: `{}`_", st.module));
    Some(text)
}
