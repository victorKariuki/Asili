//! `Orodha<Namba>` storage for the register VM and native code.
//!
//! Elements are 8-byte words in one of two representations for the whole list: `i64` when every
//! element is an integer that converts to `i64` and back without loss (no `-0.0`, NaN, ±∞ or
//! fraction, below 2^63 in magnitude), else `f64` bits. Reading always yields exactly the
//! stored `f64`, so the representation is invisible to Asili programs. Native code picks the
//! representation per list register from the range analysis (integer lists are read with plain
//! integer loads, no conversion) and asks for it through [`NumList::ensure`].

#[derive(Clone, Debug, Default)]
pub(crate) struct NumList {
    words: Vec<u64>,
    /// Elements are `f64` bits (otherwise `i64`s). An empty list starts as integers.
    floats: bool,
}

/// Whether `v` round-trips through `i64` exactly.
#[inline]
fn integral(v: f64) -> bool {
    v.abs() < 9_223_372_036_854_775_808.0 // 2^63
        && v.fract() == 0.0
        && !(v == 0.0 && v.is_sign_negative())
}

impl NumList {
    pub(crate) fn repeat(v: f64, count: usize) -> Self {
        let mut l = NumList::default();
        l.words.reserve(count);
        if !integral(v) {
            l.floats = true;
        }
        let w = l.encode(v);
        l.words.resize(count, w);
        l
    }

    #[inline]
    fn encode(&self, v: f64) -> u64 {
        if self.floats {
            v.to_bits()
        } else {
            v as i64 as u64
        }
    }

    #[inline]
    fn decode(&self, w: u64) -> f64 {
        if self.floats {
            f64::from_bits(w)
        } else {
            w as i64 as f64
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.words.len()
    }

    #[inline]
    pub(crate) fn get(&self, i: usize) -> Option<f64> {
        self.words.get(i).map(|&w| self.decode(w))
    }

    /// Store `v` at `i`; `false` if out of range.
    #[inline]
    pub(crate) fn set(&mut self, i: usize, v: f64) -> bool {
        if i >= self.words.len() {
            return false;
        }
        if !self.floats && !integral(v) {
            self.switch_to_floats();
        }
        self.words[i] = self.encode(v);
        true
    }

    pub(crate) fn push(&mut self, v: f64) {
        if !self.floats && !integral(v) {
            self.switch_to_floats();
        }
        let w = self.encode(v);
        self.words.push(w);
    }

    pub(crate) fn remove(&mut self, i: usize) -> f64 {
        let w = self.words.remove(i);
        self.decode(w)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.words.iter().map(|&w| self.decode(w))
    }

    fn switch_to_floats(&mut self) {
        for w in &mut self.words {
            *w = (*w as i64 as f64).to_bits();
        }
        self.floats = true;
    }

    /// Data pointer in the current representation.
    pub(crate) fn as_mut_ptr(&mut self) -> *mut u64 {
        self.words.as_mut_ptr()
    }

    /// Switch to integer words (`ints`) or float words and return the data pointer, or null
    /// when integers were asked for but some element is not integral.
    pub(crate) fn ensure(&mut self, ints: bool) -> *mut u64 {
        if ints && self.floats {
            if !self.words.iter().all(|&w| integral(f64::from_bits(w))) {
                return std::ptr::null_mut();
            }
            for w in &mut self.words {
                *w = f64::from_bits(*w) as i64 as u64;
            }
            self.floats = false;
        } else if !ints && !self.floats {
            self.switch_to_floats();
        }
        self.words.as_mut_ptr()
    }
}

impl FromIterator<f64> for NumList {
    fn from_iter<I: IntoIterator<Item = f64>>(items: I) -> Self {
        let values: Vec<f64> = items.into_iter().collect();
        let floats = !values.iter().all(|&v| integral(v));
        let mut l = NumList {
            words: Vec::with_capacity(values.len()),
            floats,
        };
        for v in values {
            let w = l.encode(v);
            l.words.push(w);
        }
        l
    }
}

#[cfg(test)]
mod tests {
    use super::NumList;

    fn bits(l: &NumList) -> Vec<u64> {
        l.iter().map(f64::to_bits).collect()
    }

    #[test]
    fn values_round_trip_through_both_representations() {
        let tricky = [
            0.0,
            -0.0,
            1.0,
            -7.0,
            0.5,
            f64::NAN,
            f64::INFINITY,
            9.007_199_254_740_993e15,
            1e19,
            i64::MIN as f64,
        ];
        for &v in &tricky {
            let mut ints: NumList = [3.0, 4.0].into_iter().collect();
            ints.push(v);
            assert_eq!(ints.get(2).map(f64::to_bits), Some(v.to_bits()), "{v}");
            let mut l: NumList = [v, 2.0].into_iter().collect();
            let before = bits(&l);
            l.ensure(true);
            l.ensure(false);
            assert_eq!(bits(&l), before, "{v}");
            let r = NumList::repeat(v, 3);
            assert!(r.iter().all(|x| x.to_bits() == v.to_bits()));
        }
    }

    #[test]
    fn integer_request_fails_on_fractions() {
        let mut l: NumList = [1.0, 2.5].into_iter().collect();
        assert!(l.ensure(true).is_null());
        let mut l: NumList = [1.0, 2.0].into_iter().collect();
        assert!(!l.ensure(true).is_null());
        assert!(l.set(0, -0.0));
        assert_eq!(l.get(0).map(f64::to_bits), Some((-0.0f64).to_bits()));
    }
}
