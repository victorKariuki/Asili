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

// Runtime code never panics on its own: an impossible state is an error the program sees
// (and its safe state handles), not a crash (see docs/design/safety-critical-roadmap.md §3).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

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
/// list's address (see `native::list_head`), as a 64-bit word on every target (a 32-bit target
/// keeps a zero high half after it).
#[derive(Debug)]
#[repr(C)]
pub(crate) struct NumList {
    len: usize,
    #[cfg(target_pointer_width = "32")]
    len_high: u32,
    /// Element storage, 8-byte aligned: element `i` occupies bytes `i * width ..`.
    words: Vec<u64>,
    kind: Kind,
    /// `kind.width()`.
    width: usize,
}

/// `$body` with `$xs` bound to the elements of numeric list `$l` as a slice of their
/// representation (`&[u8]` … `&[f64]`): one monomorphic loop per representation. (A cast such
/// as `x as f64` in `$body` is a no-op on some arms, hence the `allow`.)
macro_rules! typed {
    ($l:expr, $xs:ident => $body:expr) => {{
        let l = $l;
        let n = l.len;
        let p = l.words.as_ptr() as *const u8;
        // SAFETY: `words` holds `n` elements of the representation's width, aligned for it.
        unsafe {
            match l.kind {
                Kind::U8 => {
                    let $xs = std::slice::from_raw_parts(p, n);
                    #[allow(clippy::unnecessary_cast)]
                    let r = $body;
                    r
                }
                Kind::I8 => {
                    let $xs = std::slice::from_raw_parts(p as *const i8, n);
                    #[allow(clippy::unnecessary_cast)]
                    let r = $body;
                    r
                }
                Kind::U16 => {
                    let $xs = std::slice::from_raw_parts(p as *const u16, n);
                    #[allow(clippy::unnecessary_cast)]
                    let r = $body;
                    r
                }
                Kind::I16 => {
                    let $xs = std::slice::from_raw_parts(p as *const i16, n);
                    #[allow(clippy::unnecessary_cast)]
                    let r = $body;
                    r
                }
                Kind::U32 => {
                    let $xs = std::slice::from_raw_parts(p as *const u32, n);
                    #[allow(clippy::unnecessary_cast)]
                    let r = $body;
                    r
                }
                Kind::I32 => {
                    let $xs = std::slice::from_raw_parts(p as *const i32, n);
                    #[allow(clippy::unnecessary_cast)]
                    let r = $body;
                    r
                }
                Kind::I64 => {
                    let $xs = std::slice::from_raw_parts(p as *const i64, n);
                    #[allow(clippy::unnecessary_cast)]
                    let r = $body;
                    r
                }
                Kind::F64 => {
                    let $xs = std::slice::from_raw_parts(p as *const f64, n);
                    #[allow(clippy::unnecessary_cast)]
                    let r = $body;
                    r
                }
            }
        }
    }};
}

/// Words holding `n` elements of `width` bytes.
fn words_for(n: usize, width: usize) -> usize {
    (n * width).div_ceil(8)
}

// Copies reuse the destination's storage (`clone_from`), so copying a list into a register that
// already held one of the same size allocates nothing: strict code passes lists to its callees
// without allocating (see `salama.rs`).
impl Clone for NumList {
    fn clone(&self) -> Self {
        NumList {
            len: self.len,
            #[cfg(target_pointer_width = "32")]
            len_high: 0,
            words: self.words.clone(),
            kind: self.kind,
            width: self.width,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.len = source.len;
        self.words.clone_from(&source.words);
        self.kind = source.kind;
        self.width = source.width;
    }
}

impl Default for NumList {
    fn default() -> Self {
        NumList::with_kind(Kind::U8, 0)
    }
}

impl NumList {
    /// Empty — exactly a new list (`NumList::default`) — keeping its storage for reuse.
    pub(crate) fn clear(&mut self) {
        self.len = 0;
        self.words.clear();
        self.kind = Kind::U8;
        self.width = Kind::U8.width();
    }

    fn with_kind(kind: Kind, capacity: usize) -> Self {
        let width = kind.width();
        NumList {
            words: Vec::with_capacity(words_for(capacity, width)),
            len: 0,
            #[cfg(target_pointer_width = "32")]
            len_high: 0,
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

    /// Elements `start..end` (in range), in the same representation.
    pub(crate) fn slice(&self, start: usize, end: usize) -> NumList {
        let mut l = NumList::zeroed(self.kind, end - start);
        let w = self.width;
        // SAFETY: both hold at least the bytes copied: `self` `end * w`, `l` `(end - start) * w`.
        unsafe {
            std::ptr::copy_nonoverlapping(
                (self.words.as_ptr() as *const u8).add(start * w),
                l.words.as_mut_ptr() as *mut u8,
                (end - start) * w,
            );
        }
        l
    }

    /// The elements in reverse order, in the same representation.
    pub(crate) fn reversed(&self) -> NumList {
        let mut l = NumList::zeroed(self.kind, self.len);
        for i in 0..self.len {
            l.write(self.len - 1 - i, self.read(i));
        }
        l
    }

    /// The elements sorted (stable) by `cmp`, in the same representation. Integer
    /// representations are radix-sorted on their own bits (equal integers are
    /// indistinguishable, so stability is moot there).
    pub(crate) fn sorted(&self, cmp: impl Fn(&f64, &f64) -> std::cmp::Ordering) -> NumList {
        let mut l = self.clone();
        let n = self.len;
        let p = l.words.as_mut_ptr() as *mut u8;
        // SAFETY: `words` holds `n` elements of the representation's width, aligned for it.
        unsafe {
            match self.kind {
                Kind::U8 => radix_sort(std::slice::from_raw_parts_mut(p, n), |x| x as u64, 1),
                Kind::I8 => radix_sort(
                    std::slice::from_raw_parts_mut(p as *mut i8, n),
                    |x| (x as u8 ^ 0x80) as u64,
                    1,
                ),
                Kind::U16 => radix_sort(
                    std::slice::from_raw_parts_mut(p as *mut u16, n),
                    |x| x as u64,
                    2,
                ),
                Kind::I16 => radix_sort(
                    std::slice::from_raw_parts_mut(p as *mut i16, n),
                    |x| (x as u16 ^ 0x8000) as u64,
                    2,
                ),
                Kind::U32 => radix_sort(
                    std::slice::from_raw_parts_mut(p as *mut u32, n),
                    |x| x as u64,
                    4,
                ),
                Kind::I32 => radix_sort(
                    std::slice::from_raw_parts_mut(p as *mut i32, n),
                    |x| (x as u32 ^ 0x8000_0000) as u64,
                    4,
                ),
                Kind::I64 => radix_sort(
                    std::slice::from_raw_parts_mut(p as *mut i64, n),
                    |x| x as u64 ^ 0x8000_0000_0000_0000,
                    8,
                ),
                Kind::F64 => {
                    // Equal doubles can still differ (0 and -0): keep their order (stable).
                    let mut v = self.to_f64s();
                    v.sort_by(cmp);
                    for (i, x) in v.into_iter().enumerate() {
                        l.write(i, x);
                    }
                }
            }
        }
        l
    }

    /// The elements as doubles, converted in one pass per representation.
    pub(crate) fn to_f64s(&self) -> Vec<f64> {
        typed!(self, xs => xs.iter().map(|&x| x as f64).collect())
    }

    /// The sum, adding left to right as a loop of `+` would. Integer elements whose total can
    /// never pass 2^53 are summed as integers (exact, so the same result, and vectorized).
    pub(crate) fn sum(&self) -> f64 {
        const EXACT: f64 = 9_007_199_254_740_992.0; // 2^53
        let bound = match self.kind {
            Kind::U8 => 255.0,
            Kind::I8 => 128.0,
            Kind::U16 => 65_535.0,
            Kind::I16 => 32_768.0,
            Kind::U32 => 4_294_967_295.0,
            Kind::I32 => 2_147_483_648.0,
            Kind::I64 | Kind::F64 => f64::INFINITY,
        };
        if (self.len as f64) * bound < EXACT {
            typed!(self, xs => xs.iter().map(|&x| x as i64).sum::<i64>() as f64)
        } else {
            typed!(self, xs => xs.iter().fold(0.0, |a, &x| a + x as f64))
        }
    }

    /// Index of the first element equal (`==`) to `x`.
    pub(crate) fn position(&self, x: f64) -> Option<usize> {
        // -0 == 0: look for 0. Otherwise an integer list holds no value its representation
        // cannot.
        let x = if x == 0.0 { 0.0 } else { x };
        if self.kind != Kind::F64 && !self.kind.holds(x) {
            return None;
        }
        typed!(self, xs => xs.iter().position(|&v| v as f64 == x))
    }

    /// The largest (`max`) or smallest element: for integers, the representation's own
    /// max/min (equal integers are indistinguishable); for doubles, the first one that
    /// `order` ranks above (or below) every earlier one.
    pub(crate) fn extreme(
        &self,
        max: bool,
        order: impl Fn(&f64, &f64) -> std::cmp::Ordering,
    ) -> Option<f64> {
        if self.kind == Kind::F64 {
            let want = if max {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            };
            let xs = self.to_f64s();
            let (&first, rest) = xs.split_first()?;
            return Some(rest.iter().fold(
                first,
                |b, &v| {
                    if order(&v, &b) == want {
                        v
                    } else {
                        b
                    }
                },
            ));
        }
        let n = self.len;
        let p = self.words.as_ptr() as *const u8;
        macro_rules! ext {
            ($t:ty) => {{
                // SAFETY: `words` holds `n` elements of this representation, aligned for it.
                let xs = unsafe { std::slice::from_raw_parts(p as *const $t, n) };
                if max {
                    xs.iter().max().map(|&v| v as f64)
                } else {
                    xs.iter().min().map(|&v| v as f64)
                }
            }};
        }
        match self.kind {
            Kind::U8 => ext!(u8),
            Kind::I8 => ext!(i8),
            Kind::U16 => ext!(u16),
            Kind::I16 => ext!(i16),
            Kind::U32 => ext!(u32),
            Kind::I32 => ext!(i32),
            Kind::I64 | Kind::F64 => ext!(i64),
        }
    }

    /// The elements without repeats (by exact value, as a `Seti` keeps them), first occurrences
    /// in order, in the same representation.
    pub(crate) fn unique(&self) -> NumList {
        let mut out = NumList::with_kind(self.kind, 0);
        // 8- and 16-bit elements: a bitmap of the values seen.
        if self.width <= 2 {
            // Values -32,768..=65,535 (every 8- and 16-bit representation) offset by 32,768.
            let mut seen = vec![0u64; 2048]; // 131,072 bits
            let mut mark = |key: usize| {
                let (w, b) = (key / 64, key % 64);
                let fresh = seen[w] & (1 << b) == 0;
                seen[w] |= 1 << b;
                fresh
            };
            typed!(self, xs => {
                for &x in xs {
                    let v = x as f64;
                    if mark((v as i64 + 32_768) as usize) {
                        out.push(v);
                    }
                }
            });
            return out;
        }
        let mut seen: std::collections::HashSet<u64, foldhash::fast::RandomState> =
            std::collections::HashSet::with_capacity_and_hasher(self.len, Default::default());
        typed!(self, xs => {
            for &x in xs {
                let v = x as f64;
                if seen.insert(v.to_bits()) {
                    out.push(v);
                }
            }
        });
        out
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

/// Sort `xs` ascending by `key` (an order-preserving map to unsigned integers of `bytes`
/// bytes): least-significant-digit radix sort, one byte per pass, skipping passes where every
/// element has the same byte. Small slices use the standard sort.
fn radix_sort<T: Copy + Ord>(xs: &mut [T], key: impl Fn(T) -> u64, bytes: u32) {
    if xs.len() < 256 {
        xs.sort_unstable();
        return;
    }
    let mut buf: Vec<T> = xs.to_vec();
    let (mut src, mut dst): (&mut [T], &mut [T]) = (xs, &mut buf);
    let mut in_buf = false;
    for pass in 0..bytes {
        let shift = 8 * pass;
        let mut count = [0usize; 256];
        for &x in src.iter() {
            count[((key(x) >> shift) & 0xFF) as usize] += 1;
        }
        if count.contains(&src.len()) {
            continue; // every element has this byte: nothing moves
        }
        let mut at = [0usize; 256];
        let mut total = 0;
        for (b, &c) in count.iter().enumerate() {
            at[b] = total;
            total += c;
        }
        for &x in src.iter() {
            let b = ((key(x) >> shift) & 0xFF) as usize;
            dst[at[b]] = x;
            at[b] += 1;
        }
        std::mem::swap(&mut src, &mut dst);
        in_buf = !in_buf;
    }
    if in_buf {
        // The sorted elements ended in the buffer: copy them back.
        dst.copy_from_slice(src);
    }
}

#[cfg(test)]
mod method_tests {
    use super::*;

    /// Lists of every representation: small and large, positive and negative, with repeats.
    fn samples() -> Vec<Vec<f64>> {
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let mut out = vec![vec![], vec![5.0], vec![3.0, 3.0, 1.0]];
        for (range, signed) in [
            (200u64, false),
            (100, true),
            (60_000, false),
            (30_000, true),
            (4_000_000_000, false),
            (2_000_000_000, true),
            (1 << 50, true),
        ] {
            for n in [10, 300, 5000] {
                out.push(
                    (0..n)
                        .map(|_| {
                            let v = (next() % range) as f64;
                            if signed && next() % 2 == 0 {
                                -v
                            } else {
                                v
                            }
                        })
                        .collect(),
                );
            }
        }
        out.push(vec![
            0.5,
            -0.0,
            0.0,
            f64::NAN,
            -1.5,
            f64::INFINITY,
            0.5,
            -0.0,
        ]);
        out.push((0..1000).map(|i| (i as f64) * 0.25 - 100.0).collect());
        out
    }

    fn total(a: &f64, b: &f64) -> std::cmp::Ordering {
        a.partial_cmp(b)
            .unwrap_or_else(|| a.is_nan().cmp(&b.is_nan()))
    }

    #[test]
    fn methods_match_naive_versions_in_every_representation() {
        for v in samples() {
            let l: NumList = v.iter().copied().collect();
            // Sorted: stable sort of the doubles, bit for bit.
            let mut want = v.clone();
            want.sort_by(total);
            let got: Vec<u64> = l.sorted(total).iter().map(f64::to_bits).collect();
            assert_eq!(
                got,
                want.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                "{:?}",
                l.kind
            );
            // Sum: left to right.
            let want = v.iter().fold(0.0, |a, &x| a + x);
            assert_eq!(l.sum().to_bits(), want.to_bits(), "{:?}", l.kind);
            // Unique by exact value, first occurrences in order.
            let mut seen = std::collections::HashSet::new();
            let want: Vec<u64> = v
                .iter()
                .map(|x| x.to_bits())
                .filter(|b| seen.insert(*b))
                .collect();
            let got: Vec<u64> = l.unique().iter().map(f64::to_bits).collect();
            assert_eq!(got, want, "{:?}", l.kind);
            // Extremes: the first element ranked above / below every earlier one.
            for max in [true, false] {
                let want = v.iter().copied().fold(None, |b: Option<f64>, x| match b {
                    Some(b)
                        if total(&x, &b)
                            != if max {
                                std::cmp::Ordering::Greater
                            } else {
                                std::cmp::Ordering::Less
                            } =>
                    {
                        Some(b)
                    }
                    _ => Some(x),
                });
                assert_eq!(
                    l.extreme(max, total).map(f64::to_bits),
                    want.map(f64::to_bits)
                );
            }
            // Search: `==`.
            for x in v.iter().take(5).copied().chain([-1.0, 1e300, 0.5, -0.0]) {
                assert_eq!(
                    l.position(x),
                    v.iter().position(|&y| y == x),
                    "{x} in {:?}",
                    l.kind
                );
            }
            // Slices and reversal keep the values.
            let n = v.len();
            assert_eq!(
                l.slice(n / 3, n / 2)
                    .iter()
                    .map(f64::to_bits)
                    .collect::<Vec<_>>(),
                v[n / 3..n / 2]
                    .iter()
                    .map(|x| x.to_bits())
                    .collect::<Vec<_>>()
            );
            let mut r = v.clone();
            r.reverse();
            assert_eq!(
                l.reversed().iter().map(f64::to_bits).collect::<Vec<_>>(),
                r.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            );
        }
    }
}
