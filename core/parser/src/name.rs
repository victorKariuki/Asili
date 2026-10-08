//! Interned identifiers.
//!
//! A [`Name`] is a pointer to the one copy of its text in a process-wide table, so comparing two
//! names is comparing two pointers, copying one copies a pointer, and hashing one writes a hash
//! computed once when the text was first seen. Variable lookups in the tree-walker's scopes, and
//! every other map keyed by an identifier, use them instead of comparing strings.
//!
//! Interned text is never freed: the table holds each distinct identifier a process has seen,
//! which for a compiler, a run or an editor session is small.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// FxHash (Firefox/rustc's hasher): a multiply and a rotate per word — far cheaper than SipHash
/// for short identifier keys. Never use it for maps keyed by untrusted input at scale.
#[derive(Default, Clone, Copy)]
pub struct FxHasher(u64);

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
    fn write_u64(&mut self, n: u64) {
        const K: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(K);
    }

    #[inline]
    fn write_u32(&mut self, n: u32) {
        self.write_u64(n as u64);
    }

    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

/// A `HashMap` with [`FxHasher`]: faster than the default for small keys, and its iteration
/// order depends only on what was inserted, so output built by iterating one is reproducible.
pub type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

/// A `HashSet` with [`FxHasher`] (see [`FxHashMap`]).
pub type FxHashSet<T> = std::collections::HashSet<T, BuildHasherDefault<FxHasher>>;

struct Entry {
    hash: u64,
    text: Box<str>,
}

/// An interned identifier. Equal texts are the same `Name`.
#[derive(Clone, Copy)]
pub struct Name(&'static Entry);

fn table() -> &'static Mutex<FxHashMap<&'static str, Name>> {
    static TABLE: OnceLock<Mutex<FxHashMap<&'static str, Name>>> = OnceLock::new();
    TABLE.get_or_init(Default::default)
}

impl Name {
    /// The name for `text`, adding it to the table the first time.
    pub fn new(text: &str) -> Name {
        let mut table = table().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&name) = table.get(text) {
            return name;
        }
        let mut h = FxHasher::default();
        h.write(text.as_bytes());
        let entry: &'static Entry = Box::leak(Box::new(Entry {
            hash: h.finish(),
            text: text.into(),
        }));
        let name = Name(entry);
        table.insert(&entry.text, name);
        name
    }

    #[inline]
    pub fn as_str(self) -> &'static str {
        &self.0.text
    }
}

impl PartialEq for Name {
    #[inline]
    fn eq(&self, other: &Name) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for Name {}

impl Hash for Name {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.hash);
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Name) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Name {
    /// By text, so sorted output does not depend on interning order.
    fn cmp(&self, other: &Name) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl std::ops::Deref for Name {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Default for Name {
    fn default() -> Name {
        Name::new("")
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Name {
        Name::new(s)
    }
}

impl From<&String> for Name {
    fn from(s: &String) -> Name {
        Name::new(s)
    }
}

impl From<String> for Name {
    fn from(s: String) -> Name {
        Name::new(&s)
    }
}

impl From<Name> for String {
    fn from(n: Name) -> String {
        n.as_str().to_string()
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for Name {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<Name> for str {
    fn eq(&self, other: &Name) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<Name> for &str {
    fn eq(&self, other: &Name) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<Name> for String {
    fn eq(&self, other: &Name) -> bool {
        self == other.as_str()
    }
}

/// Serialized as its text, so syntax trees keep their format.
impl Serialize for Name {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Name, D::Error> {
        let text = std::borrow::Cow::<'de, str>::deserialize(d)?;
        Ok(Name::new(&text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_text_is_the_same_name() {
        let a = Name::new("hesabu");
        let b = Name::from(String::from("hesabu"));
        assert_eq!(a, b);
        assert!(std::ptr::eq(a.as_str(), b.as_str()));
        assert_ne!(a, Name::new("hesabu2"));
        assert_eq!(a, "hesabu");
    }

    #[test]
    fn serializes_as_text() {
        let a = Name::new("x");
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, "\"x\"");
        let back: Name = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }
}
