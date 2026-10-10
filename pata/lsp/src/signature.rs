//! Signature help: parameter hints while typing a call, e.g. `jumlisha(|` showing
//! `a: Namba, b: Namba`.

use crate::symbols::parse_module;
use crate::workspace::WorkspaceIndex;
use asili_parser::extern_env_from_imports;

pub struct SignatureInfo {
    /// The full rendered signature, e.g. `kazi jumlisha(a: Namba, b: Namba) -> Namba`.
    pub label: String,
    /// Each parameter's own label, e.g. `["a: Namba", "b: Namba"]` — index-aligned with
    /// `active_param`, and with `label`'s own `(...)` section (so the client can highlight the
    /// active one within `label` without re-deriving offsets).
    pub params: Vec<String>,
    pub active_param: usize,
    /// What the function does, when known (builtins carry a description).
    pub doc: Option<String>,
}

/// Find the call the cursor is currently inside: `(callee_name, active_param_index)`, or None
/// if the cursor isn't inside any call's parens.
///
/// Computed by scanning the raw source text forward from the start up to the cursor, matching
/// parens and skipping string/char/comment content, rather than from the AST: `Expr::Call`
/// doesn't track its opening paren's position (only the closing one's line), so a structural
/// lookup would need yet another position-plumbing pass for comparatively little gain over
/// this — same tradeoff as `symbols::folding_ranges`, which uses the identical technique for
/// `{ }` instead of `( )`.
fn active_call_at(source: &str, line_0: u32, char_0: u32) -> Option<(String, usize)> {
    let chars: Vec<char> = source.chars().collect();
    // One (paren_char_index, comma_count_seen_at_this_depth) per currently-open '('.
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for c in crate::scan::code_chars(&chars) {
        if c.line > line_0 || (c.line == line_0 && c.col >= char_0) {
            break;
        }
        match c.ch {
            '(' => stack.push((c.index, 0)),
            ')' => {
                stack.pop();
            }
            ',' => {
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
            }
            _ => {}
        }
    }

    let (paren_idx, comma_count) = stack.last().copied()?;

    // Walk backward from the open paren to the identifier immediately before it (skipping
    // whitespace) — that's the callee name. No identifier there (e.g. `(1 + 2)`, a grouping
    // paren, not a call) means there's nothing to show.
    let mut end = paren_idx;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    if start == end {
        return None;
    }
    let name: String = chars[start..end].iter().collect();
    Some((name, comma_count))
}

/// `workspace` is the project's resolved cross-file index (`crate::workspace`), when
/// available — lets signature help work for calls into your own other project files, not just
/// the current one.
pub fn compute_signature_help(
    source: &str,
    line_0: u32,
    char_0: u32,
    workspace: Option<&WorkspaceIndex>,
) -> Option<SignatureInfo> {
    let (name, active_param) = active_call_at(source, line_0, char_0)?;
    let module = parse_module(source)?;

    // 1. A function in the current file — real parameter names.
    if let Some(f) = module.functions.iter().find(|f| f.name == name) {
        return Some(signature_from_params(
            &name,
            f.params
                .iter()
                .map(|p| (p.name.as_str(), p.ty.name.as_str())),
            &f.return_type.name,
            active_param,
        ));
    }

    // 2. A public function in a resolved cross-file (project-local) module — still real names.
    if let Some(ws) = workspace {
        for wm in ws.modules.values() {
            if let Some(f) = wm
                .module
                .functions
                .iter()
                .find(|f| f.name == name && f.is_public)
            {
                return Some(signature_from_params(
                    &name,
                    f.params
                        .iter()
                        .map(|p| (p.name.as_str(), p.ty.name.as_str())),
                    &f.return_type.name,
                    active_param,
                ));
            }
        }
    }

    // 3. A builtin: names, optional/variadic parameters and a description from its contract.
    let (extern_fns, _) = extern_env_from_imports(&module);
    if let Some(contract) = extern_fns.get(&name) {
        let params = crate::builtin_docs::param_labels(&name, contract);
        // Every argument past the last parameter of a `...hoja` tail is that parameter.
        let active_param = if contract.variadic {
            active_param.min(params.len().saturating_sub(1))
        } else {
            active_param
        };
        return Some(SignatureInfo {
            label: format!(
                "kazi {}",
                asili_parser::builtins::format_signature(&name, contract)
            ),
            params,
            active_param,
            doc: (!contract.doc.is_empty()).then(|| contract.doc.clone()),
        });
    }

    None
}

fn signature_from_params<'a>(
    name: &str,
    params: impl Iterator<Item = (&'a str, &'a str)>,
    return_type: &str,
    active_param: usize,
) -> SignatureInfo {
    let params: Vec<String> = params.map(|(n, t)| format!("{n}: {t}")).collect();
    let label = format!("kazi {}({}) -> {}", name, params.join(", "), return_type);
    SignatureInfo {
        doc: None,
        label,
        params,
        active_param,
    }
}
