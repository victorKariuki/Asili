//! Variable environment (stack of scopes).
//!
//! Local scopes live in one flat vector of bindings with a mark where each scope starts, so
//! entering and leaving a block allocates nothing and a lookup is a short scan from the
//! innermost binding outwards (scopes hold a handful of names). Names up to 22 bytes are stored
//! inline. The outermost (global) scope — constants and module-level names — is a hash map.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use crate::value::Value;

/// FxHash (Firefox/rustc's hasher): a multiply and a rotate per word — far cheaper than SipHash
/// for short identifier keys, and this map is never keyed by untrusted input at scale.
#[derive(Default, Clone, Copy)]
struct FxHasher(u64);

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        const K: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            let w = u64::from_le_bytes(c.try_into().expect("8 bytes"));
            self.0 = (self.0.rotate_left(5) ^ w).wrapping_mul(K);
        }
        for &b in chunks.remainder() {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(K);
        }
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

type Globals = HashMap<String, Value, BuildHasherDefault<FxHasher>>;

/// An identifier, inline when short.
#[derive(Clone)]
enum Name {
    Inline { len: u8, bytes: [u8; 22] },
    Heap(Box<str>),
}

impl Name {
    #[inline]
    fn new(s: &str) -> Name {
        if s.len() <= 22 {
            let mut bytes = [0u8; 22];
            bytes[..s.len()].copy_from_slice(s.as_bytes());
            Name::Inline {
                len: s.len() as u8,
                bytes,
            }
        } else {
            Name::Heap(s.into())
        }
    }

    #[inline]
    fn as_str(&self) -> &str {
        match self {
            // SAFETY: built from a `&str` prefix of exactly `len` bytes.
            Name::Inline { len, bytes } => unsafe {
                std::str::from_utf8_unchecked(&bytes[..*len as usize])
            },
            Name::Heap(s) => s,
        }
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

/// Stack of scopes; inner scope shadows outer.
#[derive(Clone, Debug, Default)]
pub struct Env {
    /// The outermost scope.
    globals: Globals,
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
    fn find(&self, name: &str) -> Option<usize> {
        self.locals.iter().rposition(|(n, _)| n.as_str() == name)
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.get_ref(name).cloned()
    }

    #[inline]
    pub(crate) fn get_ref(&self, name: &str) -> Option<&Value> {
        match self.find(name) {
            Some(i) => Some(&self.locals[i].1),
            None => self.globals.get(name),
        }
    }

    pub(crate) fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        match self.find(name) {
            Some(i) => Some(&mut self.locals[i].1),
            None => self.globals.get_mut(name),
        }
    }

    // HACK: set() falls back to inserting into the global (first) scope when the name is not found
    // in any existing scope. This silently creates a global from inside a function body, which masks
    // undefined-variable bugs. Should return an error (or at minimum panic in debug builds) when
    // the name does not already exist in any scope.
    pub fn set(&mut self, name: &str, value: Value) {
        match self.find(name) {
            Some(i) => self.locals[i].1 = value,
            None => {
                self.globals.insert(name.to_string(), value);
            }
        }
    }

    pub fn define(&mut self, name: &str, value: Value) {
        let Some(&start) = self.marks.last() else {
            self.globals.insert(name.to_string(), value);
            return;
        };
        // Redefining a name in the same scope replaces it.
        match self.locals[start..]
            .iter()
            .position(|(n, _)| n.as_str() == name)
        {
            Some(i) => self.locals[start + i].1 = value,
            None => self.locals.push((Name::new(name), value)),
        }
    }

    pub fn drop(&mut self, name: &str) -> bool {
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
            None => self.globals.remove(name).is_some(),
        }
    }

    /// Seed the outermost scope with global constants (Ukomo, Siyo_Namba, PI, E, KWELI, TOLEO, etc.).
    pub fn seed_global_constants(&mut self) {
        for (name, value) in global_constants() {
            self.globals.insert(name.to_string(), value);
        }
    }

    /// Names defined in any scope (for REPL: pass as extern_constants so later lines see them).
    pub fn defined_names(&self) -> Vec<String> {
        let mut names: std::collections::HashSet<String> = self.globals.keys().cloned().collect();
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
            .chain(self.globals.iter().map(|(k, v)| (k.clone(), v.clone())))
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
            Value::Neno(std::path::MAIN_SEPARATOR.to_string()),
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
