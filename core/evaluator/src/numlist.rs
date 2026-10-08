//! `Orodha<Namba>` storage for native code.
//!
//! A list stores all its elements in one representation, the narrowest that holds every element
//! exactly: signed integers of 1, 2, 4 or 8 bytes while every element is an integer in range (no
//! `-0.0`, NaN, ±∞ or fraction), else `f64` bits. A store that does not fit widens the whole
//! list; nothing narrows it except native code asking for a representation. Reading always
//! yields exactly the stored `f64`, so the representation is invisible to Asili programs, and a
//! list of flags or digits takes one byte per element instead of eight.
//!
//! Native code picks a representation per list register from the range analysis (a list whose
//! elements are proven to lie in `0..=1` is read and written as bytes with plain loads and
//! stores) and asks for it through [`NumList::ensure`], which fails when the elements do not fit.

/// How a list stores its elements: integers of 1, 2, 4 or 8 bytes (unsigned where no element is
/// negative — zero-extending loads are the cheapest), or `f64` bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    I64,
    F64,
}

/// Every representation, narrowest first (unsigned before signed at each width).
const ALL: [Kind; 8] = [
    Kind::U8,
    Kind::I8,
    Kind::U16,
    Kind::I16,
    Kind::U32,
    Kind::I32,
    Kind::I64,
    Kind::F64,
];

impl Kind {
    pub(crate) fn width(self) -> usize {
        match self {
            Kind::U8 | Kind::I8 => 1,
            Kind::U16 | Kind::I16 => 2,
            Kind::U32 | Kind::I32 => 4,
            Kind::I64 | Kind::F64 => 8,
        }
    }

    pub(crate) fn signed(self) -> bool {
        matches!(self, Kind::I8 | Kind::I16 | Kind::I32 | Kind::I64)
    }

    /// The integers this representation holds (`None` for `f64`, which holds everything).
    fn range(self) -> Option<(i64, i64)> {
        Some(match self {
            Kind::U8 => (0, u8::MAX as i64),
            Kind::I8 => (i8::MIN as i64, i8::MAX as i64),
            Kind::U16 => (0, u16::MAX as i64),
            Kind::I16 => (i16::MIN as i64, i16::MAX as i64),
            Kind::U32 => (0, u32::MAX as i64),
            Kind::I32 => (i32::MIN as i64, i32::MAX as i64),
            // `i64::MAX` itself is where 2^63 and above saturate.
            Kind::I64 => (i64::MIN, i64::MAX - 1),
            Kind::F64 => return None,
        })
    }

    /// Whether `v` is stored exactly in this representation. An integer round trip instead of
    /// `fract()`, which is a libm call on some targets: NaN converts to 0 and fails the compare,
    /// `-0.0` is caught by its sign bit, and values at or beyond ±2^63 saturate.
    #[inline]
    fn holds(self, v: f64) -> bool {
        let Some((lo, hi)) = self.range() else {
            return true;
        };
        let i = v as i64;
        i as f64 == v && (lo..=hi).contains(&i) && (i != 0 || v.to_bits() == 0)
    }

    /// Whether everything `other` holds, this holds too.
    fn covers(self, other: Kind) -> bool {
        match (self.range(), other.range()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some((lo, hi)), Some((olo, ohi))) => lo <= olo && ohi <= hi,
        }
    }

    /// The narrowest representation holding `v`.
    #[inline]
    fn of(v: f64) -> Kind {
        ALL.into_iter().find(|k| k.holds(v)).unwrap_or(Kind::F64)
    }

    /// The narrowest representation holding everything `self` holds, and `v`.
    fn join(self, v: f64) -> Kind {
        ALL.into_iter()
            .find(|k| k.covers(self) && k.holds(v))
            .unwrap_or(Kind::F64)
    }

    /// The representation native code uses for the integers `lo..=hi` (which lie within ±2^53):
    /// the narrowest that holds them, skipping 16-bit elements — measured, a 16-bit store read
    /// back soon after (a swap loop) runs twice as slow as a 32-bit one, while bytes and 32-bit
    /// words cost the same as 64-bit words. Lists the host builds still use 16 bits.
    pub(crate) fn for_range(lo: f64, hi: f64) -> Kind {
        ALL.into_iter()
            .filter(|k| k.width() != 2)
            .find(|k| k.holds(lo) && k.holds(hi))
            .unwrap_or(Kind::I64)
    }

    /// Code passed between native code and the runtime (`list_ptr`).
    pub(crate) fn code(self) -> u32 {
        ALL.iter().position(|&k| k == self).unwrap_or(ALL.len() - 1) as u32
    }

    pub(crate) fn from_code(code: u32) -> Kind {
        ALL.get(code as usize).copied().unwrap_or(Kind::F64)
    }
}

/// `repr(C)` with `len` first: native code appends in place and stores the new length at the
/// list's address (see `native::list_head`).
#[derive(Clone, Debug)]
#[repr(C)]
pub(crate) struct NumList {
    len: usize,
    /// Element storage, 8-byte aligned: element `i` occupies bytes `i * width ..`.
    words: Vec<u64>,
    kind: Kind,
    /// `kind.width()`.
    width: usize,
}

/// Words holding `n` elements of `width` bytes.
fn words_for(n: usize, width: usize) -> usize {
    (n * width).div_ceil(8)
}

impl Default for NumList {
    fn default() -> Self {
        NumList::with_kind(Kind::U8, 0)
    }
}

impl NumList {
    fn with_kind(kind: Kind, capacity: usize) -> Self {
        let width = kind.width();
        NumList {
            words: Vec::with_capacity(words_for(capacity, width)),
            len: 0,
            kind,
            width,
        }
    }

    /// Room for `n` zero elements.
    fn zeroed(kind: Kind, n: usize) -> Self {
        let mut l = NumList::with_kind(kind, n);
        l.words.resize(words_for(n, l.width), 0);
        l.len = n;
        l
    }

    pub(crate) fn repeat(v: f64, count: usize) -> Self {
        let mut l = NumList::zeroed(Kind::of(v), count);
        if !(v == 0.0 && l.kind != Kind::F64) {
            for i in 0..count {
                l.write(i, v);
            }
        }
        l
    }

    #[inline(always)]
    fn read(&self, i: usize) -> f64 {
        debug_assert!(i < self.len);
        let p = self.words.as_ptr() as *const u8;
        // SAFETY: `i < len`, and `words` holds at least `len * width` bytes, aligned for every
        // width at these offsets.
        unsafe {
            match self.kind {
                Kind::U8 => *p.add(i) as f64,
                Kind::I8 => *(p.add(i) as *const i8) as f64,
                Kind::U16 => *(p.add(2 * i) as *const u16) as f64,
                Kind::I16 => *(p.add(2 * i) as *const i16) as f64,
                Kind::U32 => *(p.add(4 * i) as *const u32) as f64,
                Kind::I32 => *(p.add(4 * i) as *const i32) as f64,
                Kind::I64 => *(p.add(8 * i) as *const i64) as f64,
                Kind::F64 => f64::from_bits(*(p.add(8 * i) as *const u64)),
            }
        }
    }

    /// Store `v`, which the current representation holds, at `i < len`.
    #[inline(always)]
    fn write(&mut self, i: usize, v: f64) {
        debug_assert!(i < self.len && self.kind.holds(v));
        let p = self.words.as_mut_ptr() as *mut u8;
        // SAFETY: as in `read`.
        unsafe {
            match self.kind {
                Kind::U8 => *p.add(i) = v as u8,
                Kind::I8 => *(p.add(i) as *mut i8) = v as i8,
                Kind::U16 => *(p.add(2 * i) as *mut u16) = v as u16,
                Kind::I16 => *(p.add(2 * i) as *mut i16) = v as i16,
                Kind::U32 => *(p.add(4 * i) as *mut u32) = v as u32,
                Kind::I32 => *(p.add(4 * i) as *mut i32) = v as i32,
                Kind::I64 => *(p.add(8 * i) as *mut i64) = v as i64,
                Kind::F64 => *(p.add(8 * i) as *mut u64) = v.to_bits(),
            }
        }
    }

    /// Re-store every element as `kind` (which must hold them all).
    fn convert(&mut self, kind: Kind) {
        let values: Vec<f64> = self.iter().collect();
        let mut l = NumList::zeroed(kind, values.len());
        for (i, v) in values.into_iter().enumerate() {
            l.write(i, v);
        }
        *self = l;
    }

    /// Make room for `v`: widen if the current representation does not hold it.
    #[inline]
    fn admit(&mut self, v: f64) {
        if !self.kind.holds(v) {
            self.convert(self.kind.join(v));
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub(crate) fn get(&self, i: usize) -> Option<f64> {
        (i < self.len).then(|| self.read(i))
    }

    /// Store `v` at `i`; `false` if out of range.
    #[inline(always)]
    pub(crate) fn set(&mut self, i: usize, v: f64) -> bool {
        if i >= self.len {
            return false;
        }
        self.admit(v);
        self.write(i, v);
        true
    }

    #[inline(always)]
    pub(crate) fn push(&mut self, v: f64) {
        self.admit(v);
        if words_for(self.len + 1, self.width) > self.words.len() {
            // Grow, and hand native code the whole allocation as room to append in place.
            self.words.push(0);
            let cap = self.words.capacity();
            self.words.resize(cap, 0);
        }
        self.len += 1;
        self.write(self.len - 1, v);
    }

    #[inline(always)]
    pub(crate) fn remove(&mut self, i: usize) -> f64 {
        assert!(
            i < self.len,
            "remove index {i} out of range for length {}",
            self.len
        );
        let v = self.read(i);
        if i + 1 == self.len {
            self.len -= 1; // the last element: nothing to move
            return v;
        }
        let w = self.width;
        let end = self.len * w;
        // SAFETY: `words` holds at least `end` bytes.
        let bytes =
            unsafe { std::slice::from_raw_parts_mut(self.words.as_mut_ptr() as *mut u8, end) };
        bytes.copy_within((i + 1) * w..end, i * w);
        self.len -= 1;
        v
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        (0..self.len).map(|i| self.read(i))
    }

    /// Bytes of element storage (elements fit while `len * width` stays within it).
    pub(crate) fn room(&self) -> usize {
        8 * self.words.len()
    }

    /// Data pointer in the current representation.
    pub(crate) fn as_mut_ptr(&mut self) -> *mut u64 {
        self.words.as_mut_ptr()
    }

    /// Switch to representation `kind` and return the data pointer, or null when some element
    /// does not fit it.
    pub(crate) fn ensure(&mut self, kind: Kind) -> *mut u64 {
        if kind != self.kind {
            if !kind.covers(self.kind) && !self.iter().all(|v| kind.holds(v)) {
                return std::ptr::null_mut();
            }
            self.convert(kind);
        }
        self.words.as_mut_ptr()
    }
}

impl FromIterator<f64> for NumList {
    fn from_iter<I: IntoIterator<Item = f64>>(items: I) -> Self {
        let values: Vec<f64> = items.into_iter().collect();
        let kind = values.iter().fold(Kind::U8, |k, &v| k.join(v));
        let mut l = NumList::zeroed(kind, values.len());
        for (i, v) in values.into_iter().enumerate() {
            l.write(i, v);
        }
        l
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, NumList};

    fn bits(l: &NumList) -> Vec<u64> {
        l.iter().map(f64::to_bits).collect()
    }

    const TRICKY: [f64; 16] = [
        0.0,
        -0.0,
        1.0,
        -7.0,
        127.0,
        128.0,
        -128.0,
        -129.0,
        32_767.0,
        -32_769.0,
        2_147_483_648.0,
        0.5,
        f64::NAN,
        f64::INFINITY,
        9.007_199_254_740_993e15,
        i64::MIN as f64,
    ];

    #[test]
    fn values_round_trip_through_every_representation() {
        for &v in &TRICKY {
            let mut small: NumList = [3.0, 4.0].into_iter().collect();
            small.push(v);
            assert_eq!(small.get(2).map(f64::to_bits), Some(v.to_bits()), "{v}");
            assert_eq!(small.get(0), Some(3.0));
            let mut l: NumList = [v, 2.0].into_iter().collect();
            let before = bits(&l);
            for k in [
                Kind::F64,
                Kind::I64,
                Kind::U32,
                Kind::I32,
                Kind::U16,
                Kind::I16,
                Kind::U8,
                Kind::I8,
                Kind::F64,
            ] {
                l.ensure(k);
                assert_eq!(bits(&l), before, "{v} as {k:?}");
            }
            let r = NumList::repeat(v, 3);
            assert!(r.iter().all(|x| x.to_bits() == v.to_bits()));
            let mut s = NumList::repeat(1.0, 5);
            assert!(s.set(4, v));
            assert_eq!(s.get(4).map(f64::to_bits), Some(v.to_bits()), "{v}");
            assert_eq!(s.get(3), Some(1.0));
        }
    }

    #[test]
    fn lists_are_as_narrow_as_their_elements() {
        assert_eq!(NumList::repeat(1.0, 10).kind, Kind::U8);
        let l: NumList = [1.0, 300.0].into_iter().collect();
        assert_eq!(l.kind, Kind::U16);
        let l: NumList = [200.0, -1.0].into_iter().collect();
        assert_eq!(l.kind, Kind::I16);
        assert_eq!(l.get(0), Some(200.0));
        let mut l = NumList::repeat(0.0, 5_000_000);
        assert_eq!(l.words.len(), 625_000); // one byte per element
        l.set(7, 70_000.0);
        assert_eq!(l.kind, Kind::U32);
        l.set(9, -1.0);
        assert_eq!(l.kind, Kind::I64);
        assert_eq!(l.get(9), Some(-1.0));
        assert_eq!(l.get(7), Some(70_000.0));
        assert_eq!(l.get(8), Some(0.0));
    }

    #[test]
    fn narrowing_fails_when_elements_do_not_fit() {
        let mut l: NumList = [1.0, 2.5].into_iter().collect();
        assert!(l.ensure(Kind::I64).is_null());
        let mut l: NumList = [1.0, 200.0].into_iter().collect();
        assert!(l.ensure(Kind::I8).is_null());
        assert!(!l.ensure(Kind::I16).is_null());
        assert!(l.set(0, -0.0));
        assert_eq!(l.get(0).map(f64::to_bits), Some((-0.0f64).to_bits()));
    }

    #[test]
    fn push_and_remove_keep_order() {
        let mut l = NumList::default();
        for i in 0..20 {
            l.push(i as f64);
        }
        assert_eq!(l.remove(0), 0.0);
        assert_eq!(l.remove(5), 6.0);
        l.push(1e6);
        let v: Vec<f64> = l.iter().collect();
        let mut want: Vec<f64> = (1..20).filter(|&i| i != 6).map(|i| i as f64).collect();
        want.push(1e6);
        assert_eq!(v, want);
    }
}
