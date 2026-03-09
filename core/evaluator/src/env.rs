//! Variable environment (stack of scopes).

use std::collections::HashMap;

use crate::value::Value;

/// Stack of scopes; inner scope shadows outer.
#[derive(Clone, Debug, Default)]
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v.clone());
            }
        }
        None
    }

    pub fn set(&mut self, name: &str, value: Value) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return;
            }
        }
        if let Some(scope) = self.scopes.first_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    pub fn define(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    /// Seed the outermost scope with global constants (Ukomo, Siyo_Namba, PI, E, KWELI, TOLEO, etc.).
    pub fn seed_global_constants(&mut self) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert("KWELI".to_string(), Value::Ukweli(true));
            scope.insert("SIYO_KWELI".to_string(), Value::Ukweli(false));
            scope.insert("TUPU".to_string(), Value::Tupu);
            scope.insert("Ukomo".to_string(), Value::Namba(f64::INFINITY));
            scope.insert("Siyo_Namba".to_string(), Value::Namba(f64::NAN));
            scope.insert("TOLEO".to_string(), Value::Neno(option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0").into()));
            scope.insert("JINA_OS".to_string(), Value::Neno(std::env::consts::OS.into()));
            scope.insert("SEKUNDE_KWA_SIKU".to_string(), Value::Namba(86400.0));
            scope.insert("MWANZO_WA_ZAMANI".to_string(), Value::Namba(0.0));
            scope.insert("NJIA_SEPARATOR".to_string(), Value::Neno(std::path::MAIN_SEPARATOR.to_string()));
            scope.insert("PI".to_string(), Value::Namba(std::f64::consts::PI));
            scope.insert("E".to_string(), Value::Namba(std::f64::consts::E));
            scope.insert(
                "PHI".to_string(),
                Value::Namba((1.0_f64 + 5.0_f64.sqrt()) / 2.0),
            );
            scope.insert("TAU".to_string(), Value::Namba(2.0 * std::f64::consts::PI));
            scope.insert("LN10".to_string(), Value::Namba(10.0_f64.ln()));
            scope.insert("LN2".to_string(), Value::Namba(2.0_f64.ln()));
            scope.insert("LOG10E".to_string(), Value::Namba(std::f64::consts::E.log10()));
            scope.insert("LOG2E".to_string(), Value::Namba(std::f64::consts::E.log2()));
            scope.insert(
                "KIPEUO1_2".to_string(),
                Value::Namba(1.0 / 2.0_f64.sqrt()),
            );
            scope.insert("KIPEUO2".to_string(), Value::Namba(2.0_f64.sqrt()));
            scope.insert("KIPEUO3".to_string(), Value::Namba(3.0_f64.sqrt()));
            scope.insert("KIPEUO5".to_string(), Value::Namba(5.0_f64.sqrt()));
            scope.insert("EPSILON".to_string(), Value::Namba(f64::EPSILON));
            scope.insert("INF".to_string(), Value::Namba(f64::INFINITY));
            scope.insert("NAN".to_string(), Value::Namba(f64::NAN));
        }
    }

    /// Names defined in any scope (for REPL: pass as extern_constants so later lines see them).
    pub fn defined_names(&self) -> Vec<String> {
        let mut names = std::collections::HashSet::new();
        for scope in &self.scopes {
            for k in scope.keys() {
                names.insert(k.clone());
            }
        }
        names.into_iter().collect()
    }
}
