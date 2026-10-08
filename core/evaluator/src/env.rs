//! Variable environment (stack of scopes).
//!
//! Local scopes live in one flat vector of bindings with a mark where each scope starts, so
//! entering and leaving a block allocates nothing and a lookup is a short scan from the
//! innermost binding outwards (scopes hold a handful of names), comparing interned [`Name`]s —
//! one pointer compare per binding. The outermost (global) scope — constants and module-level
//! names — is a hash map keyed by the names' precomputed hashes.

use asili_parser::{FxHashMap, Name};

use crate::value::Value;

/// Stack of scopes; inner scope shadows outer.
#[derive(Clone, Debug, Default)]
pub struct Env {
    /// The outermost scope.
    globals: FxHashMap<Name, Value>,
    /// Bindings of every inner scope, outermost first.
    locals: Vec<(Name, Value)>,
    /// Where each inner scope's bindings start in `locals`.
    marks: Vec<usize>,
}

impl Env {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn push_scope(&mut self) {
        self.marks.push(self.locals.len());
    }

    #[inline]
    pub fn pop_scope(&mut self) {
        if let Some(start) = self.marks.pop() {
            self.locals.truncate(start);
        }
    }

    /// Index in `locals` of the innermost binding of `name`.
    #[inline]
    fn find(&self, name: Name) -> Option<usize> {
        self.locals.iter().rposition(|(n, _)| *n == name)
    }

    pub fn get(&self, name: Name) -> Option<Value> {
        self.get_ref(name).cloned()
    }

    #[inline]
    pub(crate) fn get_ref(&self, name: Name) -> Option<&Value> {
        match self.find(name) {
            Some(i) => Some(&self.locals[i].1),
            None => self.globals.get(&name),
        }
    }

    pub(crate) fn get_mut(&mut self, name: Name) -> Option<&mut Value> {
        match self.find(name) {
            Some(i) => Some(&mut self.locals[i].1),
            None => self.globals.get_mut(&name),
        }
    }

    // HACK: set() falls back to inserting into the global (first) scope when the name is not found
    // in any existing scope. This silently creates a global from inside a function body, which masks
    // undefined-variable bugs. Should return an error (or at minimum panic in debug builds) when
    // the name does not already exist in any scope.
    pub fn set(&mut self, name: Name, value: Value) {
        match self.find(name) {
            Some(i) => self.locals[i].1 = value,
            None => {
                self.globals.insert(name, value);
            }
        }
    }

    pub fn define(&mut self, name: Name, value: Value) {
        let Some(&start) = self.marks.last() else {
            self.globals.insert(name, value);
            return;
        };
        // Redefining a name in the same scope replaces it.
        match self.locals[start..].iter().position(|(n, _)| *n == name) {
            Some(i) => self.locals[start + i].1 = value,
            None => self.locals.push((name, value)),
        }
    }

    pub fn drop(&mut self, name: Name) -> bool {
        match self.find(name) {
            Some(i) => {
                self.locals.remove(i);
                for m in self.marks.iter_mut() {
                    if *m > i {
                        *m -= 1;
                    }
                }
                true
            }
            None => self.globals.remove(&name).is_some(),
        }
    }

    /// Seed the outermost scope with global constants (Ukomo, Siyo_Namba, PI, E, KWELI, TOLEO, etc.).
    pub fn seed_global_constants(&mut self) {
        for (name, value) in global_constants() {
            self.globals.insert(Name::new(name), value);
        }
    }

    /// Names defined in any scope (for REPL: pass as extern_constants so later lines see them).
    pub fn defined_names(&self) -> Vec<String> {
        let mut names: std::collections::HashSet<String> =
            self.globals.keys().map(|n| n.to_string()).collect();
        names.extend(self.locals.iter().map(|(n, _)| n.as_str().to_string()));
        names.into_iter().collect()
    }

    /// Every `(name, value)` binding currently in scope, innermost scope first — for a debugger's
    /// "current variables" snapshot (`debug_hook::RealDebugHook::record_bindings`), where a name
    /// shadowed by an inner scope must report the inner (shadowing) value, matching `Env::get`'s
    /// own lookup order exactly. Order/dedup of same-named entries across scopes is the caller's
    /// job (keep only the first occurrence of each name) — this just yields every scope's own
    /// entries, innermost first.
    pub fn iter_innermost_first(&self) -> impl Iterator<Item = (String, Value)> + '_ {
        self.locals
            .iter()
            .rev()
            .map(|(n, v)| (n.as_str().to_string(), v.clone()))
            .chain(self.globals.iter().map(|(k, v)| (k.to_string(), v.clone())))
    }
}

/// The language's predefined names (`Ukomo`, `Siyo_Namba`, `PI`, `KWELI`, `TOLEO`, …): what the
/// tree-walker's outermost scope starts with, and what the bytecode compiler resolves a name
/// to when it is neither local nor a module constant.
pub(crate) fn global_constants() -> Vec<(&'static str, Value)> {
    vec![
        ("KWELI", Value::Ukweli(true)),
        ("SIYO_KWELI", Value::Ukweli(false)),
        ("TUPU", Value::Tupu),
        ("Ukomo", Value::Namba(f64::INFINITY)),
        ("Siyo_Namba", Value::Namba(f64::NAN)),
        (
            "TOLEO",
            Value::Neno(option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0").into()),
        ),
        ("JINA_OS", Value::Neno(std::env::consts::OS.into())),
        ("SEKUNDE_KWA_SIKU", Value::Namba(86400.0)),
        ("MWANZO_WA_ZAMANI", Value::Namba(0.0)),
        (
            "NJIA_SEPARATOR",
            Value::neno(std::path::MAIN_SEPARATOR.to_string()),
        ),
        ("PI", Value::Namba(std::f64::consts::PI)),
        ("E", Value::Namba(std::f64::consts::E)),
        ("PHI", Value::Namba((1.0_f64 + 5.0_f64.sqrt()) / 2.0)),
        ("TAU", Value::Namba(2.0 * std::f64::consts::PI)),
        ("LN10", Value::Namba(10.0_f64.ln())),
        ("LN2", Value::Namba(2.0_f64.ln())),
        ("LOG10E", Value::Namba(std::f64::consts::E.log10())),
        ("LOG2E", Value::Namba(std::f64::consts::E.log2())),
        ("KIPEUO1_2", Value::Namba(1.0 / 2.0_f64.sqrt())),
        ("KIPEUO2", Value::Namba(2.0_f64.sqrt())),
        ("KIPEUO3", Value::Namba(3.0_f64.sqrt())),
        ("KIPEUO5", Value::Namba(5.0_f64.sqrt())),
        ("EPSILON", Value::Namba(f64::EPSILON)),
        ("INF", Value::Namba(f64::INFINITY)),
        ("NAN", Value::Namba(f64::NAN)),
    ]
}
