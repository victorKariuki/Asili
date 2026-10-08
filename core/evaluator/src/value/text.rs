//! [`Text`]: the storage of a `Neno` — a reference-counted, immutable-while-shared string in one
//! allocation (a header, then the bytes), one pointer wide.
//!
//! Like `Rc<str>`, copying a `Text` bumps a count and every copy reads the same bytes. Unlike
//! it, a `Text` has spare capacity and [`Text::push_str`] appends in place when no other copy
//! exists, so building a string by repeated `s = s + t` costs amortized linear time instead of
//! copying the whole string at every step.

use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

#[repr(C)]
struct Header {
    /// Copies sharing the allocation.
    refs: Cell<usize>,
    len: usize,
    cap: usize,
}

const HEADER: usize = std::mem::size_of::<Header>();

/// Shared text (see the module documentation).
pub struct Text(NonNull<Header>);

fn layout(cap: usize) -> Layout {
    Layout::from_size_align(HEADER + cap, std::mem::align_of::<Header>()).expect("text size")
}

impl Text {
    /// A new, unshared text holding `s`, with room for `cap` bytes (at least `s.len()`).
    fn with_capacity(s: &str, cap: usize) -> Text {
        let cap = cap.max(s.len());
        let l = layout(cap);
        // SAFETY: `l` has a non-zero size (the header); the header is written before any read,
        // and `s.len() <= cap` bytes follow it.
        unsafe {
            let p = alloc(l) as *mut Header;
            let Some(p) = NonNull::new(p) else {
                handle_alloc_error(l)
            };
            p.as_ptr().write(Header {
                refs: Cell::new(1),
                len: s.len(),
                cap,
            });
            std::ptr::copy_nonoverlapping(s.as_ptr(), Self::data(p), s.len());
            Text(p)
        }
    }

    fn data(p: NonNull<Header>) -> *mut u8 {
        // SAFETY: the bytes start right after the header, in the same allocation.
        unsafe { (p.as_ptr() as *mut u8).add(HEADER) }
    }

    fn header(&self) -> &Header {
        // SAFETY: the allocation lives while any `Text` points at it.
        unsafe { self.0.as_ref() }
    }

    pub fn as_str(&self) -> &str {
        let h = self.header();
        // SAFETY: `len` initialized bytes of UTF-8 follow the header.
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(Self::data(self.0), h.len))
        }
    }

    /// Whether this is the only copy (so it may change in place).
    pub fn is_unique(&self) -> bool {
        self.header().refs.get() == 1
    }

    /// Append `s`: in place when this is the only copy (growing the allocation geometrically),
    /// otherwise into a new allocation this copy then holds alone.
    pub fn push_str(&mut self, s: &str) {
        let (len, cap) = (self.header().len, self.header().cap);
        let need = len + s.len();
        if !self.is_unique() {
            let mut fresh = Text::with_capacity(self.as_str(), need.max(2 * len));
            fresh.push_str(s);
            *self = fresh;
            return;
        }
        if need > cap {
            let new_cap = need.max(2 * cap).max(16);
            // SAFETY: the allocation was made with `layout(cap)`; the new size is non-zero.
            unsafe {
                let p = realloc(self.0.as_ptr() as *mut u8, layout(cap), HEADER + new_cap);
                let Some(p) = NonNull::new(p as *mut Header) else {
                    handle_alloc_error(layout(new_cap))
                };
                self.0 = p;
                (*p.as_ptr()).cap = new_cap;
            }
        }
        // SAFETY: unique, and `need <= cap` bytes fit after the header.
        unsafe {
            std::ptr::copy_nonoverlapping(s.as_ptr(), Self::data(self.0).add(len), s.len());
            (*self.0.as_ptr()).len = need;
        }
    }

    /// `a` followed by `b`, in one new allocation.
    pub fn concat(a: &str, b: &str) -> Text {
        let mut t = Text::with_capacity(a, a.len() + b.len());
        t.push_str(b);
        t
    }
}

impl Clone for Text {
    fn clone(&self) -> Text {
        let refs = &self.header().refs;
        refs.set(refs.get() + 1);
        Text(self.0)
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        let h = self.header();
        let refs = h.refs.get() - 1;
        h.refs.set(refs);
        if refs == 0 {
            let cap = h.cap;
            // SAFETY: the last copy frees the allocation it was made with.
            unsafe { dealloc(self.0.as_ptr() as *mut u8, layout(cap)) }
        }
    }
}

impl std::ops::Deref for Text {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::borrow::Borrow<str> for Text {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Text {
        Text::with_capacity(s, s.len())
    }
}

impl From<&String> for Text {
    fn from(s: &String) -> Text {
        Text::from(s.as_str())
    }
}

impl From<String> for Text {
    fn from(s: String) -> Text {
        Text::from(s.as_str())
    }
}

impl From<Box<str>> for Text {
    fn from(s: Box<str>) -> Text {
        Text::from(&*s)
    }
}

impl From<std::rc::Rc<str>> for Text {
    fn from(s: std::rc::Rc<str>) -> Text {
        Text::from(&*s)
    }
}

impl From<char> for Text {
    fn from(c: char) -> Text {
        Text::from(c.encode_utf8(&mut [0; 4]) as &str)
    }
}

impl From<&Text> for Text {
    fn from(s: &Text) -> Text {
        s.clone()
    }
}

impl Default for Text {
    fn default() -> Text {
        Text::from("")
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Text) -> bool {
        self.0 == other.0 || self.as_str() == other.as_str()
    }
}

impl Eq for Text {}

impl PartialEq<str> for Text {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Text) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Text {
    fn cmp(&self, other: &Text) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl std::hash::Hash for Text {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_in_place_only_when_unshared() {
        let mut a = Text::from("ab");
        a.push_str("cd");
        assert_eq!(a.as_str(), "abcd");
        let b = a.clone();
        a.push_str("e");
        assert_eq!((a.as_str(), b.as_str()), ("abcde", "abcd"));
        assert!(a.is_unique() && b.is_unique());
        let mut long = Text::default();
        for i in 0..10_000 {
            long.push_str(if i % 2 == 0 { "é" } else { "x" });
        }
        assert_eq!(long.chars().count(), 10_000);
        assert_eq!(Text::concat("ma", "neno"), Text::from("maneno"));
    }
}
