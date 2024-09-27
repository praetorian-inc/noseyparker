use bstr::{BStr, BString};
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol<T> {
    i: T,
    j: T,
}

#[allow(clippy::len_without_is_empty)]
pub trait SymbolType: Copy + PartialEq + Eq + std::hash::Hash {
    fn to_range(self) -> std::ops::Range<usize>;
    fn from_range(r: std::ops::Range<usize>) -> Self;

    fn len(&self) -> usize {
        self.to_range().len()
    }
}

impl SymbolType for Symbol<usize> {
    #[inline]
    fn to_range(self) -> std::ops::Range<usize> {
        self.i..self.j
    }

    #[inline]
    fn from_range(r: std::ops::Range<usize>) -> Self {
        Self {
            i: r.start,
            j: r.end,
        }
    }
}

impl SymbolType for Symbol<u32> {
    #[inline]
    fn to_range(self) -> std::ops::Range<usize> {
        self.i as usize..self.j as usize
    }

    #[inline]
    fn from_range(r: std::ops::Range<usize>) -> Self {
        let i = r.start.try_into().expect("range should fit in u32");
        let j = r.end.try_into().expect("range should fit in u32");
        Self { i, j }
    }
}

#[derive(Default)]
pub struct BStringTable<S = Symbol<u32>> {
    storage: Vec<u8>,
    mapping: HashMap<BString, S>,
}

impl<S: SymbolType> BStringTable<S> {
    pub fn new() -> Self {
        Self {
            storage: Default::default(),
            mapping: Default::default(),
        }
    }

    pub fn with_capacity(num_symbols: usize, total_bytes: usize) -> Self {
        Self {
            storage: Vec::with_capacity(total_bytes),
            mapping: HashMap::with_capacity(num_symbols),
        }
    }

    #[inline]
    pub fn get_or_intern(&mut self, s: &BStr) -> S {
        match self.mapping.get(s) {
            Some(s) => *s,
            None => {
                let s = s.to_owned();
                let i = self.storage.len();
                let j = i + s.len();
                self.storage.extend(s.as_slice());
                let k = S::from_range(i..j);
                self.mapping.insert(s, k);
                k
            }
        }
    }

    #[inline]
    pub fn resolve(&self, s: S) -> &BStr {
        BStr::new(&self.storage[s.to_range()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn simple_roundtrip() {
        let mut t: BStringTable = BStringTable::new();

        let s1 = BStr::new("Hello");
        let s2 = BStr::new("World");

        let sym1 = t.get_or_intern(s1);
        let sym1a = t.get_or_intern(s1);
        assert_eq!(sym1, sym1a);

        let sym2 = t.get_or_intern(s2);
        let sym2a = t.get_or_intern(s2);
        assert_eq!(sym2, sym2a);

        assert_ne!(sym1, sym2);

        assert_eq!(s1, t.resolve(sym1));
        assert_eq!(s2, t.resolve(sym2));

        let sym1b = t.get_or_intern(s1);
        assert_eq!(sym1, sym1b);
    }
}
